import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Filter } from "../types";
import type {
  Benchmark,
  LeakReport,
  SpotDefinition,
  StudyItem,
} from "../study-types";
import { study, useStudy, useStudyMutation } from "../study-api";
import { decimal, number } from "../format";
import { Field, ErrorBanner, Skeleton } from "./UI";
import { SpotEditor } from "./StudySpots";
import { StudySelect, actions } from "./StudyWorkspace";

export function LeakFinder({
  filter,
  spot,
  pack,
  revision,
  refresh,
  explore,
  strategy,
}: {
  filter: Filter;
  spot: SpotDefinition;
  pack: string;
  revision: number;
  refresh: () => void;
  explore: (spot: SpotDefinition) => void;
  strategy: (node: string) => void;
}) {
  const { t } = useTranslation();
  const result = useStudy<LeakReport>(
    { op: "leaks", filter, ...(pack ? { pack } : {}) },
    revision,
  );
  const saved = useStudy<StudyItem<Benchmark>[]>(
    { op: "items", kind: "benchmark" },
    revision,
  );
  const [editing, setEditing] = useState(false),
    [id, setId] = useState<string | undefined>();
  const [target, setTarget] = useState<Benchmark>({
    name: "",
    spot,
    action: "fold",
    low: 0,
    high: 100,
    min_samples: 100,
    note: "",
  });
  const [conditions, setConditions] = useState(false);
  const operation = useStudyMutation();
  const patch = (p: Partial<Benchmark>) => setTarget((s) => ({ ...s, ...p }));
  return (
    <div className="study-leaks">
      <section className="panel study-intro">
        <div>
          <span className="section-kicker">LEAK FINDER</span>
          <h2>{t("study.leakTitle")}</h2>
          <p>{t("study.leakHelp")}</p>
        </div>
        <button
          className="button primary"
          onClick={() => {
            setId(undefined);
            setTarget({
              name: "",
              spot,
              action: "fold",
              low: 0,
              high: 100,
              min_samples: 100,
              note: "",
            });
            setEditing(true);
          }}
        >
          {t("study.newTarget")}
        </button>
      </section>
      {(result.error || saved.error || operation.error) && (
        <ErrorBanner>
          {result.error || saved.error || operation.error}
        </ErrorBanner>
      )}
      {editing && (
        <section className="panel study-builder">
          <h3>{t(id ? "study.editTarget" : "study.newTarget")}</h3>
          <div className="study-fields">
            <Field label={t("study.targetName")}>
              <input
                maxLength={150}
                value={target.name}
                onChange={(e) => patch({ name: e.target.value })}
              />
            </Field>
            <StudySelect
              label="study.action"
              value={target.action}
              all={false}
              options={actions}
              change={(action) => patch({ action: action! })}
            />
            <Field label={t("study.targetLow")}>
              <input
                type="number"
                min="0"
                max="100"
                step="0.1"
                value={target.low}
                onChange={(e) => patch({ low: Number(e.target.value) })}
              />
            </Field>
            <Field label={t("study.targetHigh")}>
              <input
                type="number"
                min="0"
                max="100"
                step="0.1"
                value={target.high}
                onChange={(e) => patch({ high: Number(e.target.value) })}
              />
            </Field>
            <Field label={t("study.minSamples")}>
              <input
                type="number"
                min="1"
                max="1000000"
                value={target.min_samples}
                onChange={(e) => patch({ min_samples: Number(e.target.value) })}
              />
            </Field>
          </div>
          <Field label={t("study.targetNote")}>
            <textarea
              rows={2}
              maxLength={4000}
              value={target.note}
              onChange={(e) => patch({ note: e.target.value })}
            />
          </Field>
          <button
            className="text-button"
            aria-expanded={conditions}
            onClick={() => setConditions(!conditions)}
          >
            {t("study.editConditions")}
          </button>
          {conditions && (
            <SpotEditor
              spot={target.spot}
              setSpot={(spot) => patch({ spot })}
            />
          )}
          <div className="study-row-actions">
            <button
              className="button primary"
              disabled={operation.busy || !target.name.trim()}
              onClick={() =>
                void operation.run(async () => {
                  await study({
                    op: "save_item",
                    kind: "benchmark",
                    id,
                    data: target,
                  });
                  setEditing(false);
                  refresh();
                })
              }
            >
              {t("study.saveTarget")}
            </button>
            <button
              className="button subtle"
              disabled={operation.busy}
              onClick={() => setEditing(false)}
            >
              {t("study.cancel")}
            </button>
          </div>
        </section>
      )}
      {!result.data && !result.error ? (
        <Skeleton rows={5} />
      ) : (
        <div className="study-leak-grid">
          {result.data?.rows.map((row) => (
            <article
              key={row.id}
              className={`panel study-leak ${row.enough && row.priority > 0 ? "needs-review" : ""}`}
            >
              <div className="study-row-actions">
                <span className="subtle-badge">
                  {t(
                    row.source === "gto"
                      ? "study.gtoTarget"
                      : "study.customTarget",
                  )}
                </span>
                <span className="muted">
                  {t(row.enough ? "study.reviewPriority" : "study.lowSample")}
                </span>
              </div>
              <h3>{row.name}</h3>
              <p className="mono">
                {row.source === "gto" ? row.action : t(`study.${row.action}`)}
              </p>
              <div className="study-leak-values">
                <div>
                  <span>{t("study.actual")}</span>
                  <strong>
                    {row.actual === null ? "—" : `${decimal(row.actual, 1)}%`}
                  </strong>
                </div>
                <div>
                  <span>{t("study.target")}</span>
                  <strong>
                    {decimal(row.low, 1)}–{decimal(row.high, 1)}%
                  </strong>
                </div>
                <div>
                  <span>{t("study.difference")}</span>
                  <strong>
                    {row.gap === null
                      ? "—"
                      : `${row.gap > 0 ? "+" : ""}${decimal(row.gap, 1)} pp`}
                  </strong>
                </div>
              </div>
              <p className="muted">
                {t("study.sampleCount", {
                  hits: number(row.hits),
                  n: number(row.opportunities),
                })}{" "}
                ·{" "}
                {row.interval
                  ? t("study.interval", {
                      low: decimal(row.interval[0], 2),
                      high: decimal(row.interval[1], 2),
                    })
                  : "—"}
              </p>
              {row.note && <p className="study-user-note">{row.note}</p>}
              <div className="study-row-actions">
                <button
                  className="text-button"
                  onClick={() =>
                    row.source === "gto"
                      ? strategy(row.node!)
                      : explore(row.spot!)
                  }
                >
                  {t(
                    row.source === "gto"
                      ? "study.viewStrategy"
                      : "study.viewDecisions",
                  )}
                </button>
                {row.source === "custom" && (
                  <>
                    <button
                      className="text-button"
                      onClick={() => {
                        const target = saved.data?.find((b) => b.id === row.id);
                        if (target) {
                          setTarget(target.data);
                          setId(target.id);
                          setEditing(true);
                        }
                      }}
                    >
                      {t("study.edit")}
                    </button>
                    <button
                      className="text-button"
                      disabled={operation.busy}
                      onClick={() =>
                        void operation.run(async () => {
                          await study({
                            op: "delete_item",
                            kind: "benchmark",
                            id: row.id,
                          });
                          refresh();
                        })
                      }
                    >
                      {t("study.remove")}
                    </button>
                  </>
                )}
              </div>
            </article>
          ))}
        </div>
      )}
      {result.data?.rows.length === 0 && (
        <p className="study-empty">{t("study.noLeaks")}</p>
      )}
      <p className="study-footnote">{t("study.leakCaveat")}</p>
    </div>
  );
}
