import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Filter } from "../types";
import type {
  DecisionRef,
  SavedSpot,
  SpotDefinition,
  SpotReport,
  SpotRow,
  StudyItem,
} from "../study-types";
import { study, useStudy, useStudyMutation } from "../study-api";
import { time, number, decimal } from "../format";
import { ErrorBanner, Field, Skeleton } from "./UI";
import { Card } from "./Replayer";
import {
  StudySelect,
  positions,
  streets,
  actions,
  emptySpot,
} from "./StudyWorkspace";

const presets: [string, SpotDefinition][] = [
  [
    "study.blindDefence",
    { street: "preflop", position: "BB", facing: "open", line: [] },
  ],
  ["study.facing3bet", { street: "preflop", facing: "three_bet", line: [] }],
  [
    "study.flopFacingRaise",
    {
      street: "flop",
      facing: "raise",
      line: [
        { street: "flop", actor: "hero", action: "bet" },
        { street: "flop", actor: "villain", action: "raise" },
      ],
    },
  ],
  [
    "study.secondBarrel",
    {
      street: "turn",
      facing: "bet",
      line: [
        { street: "flop", actor: "hero", action: "check" },
        { street: "flop", actor: "villain", action: "bet" },
        { street: "flop", actor: "hero", action: "call" },
        { street: "turn", actor: "hero", action: "check" },
        { street: "turn", actor: "villain", action: "bet" },
      ],
    },
  ],
  ["study.riverDecision", { street: "river", facing: "bet", line: [] }],
];

export function SpotEditor({
  spot,
  setSpot,
}: {
  spot: SpotDefinition;
  setSpot: (s: SpotDefinition) => void;
}) {
  const { t } = useTranslation();
  const patch = (p: Partial<SpotDefinition>) => setSpot({ ...spot, ...p });
  return (
    <>
      <div className="study-fields">
        <StudySelect
          label="study.street"
          value={spot.street}
          options={streets}
          change={(street) => patch({ street })}
        />
        <StudySelect
          label="study.heroPosition"
          value={spot.position}
          options={positions}
          change={(position) => patch({ position })}
        />
        <StudySelect
          label="study.opponent"
          value={spot.opponent}
          options={positions}
          change={(opponent) => patch({ opponent })}
        />
        <StudySelect
          label="study.role"
          value={spot.role}
          options={[
            ["IP", "IP"],
            ["OOP", "OOP"],
          ]}
          change={(role) => patch({ role })}
        />
        <StudySelect
          label="study.facing"
          value={spot.facing}
          options={[
            "unopened",
            "limp",
            "open",
            "three_bet",
            "four_bet_plus",
            "checked_to",
            "bet",
            "raise",
          ].map((f) => [f, `study.facing.${f}`])}
          change={(facing) => patch({ facing })}
        />
        <StudySelect
          label="study.potType"
          value={spot.pot_type}
          options={["limped", "SRP", "3-bet", "4-bet+", "special"].map((p) => [
            p,
            p,
          ])}
          change={(pot_type) => patch({ pot_type })}
        />
        <Field label={t("study.effectiveMin")}>
          <input
            type="number"
            min="0"
            value={spot.effective_min ?? ""}
            onChange={(e) =>
              patch({
                effective_min:
                  e.target.value === "" ? undefined : Number(e.target.value),
              })
            }
          />
        </Field>
        <Field label={t("study.effectiveMax")}>
          <input
            type="number"
            min="0"
            value={spot.effective_max ?? ""}
            onChange={(e) =>
              patch({
                effective_max:
                  e.target.value === "" ? undefined : Number(e.target.value),
              })
            }
          />
        </Field>
        <StudySelect
          label="study.texture"
          value={spot.texture}
          options={["rainbow", "two-tone", "monotone"].map((p) => [p, p])}
          change={(texture) => patch({ texture })}
        />
        <StudySelect
          label="study.paired"
          value={spot.paired === undefined ? undefined : String(spot.paired)}
          options={[
            ["true", "study.yes"],
            ["false", "study.no"],
          ]}
          change={(v) =>
            patch({ paired: v === undefined ? undefined : v === "true" })
          }
        />
        <StudySelect
          label="study.highCard"
          value={spot.high_card}
          options={"AKQJT98765432".split("").map((p) => [p, p])}
          change={(high_card) => patch({ high_card })}
        />
        <Field label={t("study.facingMin")}>
          <input
            type="number"
            min="0"
            value={spot.facing_min ?? ""}
            onChange={(e) =>
              patch({
                facing_min:
                  e.target.value === "" ? undefined : Number(e.target.value),
              })
            }
          />
        </Field>
        <Field label={t("study.facingMax")}>
          <input
            type="number"
            min="0"
            value={spot.facing_max ?? ""}
            onChange={(e) =>
              patch({
                facing_max:
                  e.target.value === "" ? undefined : Number(e.target.value),
              })
            }
          />
        </Field>
      </div>
      <div className="study-path">
        <h3>{t("study.path")}</h3>
        <p className="muted">{t("study.pathHelp")}</p>
        {spot.line.map((step, i) => (
          <div key={i} className="study-path-step">
            <span className="mono">{i + 1}</span>
            <StudySelect
              label="study.street"
              value={step.street}
              options={streets.slice(1)}
              all={false}
              change={(v) =>
                patch({
                  line: spot.line.map((s, j) =>
                    j === i ? { ...s, street: v! } : s,
                  ),
                })
              }
            />
            <StudySelect
              label="study.actor"
              value={step.actor}
              options={[
                ["hero", "Hero"],
                ["villain", "study.villain"],
              ]}
              all={false}
              change={(v) =>
                patch({
                  line: spot.line.map((s, j) =>
                    j === i ? { ...s, actor: v! } : s,
                  ),
                })
              }
            />
            <StudySelect
              label="study.action"
              value={step.action}
              options={actions}
              all={false}
              change={(v) =>
                patch({
                  line: spot.line.map((s, j) =>
                    j === i ? { ...s, action: v! } : s,
                  ),
                })
              }
            />
            <button
              className="text-button"
              aria-label={t("study.removeStep", { n: i + 1 })}
              onClick={() =>
                patch({ line: spot.line.filter((_, j) => j !== i) })
              }
            >
              {t("study.remove")}
            </button>
          </div>
        ))}
        <button
          className="button subtle"
          disabled={spot.line.length >= 24}
          onClick={() =>
            patch({
              line: [
                ...spot.line,
                {
                  street: spot.line.at(-1)?.street || "flop",
                  actor: "hero",
                  action: "check",
                },
              ],
            })
          }
        >
          {t("study.addAction")}
        </button>
      </div>
    </>
  );
}

