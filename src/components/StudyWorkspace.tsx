import { useEffect, useState } from "react";
import "../study.css";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { api } from "../api";
import { useStudy, useStudyMutation } from "../study-api";
import { decimal } from "../format";
import type { Filter, Health, Profile } from "../types";
import type { DecisionRef, PackSummary, SpotDefinition } from "../study-types";
import { Field, ErrorBanner } from "./UI";
import { RouteTabs, TabPanel } from "./RouteUI";
import { SpotExplorer } from "./StudySpots";
import { LeakFinder } from "./StudyLeaks";
import { StrategyCompare } from "./StudyStrategy";
import { Trainer } from "./StudyTrainer";
import { RangeDrill } from "./StudyRange";
import { StudyProgress } from "./StudyProgress";
import type { TrainingFeedback } from "../study-types";

export type StudyTab =
  "progress" | "spots" | "leaks" | "strategy" | "range" | "trainer";
export const emptySpot = (): SpotDefinition => ({ line: [] });
export function StudySelect({
  label,
  value,
  options,
  change,
  all = true,
  emptyLabel = "study.all",
  disabled = false,
}: {
  label: string;
  value?: string;
  options: readonly (readonly [string, string])[];
  change: (value: string | undefined) => void;
  all?: boolean;
  emptyLabel?: string;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <Field label={t(label)}>
      <select
        value={value || ""}
        onChange={(e) => change(e.target.value || undefined)}
        disabled={disabled}
      >
        {all && <option value="">{t(emptyLabel)}</option>}
        {options.map(([key, label]) => (
          <option key={key} value={key}>
            {label.startsWith("study.") ? t(label) : label}
          </option>
        ))}
      </select>
    </Field>
  );
}
export const positions = ["UTG", "HJ", "CO", "BTN", "SB", "BB", "BTN/SB"].map(
  (p) => [p, p] as const,
);
export const streets = ["preflop", "flop", "turn", "river"].map(
  (s) => [s, `study.${s}`] as const,
);
export const actions = ["fold", "check", "call", "bet", "raise"].map(
  (a) => [a, `study.${a}`] as const,
);

