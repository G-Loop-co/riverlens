import { useTranslation } from "react-i18next";
import { api, chooseOutput, desktop } from "../api";
import { useStudy } from "../study-api";
import type { Filter } from "../types";
import type {
  LeakReport,
  LeakRow,
  RangeQuizRecord,
  StudyItem,
  TrainingState,
} from "../study-types";
import { decimal, number } from "../format";
import { ErrorBanner, Skeleton } from "./UI";
import { StudyRecentPractice } from "./StudyWorkspace";

export type LeakStatus = "onTrack" | "tooFew" | "offTarget";

export function leakStatus(row: LeakRow): LeakStatus {
  if (
    !row.enough ||
    row.actual === null ||
    row.low === null ||
    row.high === null
  ) {
    return "tooFew";
  }
  return row.actual >= row.low && row.actual <= row.high
    ? "onTrack"
    : "offTarget";
}

export function StudyProgress({
  filter,
  pack,
  revision,
  open,
  goTrainer,
}: {
  filter: Filter;
  pack: string;
  revision: number;
  open: (id: number, step: number) => void;
  goTrainer: () => void;
}) {
  const { t } = useTranslation();
  const state = useStudy<TrainingState>({ op: "training_state" }, revision);
  const quizzes = useStudy<StudyItem<RangeQuizRecord>[]>(
    { op: "items", kind: "range_quiz" },
    revision,
  );
  const leaks = useStudy<LeakReport>(
    { op: "leaks", filter, ...(pack ? { pack } : {}) },
    revision,
  );
  const rows = (leaks.data?.rows || [])
    .filter((r) => r.priority !== null && r.priority > 0)
    .sort((a, b) => (b.priority || 0) - (a.priority || 0));
  const counts = { onTrack: 0, tooFew: 0, offTarget: 0 };
  // Only user-defined targets (custom / pack-backed) count toward target
  // status; observation presets are descriptive listings without targets.
  for (const r of leaks.data?.rows || []) {
    if (r.source !== "observation") counts[leakStatus(r)] += 1;
  }
  const recent = state.data?.recent.slice(0, 8) || [];
  async function exportCsv(kind: string, filename: string) {
    const path = await chooseOutput("csv", filename);
    if (!path) return;
    await api({
      op: "export_study",
      kind,
      path,
    });
  }
  return (
    <div className="study-progress">
      <section className="panel study-intro">
        <div>
          <span className="section-kicker">PROGRESS</span>
          <h2>{t("study.progressTitle")}</h2>
          <p>{t("study.progressHelp")}</p>
        </div>
        <button className="button subtle" onClick={goTrainer}>
          {t("study.progressGoTrainer")}
        </button>
      </section>
      {desktop && (
        <div className="study-row-actions">
          <button
            className="button subtle"
            onClick={() => void exportCsv("attempts", "riverlens-practice.csv")}
          >
            {t("study.exportAttempts")}
          </button>
          <button
            className="button subtle"
            onClick={() =>
              void exportCsv("range_quiz", "riverlens-range-drills.csv")
            }
          >
            {t("study.exportDrills")}
          </button>
          <button
            className="button subtle"
            onClick={() =>
              void exportCsv("leak_snapshots", "riverlens-leak-snapshots.csv")
            }
          >
            {t("study.exportSnapshots")}
          </button>
        </div>
      )}
      {(state.error || quizzes.error || leaks.error) && (
        <ErrorBanner>
          {state.error || quizzes.error || leaks.error}
        </ErrorBanner>
      )}
      {state.loading ? (
        <Skeleton rows={2} />
      ) : (
        <div className="study-metrics">
          <div>
            <span>{t("study.cardsTotal")}</span>
            <strong>{number(state.data?.total || 0)}</strong>
          </div>
          <div>
            <span>{t("study.due")}</span>
            <strong>{number(state.data?.due || 0)}</strong>
          </div>
          <div>
            <span>{t("study.reviewsCompleted")}</span>
            <strong>{number(state.data?.completed || 0)}</strong>
          </div>
          <div>
            <span>{t("study.progressOnTrack")}</span>
            <strong>{number(counts.onTrack)}</strong>
          </div>
          <div>
            <span>{t("study.progressTooFew")}</span>
            <strong>{number(counts.tooFew)}</strong>
          </div>
          <div>
            <span>{t("study.progressOffTarget")}</span>
            <strong>{number(counts.offTarget)}</strong>
          </div>
        </div>
      )}
      {!!recent.length && (
        <StudyRecentPractice
          recent={recent}
          openReplay={(hand, seq) => open(hand, seq)}
        />
      )}
      <section className="panel">
        <h3>{t("study.progressLeaks")}</h3>
        {rows.length ? (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>{t("情境")}</th>
                  <th>{t("study.progressActual")}</th>
                  <th>{t("study.progressTarget")}</th>
                  <th>Wilson 95%</th>
                  <th>{t("study.opportunities")}</th>
                  <th>{t("study.progressStatus")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.slice(0, 5).map((r) => (
                  <tr key={r.id}>
                    <td>
                      {r.name}
                      <br />
                      <small>
                        {r.action} · {r.source}
                      </small>
                    </td>
                    <td>{r.actual === null ? "—" : `${decimal(r.actual, 1)}%`}</td>
                    <td>
                      {r.low === null || r.high === null
                        ? "—"
                        : `${decimal(r.low, 0)}–${decimal(r.high, 0)}%`}
                    </td>
                    <td>
                      {r.interval
                        ? `${decimal(r.interval[0], 1)}–${decimal(r.interval[1], 1)}%`
                        : "—"}
                    </td>
                    <td>{number(r.opportunities)}</td>
                    <td>{t(`study.progressStatus.${leakStatus(r)}`)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="muted">{t("study.progressNoLeaks")}</p>
        )}
      </section>
      {!!quizzes.data?.length && (
        <section className="panel">
          <h3>{t("study.progressRangeTrend")}</h3>
          <ul>
            {quizzes.data
              .slice(-5)
              .reverse()
              .map((item) => (
                <li key={item.id}>
                  {new Date(item.data.created * 1000).toLocaleDateString()} ·{" "}
                  {item.data.name} · {number(item.data.count)}{" "}
                  {t("study.rangeQuestionCount")} · {t("study.rangeScore")}:{" "}
                  <span>{decimal(item.data.avg, 1)}</span>
                </li>
              ))}
          </ul>
        </section>
      )}
      <p className="study-footnote">{t("study.progressCaveat")}</p>
    </div>
  );
}