export function SpotExplorer({
  filter,
  spot,
  setSpot,
  pack,
  revision,
  refresh,
  open,
  compare,
  notify,
  onLeaks,
}: {
  filter: Filter;
  spot: SpotDefinition;
  setSpot: (s: SpotDefinition) => void;
  pack: string;
  revision: number;
  refresh: () => void;
  open: (id: number, step: number) => void;
  compare: (r: DecisionRef) => void;
  notify: (message: string) => void;
  onLeaks: () => void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const result = useStudy<SpotReport>(
    { op: "explore", query: { filter, spot } },
    revision,
  );
  const saved = useStudy<StudyItem<SavedSpot>[]>(
    { op: "items", kind: "spot" },
    revision,
  );
  const mutation = useStudyMutation();
  const [extra, setExtra] = useState<SpotRow[]>([]),
    [cursor, setCursor] = useState<number | null>(null);
  const key = JSON.stringify({ filter, spot, revision });
  const currentKey = useRef(key);
  currentKey.current = key;
  const [selected, setSelected] = useState<Set<number>>(new Set());
  useEffect(() => {
    setExtra([]);
    setCursor(null);
    setSelected(new Set());
  }, [key]);
  const rows = [...(result.data?.rows || []), ...extra];
  const next = extra.length ? cursor : result.data?.next_cursor;
  const toggle = (id: number) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const add = (refs: DecisionRef[]) =>
    mutation.run(async () => {
      const result = await study<{ added: number; duplicates: number }>({
        op: "enqueue",
        references: refs,
        ...(pack ? { pack } : {}),
      });
      notify(t("study.added", result));
      refresh();
    });
  return (
    <div className="study-spots">
      <section className="panel study-builder">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">SPOT EXPLORER</span>
            <h2>{t("study.spotTitle")}</h2>
          </div>
          <button className="text-button" onClick={() => setSpot(emptySpot())}>
            {t("study.reset")}
          </button>
        </div>
        <div className="study-presets">
          {presets.map(([name, p]) => (
            <button
              key={name}
              className="button subtle"
              onClick={() =>
                setSpot({ ...p, line: p.line.map((s) => ({ ...s })) })
              }
            >
              {t(name)}
            </button>
          ))}
        </div>
        <div className="study-save">
          <Field label={t("study.spotName")}>
            <input
              maxLength={150}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </Field>
          <button
            className="button subtle"
            disabled={mutation.busy || !name.trim()}
            onClick={() =>
              void mutation.run(async () => {
                await study({
                  op: "save_item",
                  kind: "spot",
                  data: { name: name.trim(), spot },
                });
                refresh();
                notify(t("study.saved"));
              })
            }
          >
            {t("study.saveSpot")}
          </button>
          <StudySelect
            label="study.savedSpots"
            options={(saved.data || []).map((s) => [s.id, s.data.name])}
            change={(id) => {
              const s = saved.data?.find((s) => s.id === id);
              if (s) {
                setSpot(s.data.spot);
                setName(s.data.name);
              }
            }}
          />
          <button
            className="text-button"
            aria-expanded={advanced}
            onClick={() => setAdvanced(!advanced)}
          >
            {t("study.editConditions")}
          </button>
        </div>
        {advanced && <SpotEditor spot={spot} setSpot={setSpot} />}
        <p className="study-selection">
          {[
            spot.position,
            spot.opponent ? `vs ${spot.opponent}` : null,
            spot.street ? t(`study.${spot.street}`) : null,
            spot.facing ? t(`study.facing.${spot.facing}`) : null,
            spot.pot_type,
          ]
            .filter(Boolean)
            .join(" · ") || t("study.allDecisions")}
        </p>
      </section>
      {(result.error || mutation.error || saved.error) && (
        <ErrorBanner>
          {result.error || mutation.error || saved.error}
        </ErrorBanner>
      )}
      {!result.data && !result.error ? (
        <Skeleton rows={6} />
      ) : (
        result.data && (
          <>
            <div className="study-metrics">
              <div>
                <span>{t("study.opportunities")}</span>
                <strong>{number(result.data.opportunities)}</strong>
              </div>
              <div>
                <span>{t("study.uniqueHands")}</span>
                <strong>{number(result.data.hands)}</strong>
              </div>
              <div>
                <span>{t("study.observedNet")}</span>
                <strong>{decimal(result.data.net_bb, 1)} bb</strong>
              </div>
              {actions.map(([key, label]) => (
                <div key={key}>
                  <span>{t(label)}</span>
                  <strong>
                    {result.data!.opportunities
                      ? `${decimal(((result.data!.actions[key] || 0) / result.data!.opportunities) * 100, 1)}%`
                      : "—"}
                  </strong>
                  <small>{number(result.data!.actions[key] || 0)}</small>
                </div>
              ))}
            </div>
            <p className="muted">{t("study.denominatorHelp")}</p>
            <div className="study-row-actions">
              <button
                className="button primary"
                disabled={!selected.size || mutation.busy}
                onClick={() =>
                  void add(
                    rows
                      .filter((r) => selected.has(r.id))
                      .map((r) => r.decision.reference),
                  )
                }
              >
                {t("study.trainSelected", { n: selected.size })}
              </button>
              <button className="button subtle" onClick={onLeaks}>
                {t("study.setTarget")}
              </button>
            </div>
            <div className="panel table-scroll">
              <table className="data-table study-table">
                <thead>
                  <tr>
                    <th>{t("study.select")}</th>
                    <th>{t("study.decision")}</th>
                    <th>{t("study.context")}</th>
                    <th>{t("study.observed")}</th>
                    <th>{t("study.tools")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.id}>
                      <td>
                        <input
                          type="checkbox"
                          checked={selected.has(row.id)}
                          aria-label={t("study.selectDecision", {
                            id: row.decision.reference.hand_id,
                            seq: row.decision.reference.seq,
                          })}
                          onChange={() => toggle(row.id)}
                        />
                      </td>
                      <td>
                        <strong>{row.decision.hand_class}</strong>
                        <small className="block muted">
                          {row.decision.reference.hand_id} ·{" "}
                          {time(row.played_at)}
                        </small>
                      </td>
                      <td>
                        {row.decision.position}{" "}
                        {row.decision.opponent && `vs ${row.decision.opponent}`}{" "}
                        · {row.decision.role}
                        <small className="block">
                          {t(`study.${row.decision.street}`)} ·{" "}
                          {t(`study.facing.${row.decision.facing}`)} ·{" "}
                          {row.decision.effective_bb !== null
                            ? `${decimal(row.decision.effective_bb, 1)}bb`
                            : "—"}
                        </small>
                        <div className="study-board">
                          {row.decision.board.map((card) => (
                            <Card card={card} key={card} />
                          ))}
                        </div>
                      </td>
                      <td>
                        {t(`study.${row.decision.action}`)}{" "}
                        {row.decision.size_bb !== null &&
                          `${row.decision.size_bb}bb`}
                      </td>
                      <td>
                        <div className="study-row-actions">
                          <button
                            className="text-button"
                            onClick={() =>
                              open(row.hand, row.decision.reference.seq)
                            }
                          >
                            {t("study.replay")}
                          </button>
                          {row.decision.street === "preflop" && (
                            <button
                              className="text-button"
                              disabled={!pack}
                              onClick={() => compare(row.decision.reference)}
                            >
                              {t("study.compare")}
                            </button>
                          )}
                          <button
                            className="text-button"
                            disabled={mutation.busy}
                            onClick={() => void add([row.decision.reference])}
                          >
                            {t("study.train")}
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {!rows.length && (
                <p className="study-empty">{t("study.noDecisions")}</p>
              )}
            </div>
            {next !== null && next !== undefined && (
              <button
                className="button subtle"
                disabled={mutation.busy}
                onClick={() =>
                  void mutation.run(async () => {
                    const expected = key;
                    const page = await study<SpotReport>({
                      op: "explore",
                      query: { filter, spot, before: next },
                    });
                    if (currentKey.current !== expected) return;
                    setExtra((old) => [
                      ...old,
                      ...page.rows.filter(
                        (r) => !old.some((o) => o.id === r.id),
                      ),
                    ]);
                    setCursor(page.next_cursor);
                  })
                }
              >
                {t("study.loadMore")}
              </button>
            )}
          </>
        )
      )}
    </div>
  );
}