export function StudyRecentPractice({
  recent,
  openReplay,
}: {
  recent: TrainingFeedback[];
  openReplay?: (hand: number, seq: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <section className="panel study-recent">
      <h3>{t("study.recentPractice")}</h3>
      <ul>
        {recent.map((r, i) => (
          <li key={`${r.created}-${i}`}>
            <span>{r.reference?.hand_id || t("study.gtoExercise")}</span>
            <span>
              {t(r.rating === "repeat" ? "study.repeat" : "study.advance")}
            </span>
            {r.feedback.max_gap !== undefined && (
              <span>{decimal(r.feedback.max_gap, 1)} pp</span>
            )}
            {openReplay && r.replay_hand !== null && (
              <button
                className="text-button"
                onClick={() =>
                  openReplay(r.replay_hand!, r.reference?.seq ?? 0)
                }
              >
                {t("study.replay")}
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

export function StudyWorkspace({
  profiles,
  health,
  revision,
  refresh,
  open,
  tab,
  onTabChange,
  notify,
}: {
  profiles: Profile[];
  health?: Health;
  revision: number;
  refresh: () => void;
  open: (id: number, step: number) => void;
  tab: StudyTab;
  onTabChange: (tab: StudyTab) => void;
  notify: (message: string) => void;
}) {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const [filter, setFilter] = useState<Filter>({});
  const [spot, setSpot] = useState<SpotDefinition>(emptySpot);
  const pack = searchParams.get("pack") || "";
  const node = searchParams.get("node") || "";
  const [reference, setReference] = useState<DecisionRef | null>(null);
  const packs = useStudy<PackSummary[]>({ op: "packs" }, revision);
  const operation = useStudyMutation();
  function updateStudyParams(updates: Record<string, string | undefined>) {
    const next = new URLSearchParams(searchParams);
    for (const [key, value] of Object.entries(updates)) {
      if (value) next.set(key, value);
      else next.delete(key);
    }
    setSearchParams(next, { replace: true });
  }
  const setPack = (value: string) => updateStudyParams({ pack: value });
  const setNode = (value: string) => updateStudyParams({ node: value });
  useEffect(() => {
    if (packs.data && !pack && packs.data[0] && !searchParams.has("pack")) {
      updateStudyParams({ pack: packs.data[0].id });
    }
  }, [packs.data, pack, searchParams]);
  const goStrategy = (id: string, ref?: DecisionRef) => {
    updateStudyParams({ node: id });
    setReference(ref || null);
    onTabChange("strategy");
  };
  function updateFilter(patch: Partial<Filter>) {
    setFilter((f) => ({ ...f, ...patch }));
  }
  return (
    <div className="study-workspace">
      <RouteTabs
        label="study.flow"
        panelId="study-workspace"
        value={tab}
        onChange={onTabChange}
        items={[
          { value: "progress", label: "study.progress" },
          { value: "spots", label: "study.spots" },
          { value: "leaks", label: "study.leaks" },
          { value: "strategy", label: "study.strategy" },
          { value: "range", label: "study.range" },
          { value: "trainer", label: "study.trainer" },
        ]}
      />
      <TabPanel id="study-workspace" activeTab={tab}>
        <div className="study-status" role="status">
          <span>
            {t("study.coverage", {
              indexed: health?.study?.indexed ?? 0,
              total: health?.study?.total ?? 0,
            })}
          </span>
          {health?.study && !health.study.complete && (
            <strong>{t("study.partial")}</strong>
          )}
          {(health?.study_running ||
            health?.study_paused ||
            health?.study_error) && (
            <button
              className="text-button"
              disabled={operation.busy}
              onClick={() =>
                void operation.run(async () => {
                  await api({
                    op: "study_control",
                    paused: !health?.study_paused,
                  });
                  refresh();
                })
              }
            >
              {t(
                health?.study_paused ? "study.resumeIndex" : "study.pauseIndex",
              )}
            </button>
          )}
        </div>
        {(health?.study_error || operation.error || packs.error) && (
          <ErrorBanner>
            {health?.study_error || operation.error || packs.error}
          </ErrorBanner>
        )}
        {tab !== "trainer" && (
          <section
            className="study-cohort panel"
            aria-label={t("study.cohort")}
          >
            <StudySelect
              label="study.profile"
              value={filter.profile}
              options={profiles.map((p) => [p.id, p.name])}
              change={(profile) => updateFilter({ profile })}
            />
            <Field label={t("study.from")}>
              <input
                type="date"
                value={filter.date_from || ""}
                onChange={(e) =>
                  updateFilter({ date_from: e.target.value || undefined })
                }
              />
            </Field>
            <Field label={t("study.to")}>
              <input
                type="date"
                value={filter.date_to || ""}
                onChange={(e) =>
                  updateFilter({ date_to: e.target.value || undefined })
                }
              />
            </Field>
            <StudySelect
              label="study.game"
              value={filter.game}
              options={[
                ["Cash", "Cash"],
                ["Rush", "Rush"],
                ["Tournament", t("MTT／錦標賽")],
              ]}
              change={(game) =>
                updateFilter({
                  game,
                  currency: game === "Tournament" ? "CHIPS" : undefined,
                  tournament_id: undefined,
                  stakes: undefined,
                })
              }
            />
            <Field label={t("賽事編號")}>
              <input
                value={filter.tournament_id || ""}
                onChange={(e) =>
                  updateFilter({
                    tournament_id: e.target.value || undefined,
                    game: "Tournament",
                    currency: "CHIPS",
                  })
                }
              />
            </Field>
            <StudySelect
              label="study.stakes"
              value={filter.stakes}
              options={[
                "0.01/0.02",
                "0.02/0.05",
                "0.05/0.10",
                "0.10/0.25",
                "0.25/0.50",
                "0.50/1.00",
                "1.00/2.00",
              ].map((s) => [s, s])}
              change={(stakes) => updateFilter({ stakes })}
            />
            <StudySelect
              label="study.pack"
              value={pack}
              options={(packs.data || []).map((p) => [p.id, p.name])}
              all={false}
              change={(p) => {
                setPack(p || "");
                setNode("");
              }}
            />
          </section>
        )}
        {tab === "progress" && (
          <StudyProgress
            filter={filter}
            pack={pack}
            revision={revision}
            open={open}
            goTrainer={() => onTabChange("trainer")}
          />
        )}
        {tab === "spots" && (
          <SpotExplorer
            filter={filter}
            spot={spot}
            setSpot={setSpot}
            pack={pack}
            revision={revision}
            refresh={refresh}
            open={open}
            compare={(r) => goStrategy("", r)}
            notify={notify}
            onLeaks={() => onTabChange("leaks")}
          />
        )}
        {tab === "leaks" && (
          <LeakFinder
            filter={filter}
            spot={spot}
            pack={pack}
            revision={revision}
            refresh={refresh}
            explore={(s) => {
              setSpot(s);
              onTabChange("spots");
            }}
            strategy={goStrategy}
          />
        )}
        {tab === "strategy" && (
          <StrategyCompare
            filter={filter}
            pack={pack}
            setPack={setPack}
            node={node}
            setNode={setNode}
            reference={reference}
            clearReference={() => setReference(null)}
            revision={revision}
            refresh={refresh}
            notify={notify}
            open={open}
            trainer={() => onTabChange("trainer")}
          />
        )}
        {tab === "range" && (
          <RangeDrill
            pack={pack}
            node={node}
            revision={revision}
            refresh={refresh}
            notify={notify}
            goTrainer={() => onTabChange("trainer")}
          />
        )}
        {tab === "trainer" && (
          <Trainer
            revision={revision}
            refresh={refresh}
            open={open}
            findSpots={() => onTabChange("spots")}
          />
        )}
      </TabPanel>
    </div>
  );
}
