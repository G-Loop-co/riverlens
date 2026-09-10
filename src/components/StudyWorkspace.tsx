import { useEffect, useState } from "react";
import "../study.css";
import { useTranslation } from "react-i18next";
import {
  ArrowRight,
  Crosshair,
  GraduationCap,
  MagnifyingGlass,
  TrendUp,
} from "@phosphor-icons/react";
import { api } from "../api";
import { useStudy, useStudyMutation } from "../study-api";
import { readPreference, writePreference } from "../preferences";
import type { Filter, Health, Profile } from "../types";
import type { DecisionRef, PackSummary, SpotDefinition } from "../study-types";
import { Field, ErrorBanner } from "./UI";
import { SpotExplorer } from "./StudySpots";
import { LeakFinder } from "./StudyLeaks";
import { StrategyCompare } from "./StudyStrategy";
import { Trainer } from "./StudyTrainer";

export type StudyTab = "spots" | "leaks" | "strategy" | "trainer";
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

export function StudyWorkspace({
  profiles,
  health,
  revision,
  refresh,
  open,
  notify,
}: {
  profiles: Profile[];
  health?: Health;
  revision: number;
  refresh: () => void;
  open: (id: number, step: number) => void;
  notify: (message: string) => void;
}) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<StudyTab>("spots");
  const [filter, setFilter] = useState<Filter>({});
  const [spot, setSpot] = useState<SpotDefinition>(emptySpot);
  const [pack, setPack] = useState(() =>
    readPreference("riverlens-study-pack"),
  );
  const [node, setNode] = useState("");
  const [reference, setReference] = useState<DecisionRef | null>(null);
  const packs = useStudy<PackSummary[]>({ op: "packs" }, revision);
  const operation = useStudyMutation();
  useEffect(() => {
    writePreference("riverlens-study-pack", pack);
  }, [pack]);
  useEffect(() => {
    if (packs.data && !packs.data.some((p) => p.id === pack))
      setPack(packs.data[0]?.id || "");
  }, [packs.data, pack]);
  const goStrategy = (id: string, ref?: DecisionRef) => {
    setNode(id);
    setReference(ref || null);
    setTab("strategy");
  };
  function updateFilter(patch: Partial<Filter>) {
    setFilter((f) => ({ ...f, ...patch }));
  }
  return (
    <div className="study-workspace">
      <nav className="study-steps" aria-label={t("study.flow")}>
        {(
          [
            ["spots", MagnifyingGlass],
            ["leaks", TrendUp],
            ["strategy", Crosshair],
            ["trainer", GraduationCap],
          ] as const
        ).map(([key, Icon], i) => (
          <button
            key={key}
            className={tab === key ? "active" : ""}
            aria-current={tab === key ? "page" : undefined}
            onClick={() => setTab(key)}
          >
            <span className="step-number">0{i + 1}</span>
            <Icon size={20} />
            <span>{t(`study.${key}`)}</span>
            {i < 3 && <ArrowRight size={14} />}
          </button>
        ))}
      </nav>
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
            {t(health?.study_paused ? "study.resumeIndex" : "study.pauseIndex")}
          </button>
        )}
      </div>
      {(health?.study_error || operation.error || packs.error) && (
        <ErrorBanner>
          {health?.study_error || operation.error || packs.error}
        </ErrorBanner>
      )}
      {tab !== "trainer" && (
        <section className="study-cohort panel" aria-label={t("study.cohort")}>
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
            ]}
            change={(game) => updateFilter({ game })}
          />
          <StudySelect label="study.stakes" value={filter.stakes} options={["0.01/0.02","0.02/0.05","0.05/0.10","0.10/0.25","0.25/0.50","0.50/1.00","1.00/2.00"].map(s=>[s,s])} change={stakes=>updateFilter({stakes})}/>
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
          onLeaks={() => setTab("leaks")}
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
            setTab("spots");
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
          trainer={() => setTab("trainer")}
        />
      )}
      {tab === "trainer" && (
        <Trainer
          revision={revision}
          refresh={refresh}
          open={open}
          findSpots={() => setTab("spots")}
        />
      )}
    </div>
  );
}
