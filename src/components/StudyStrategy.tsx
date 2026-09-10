import { StrategyDecisions } from "./StudyStrategyDecisions";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open as choose } from "@tauri-apps/plugin-dialog";
import { desktop } from "../api";
import { study, useStudy, useStudyMutation } from "../study-api";
import type { Filter } from "../types";
import type {
  DecisionComparison,
  DecisionRef,
  NodeSummary,
  PackSummary,
  StrategyMatrix,
} from "../study-types";
import { decimal, number } from "../format";
import { ErrorBanner, Field, Skeleton } from "./UI";
import { StudySelect } from "./StudyWorkspace";

export function StrategyCompare({
  filter,
  pack,
  setPack,
  node,
  setNode,
  reference,
  clearReference,
  revision,
  refresh,
  notify,
  open,
  trainer,
}: {
  filter: Filter;
  pack: string;
  setPack: (p: string) => void;
  node: string;
  setNode: (n: string) => void;
  reference: DecisionRef | null;
  clearReference: () => void;
  revision: number;
  refresh: () => void;
  notify: (s: string) => void;
  open: (id: number, step: number) => void;
  trainer: () => void;
}) {
  const { t } = useTranslation();
  const [path, setPath] = useState("");
  const [hand, setHand] = useState("AA");
  const [mode, setMode] = useState("strategy"),
    [action, setAction] = useState("");
  const nodes = useStudy<NodeSummary[]>(
    pack ? { op: "nodes", pack } : null,
    revision,
  );
  const comparison = useStudy<DecisionComparison>(
    pack && reference
      ? { op: "compare", pack, reference, ...(node ? { node } : {}) }
      : null,
    revision,
  );
  useEffect(() => {
    if (!nodes.data?.length) return;
    if (!node) {
      const n = reference
        ? nodes.data.find((n) => n.id === comparison.data?.node?.id)
        : nodes.data[0];
      if (n) setNode(n.id);
    }
  }, [nodes.data, node, reference, comparison.data, setNode]);
  useEffect(() => {
    if (reference && comparison.data)
      setHand(comparison.data.decision.hand_class);
  }, [reference, comparison.data]);
  const matrix = useStudy<StrategyMatrix>(
    pack && node ? { op: "matrix", pack, node, filter } : null,
    revision,
  );
  useEffect(() => {
    if (matrix.data && !matrix.data.node.actions.some((a) => a.code === action))
      setAction(
        matrix.data.node.actions.find(
          (a) => a.kind === "raise" && a.code !== "RAI",
        )?.code ||
          matrix.data.node.actions[0]?.code ||
          "",
      );
  }, [matrix.data, action]);
  const operation = useStudyMutation();
  const cell = matrix.data?.cells.find((c) => c.hand === hand);
  const importPack = () =>
    operation.run(async () => {
      let input = path;
      if (desktop && !input.trim()) {
        const p = await choose({
          multiple: false,
          filters: [{ name: "Nexus manifest", extensions: ["json"] }],
        });
        if (!p || Array.isArray(p)) return;
        input = p;
      }
      if (!input.trim()) return;
      const result = await study<PackSummary>({
        op: "import_pack",
        path: input,
      });
      setPack(result.id);
      setNode("");
      clearReference();
      refresh();
      notify(
        t("study.packImported", {
          files: result.files,
          nodes: result.nodes,
          issues: result.reference_cells,
        }),
      );
    });
  return (
    <div className="study-strategy">
      <section className="panel study-intro">
        <div>
          <span className="section-kicker">PREFLOP GTO</span>
          <h2>{t("study.strategyTitle")}</h2>
          <p>{t("study.strategyHelp")}</p>
        </div>
        <div>
          <Field label={t("study.manifestPath")}>
            <input value={path} onChange={(e) => setPath(e.target.value)} />
          </Field>
          <button
            className="button primary"
            disabled={operation.busy}
            onClick={() => void importPack()}
          >
            {t(operation.busy ? "study.working" : "study.importPack")}
          </button>
        </div>
      </section>
      {(operation.error || nodes.error || matrix.error || comparison.error) && (
        <ErrorBanner>
          {operation.error || nodes.error || matrix.error || comparison.error}
        </ErrorBanner>
      )}
      {!pack && <p className="study-empty">{t("study.noPack")}</p>}
      {pack && (
        <>
          <div className="study-strategy-controls">
            <StudySelect
              label="study.node"
              value={node}
              all={false}
              options={(nodes.data || []).map((n) => [
                n.id,
                `${n.hero} · ${n.path || t("study.firstIn")} (${n.ready_cells}/${n.ready_cells + n.reference_cells})`,
              ])}
              change={(n) => setNode(n || "")}
            />
            <StudySelect
              label="study.action"
              value={action}
              all={false}
              options={(matrix.data?.node.actions || []).map((a) => [
                a.code,
                a.label,
              ])}
              change={(a) => setAction(a || "")}
            />
          </div>
          {reference && (
            <section className="panel study-comparison">
              <div className="study-row-actions">
                <strong>
                  {reference.hand_id} · {reference.seq}
                </strong>
                <button
                  className="text-button"
                  onClick={() => {
                    clearReference();
                    setNode("");
                  }}
                >
                  {t("study.clearReference")}
                </button>
              </div>
              {!comparison.data ? (
                <p>{t("study.loading")}</p>
              ) : (
                <>
                  <span
                    className={`subtle-badge ${comparison.data.status === "matched" ? "" : "reference"}`}
                  >
                    {t(`study.match.${comparison.data.status}`)}
                  </span>
                  <p>
                    {comparison.data.reasons
                      .map((r) => t(`study.reason.${r}`))
                      .join(" · ")}
                  </p>
                  <p>
                    {t("study.observed")}:{" "}
                    {t(`study.${comparison.data.decision.action}`)}{" "}
                    {comparison.data.decision.size_bb !== null &&
                      `${comparison.data.decision.size_bb}bb`}
                  </p>
                  <button
                    className="text-button"
                    disabled={operation.busy}
                    onClick={() =>
                      void operation.run(async () => {
                        const r = await study<{ hand: number; seq: number }>({
                          op: "resolve",
                          reference,
                        });
                        open(r.hand, r.seq);
                      })
                    }
                  >
                    {t("study.replay")}
                  </button>
                </>
              )}
            </section>
          )}
          {!matrix.data && !matrix.error && node ? (
            <Skeleton rows={7} />
          ) : (
            matrix.data && (
              <div className="range-layout">
                <section className="panel range-panel">
                  <div className="panel-heading">
                    <h3>
                      {matrix.data.node.hero} ·{" "}
                      {matrix.data.node.path || t("study.firstIn")}
                    </h3>
                  </div>
                  <div className="segmented">
                    {["strategy", "actual", "difference"].map((m) => (
                      <button
                        key={m}
                        className={mode === m ? "active" : ""}
                        aria-pressed={mode === m}
                        onClick={() => setMode(m)}
                      >
                        {t(`study.${m}`)}
                      </button>
                    ))}
                  </div>
                  <div className="range-matrix study-matrix">
                    {matrix.data.cells.map((c) => {
                      const expected = c.strategy?.[action];
                      const n = c.observed?.matched || 0;
                      const actual = n
                        ? (c.observed?.matched_actions[action] || 0) / n
                        : undefined;
                      const value =
                        mode === "strategy"
                          ? expected
                          : mode === "actual"
                            ? actual
                            : actual !== undefined && expected !== undefined
                              ? actual - expected
                              : undefined;
                      const intensity =
                        value === undefined ? 0 : Math.min(Math.abs(value), 1);
                      return (
                        <button
                          key={c.hand}
                          className={`range-cell ${hand === c.hand ? "focused" : ""} ${c.issue ? "study-uncertain" : ""}`}
                          aria-label={`${c.hand} · ${value === undefined ? "—" : `${decimal(value * 100, 1)}${mode === "difference" ? " pp" : "%"}`}`}
                          style={{
                            background:
                              value === undefined
                                ? undefined
                                : value < 0
                                  ? `rgba(190,118,105,${0.1 + intensity * 0.5})`
                                  : `rgba(114,173,141,${0.08 + intensity * 0.5})`,
                          }}
                          onClick={() => setHand(c.hand)}
                        >
                          <strong>{c.hand}</strong>
                          <small>
                            {c.issue
                              ? "?"
                              : value === undefined
                                ? "—"
                                : `${decimal(value * 100, 0)}${mode === "difference" ? "pp" : "%"}`}
                          </small>
                        </button>
                      );
                    })}
                  </div>
                  <p className="study-footnote">{t("study.matrixHelp")}</p>
                </section>
                <aside className="range-detail">
                  <span className="section-kicker">
                    {t("study.handDetail")}
                  </span>
                  <h2>{hand}</h2>
                  {cell?.issue && (
                    <p className="study-warning">
                      {t(`study.reason.${cell.issue}`)}
                    </p>
                  )}
                  <div className="detail-metric">
                    <span>{t("study.matchedSamples")}</span>
                    <strong>
                      {number(cell?.observed?.matched || 0)} /{" "}
                      {number(cell?.observed?.opportunities || 0)}
                    </strong>
                  </div>
                  {matrix.data.node.actions.map((a) => (
                    <div className="detail-metric" key={a.code}>
                      <span>{a.label}</span>
                      <strong>
                        {cell?.strategy
                          ? `${decimal((cell.strategy[a.code] || 0) * 100, 2)}%`
                          : "—"}
                      </strong>
                    </div>
                  ))}
                  <button
                    className="button primary"
                    disabled={!cell?.strategy || operation.busy}
                    onClick={() =>
                      void operation.run(async () => {
                        const result = await study<{
                          added: number;
                          duplicates: number;
                        }>({
                          op: "enqueue",
                          references: [],
                          pack,
                          node,
                          hand_class: hand,
                        });
                        refresh();
                        notify(t("study.added", result));
                      })
                    }
                  >
                    {t("study.trainThisHand")}
                  </button>
                  <button className="text-button" onClick={trainer}>
                    {t("study.openTrainer")}
                  </button>
                  <a
                    className="text-button"
                    href={matrix.data.node.source_url}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {t("study.source")}
                  </a>
                </aside>
              </div>
            )
          )}
          {matrix.data && Object.keys(matrix.data.reasons).length > 0 && (
            <section className="panel study-reasons">
              <h3>{t("study.unmatchedReasons")}</h3>
              {Object.entries(matrix.data.reasons).map(([reason, count]) => (
                <span className="subtle-badge" key={reason}>
                  {t(`study.reason.${reason}`)} · {number(count)}
                </span>
              ))}
            </section>
          )}
        </>
      )}
      {pack && node && <StrategyDecisions pack={pack} node={node} hand={hand} filter={filter} revision={revision} open={open} notify={notify} refresh={refresh} />}
      <p className="study-footnote">{t("study.modelCaveat")}</p>
    </div>
  );
}
