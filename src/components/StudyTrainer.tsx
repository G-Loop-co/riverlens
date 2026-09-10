import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { study, useStudy, useStudyMutation } from "../study-api";
import type {
  TrainingAnswer,
  TrainingCurrent,
  TrainingState,
} from "../study-types";
import { decimal, number } from "../format";
import { ErrorBanner, Field } from "./UI";
import { Card } from "./Replayer";
import { StudySelect } from "./StudyWorkspace";

export function Trainer({
  revision,
  refresh,
  open,
  findSpots,
}: {
  revision: number;
  refresh: () => void;
  open: (id: number, step: number) => void;
  findSpots: () => void;
}) {
  const { t } = useTranslation();
  const state = useStudy<TrainingState>({ op: "training_state" }, revision);
  const [current, setCurrent] = useState<TrainingCurrent | null>(null),
    [limit, setLimit] = useState(20);
  const [mode, setMode] = useState("frequency"),
    [chosen, setChosen] = useState(""),
    [note, setNote] = useState("");
  const [frequencies, setFrequencies] = useState<Record<string, string>>({});
  const operation = useStudyMutation();
  useEffect(() => {
    setChosen("");
    setNote("");
    setFrequencies({});
    setMode(current?.source === "gto" ? "frequency" : "action");
  }, [current?.card_id, current?.source]);
  const q = current?.question,
    feedback = current?.feedback;
  const options =
    q?.options?.map((a) => [a.code, a.label] as const) ||
    q?.legal?.map((a) => [a, `study.${a}`] as const) ||
    [];
  const total = options.reduce(
    (n, [key]) => n + Number(frequencies[key] || 0),
    0,
  );
  async function submit() {
    if (!current?.card_id) return;
    const answer: TrainingAnswer =
      mode === "frequency"
        ? {
            mode: "frequency",
            frequencies: Object.fromEntries(
              options.map(([key]) => [key, Number(frequencies[key] || 0)]),
            ),
            note,
          }
        : { mode: "action", action: chosen, note };
    setCurrent(
      await study<TrainingCurrent>({
        op: "training_answer",
        session: current.id,
        card: current.card_id,
        answer,
      }),
    );
    refresh();
  }
  async function rate(rating: "repeat" | "advance" | "skip") {
    if (!current?.card_id) return;
    setCurrent(
      await study<TrainingCurrent>({
        op: "training_rate",
        session: current.id,
        card: current.card_id,
        rating,
      }),
    );
    refresh();
  }
  return (
    <div className="study-trainer">
      <section className="panel study-intro">
        <div>
          <span className="section-kicker">DAILY PRACTICE</span>
          <h2>{t("study.trainerTitle")}</h2>
          <p>{t("study.trainerHelp")}</p>
        </div>
        <button className="button subtle" onClick={findSpots}>
          {t("study.findSpots")}
        </button>
      </section>
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
      </div>
      {(state.error || operation.error) && (
        <ErrorBanner>{state.error || operation.error}</ErrorBanner>
      )}
      {!current || current.complete ? (
        <section className="panel study-start">
          {current?.complete && (
            <h3>{t("study.sessionComplete", { n: current.total })}</h3>
          )}
          <Field label={t("study.dailyLimit")}>
            <input
              type="number"
              min="1"
              max="200"
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
            />
          </Field>
          <button
            className="button primary"
            disabled={
              operation.busy ||
              (!state.data?.due && !state.data?.active) ||
              (!state.data?.active &&
                (!Number.isInteger(limit) || limit < 1 || limit > 200))
            }
            onClick={() =>
              void operation.run(async () => {
                setCurrent(
                  await study<TrainingCurrent>(
                    state.data?.active
                      ? { op: "training_current", session: state.data.active }
                      : { op: "training_start", limit },
                  ),
                );
                refresh();
              })
            }
          >
            {t(
              state.data?.active ? "study.resumeSession" : "study.startSession",
            )}
          </button>
          {!state.data?.due && !state.data?.active && (
            <p className="muted">{t("study.noDue")}</p>
          )}
        </section>
      ) : (
        q && (
          <section className="panel study-question">
            <div className="study-row-actions">
              <span className="section-kicker">
                {t("study.questionNumber", {
                  n: current.index + 1,
                  total: current.total,
                })}
              </span>
              <span className="subtle-badge">
                {t(
                  current.source === "gto"
                    ? "study.gtoExercise"
                    : "study.selfExercise",
                )}
              </span>
            </div>
            <progress
              max={current.total}
              value={current.index}
              aria-label={t("study.progress")}
            />
            <h2>
              {q.position} {q.opponent && `vs ${q.opponent}`} ·{" "}
              {t(`study.${q.street}`)}
            </h2>
            <div className="study-question-cards">
              {q.cards.map((card) => (
                <Card key={card} card={card} />
              ))}
              <span className="study-card-divider" />
              {q.board.map((card) => (
                <Card key={card} card={card} />
              ))}
            </div>
            <div className="study-question-context">
              <span>{q.theory ? "100bb" : q.role}</span>
              {q.pot_before && (
                <span>
                  {t("study.pot")}: ${q.pot_before}
                </span>
              )}
              {q.to_call && (
                <span>
                  {t("study.toCall")}: ${q.to_call}
                </span>
              )}
            </div>
            {q.preflop_path && (
              <p className="study-line mono">
                {t("study.preflop")}: {q.preflop_path}
              </p>
            )}
            {q.line.length > 0 && (
              <ol className="study-history">
                {q.line.map((a, i) => (
                  <li key={i}>
                    {t(`study.${a.street}`)} ·{" "}
                    {a.actor === "hero" ? "Hero" : t("study.villain")} ·{" "}
                    {t(`study.${a.action}`)}
                  </li>
                ))}
              </ol>
            )}
            {current.stale ? (
              <>
                <p className="study-warning">{t("study.staleQuestion")}</p>
                <button
                  className="button primary"
                  disabled={operation.busy}
                  onClick={() => void operation.run(() => rate("skip"))}
                >
                  {t("study.skip")}
                </button>
              </>
            ) : !feedback ? (
              <>
                {current.source === "gto" && (
                  <div className="segmented">
                    {["frequency", "action"].map((m) => (
                      <button
                        key={m}
                        aria-pressed={mode === m}
                        className={mode === m ? "active" : ""}
                        onClick={() => setMode(m)}
                      >
                        {t(`study.mode.${m}`)}
                      </button>
                    ))}
                  </div>
                )}
                {mode === "frequency" ? (
                  <>
                    <div className="study-frequency-inputs">
                      {options.map(([key, label]) => (
                        <Field key={key} label={`${label} %`}>
                          <input
                            type="number"
                            step="0.1"
                            min="0"
                            max="100"
                            value={frequencies[key] || ""}
                            onChange={(e) =>
                              setFrequencies((f) => ({
                                ...f,
                                [key]: e.target.value,
                              }))
                            }
                          />
                        </Field>
                      ))}
                    </div>
                    <p className="mono">
                      {t("study.frequencyTotal", { n: decimal(total, 1) })}
                    </p>
                  </>
                ) : (
                  <StudySelect
                    label="study.yourAction"
                    emptyLabel="study.chooseAction"
                    value={chosen}
                    options={options}
                    change={(v) => setChosen(v || "")}
                  />
                )}
                <Field label={t("study.reasoning")}>
                  <textarea
                    maxLength={4000}
                    rows={3}
                    value={note}
                    onChange={(e) => setNote(e.target.value)}
                  />
                </Field>
                <button
                  className="button primary"
                  disabled={
                    operation.busy ||
                    (mode === "frequency"
                      ? Math.abs(total - 100) > 0.01 ||
                        options.some(([key]) => {
                          const v = Number(frequencies[key] || 0);
                          return !Number.isFinite(v) || v < 0 || v > 100;
                        })
                      : !chosen)
                  }
                  onClick={() => void operation.run(submit)}
                >
                  {t("study.submitAnswer")}
                </button>
              </>
            ) : (
              <div className="study-feedback" aria-live="polite">
                <h3>{t("study.feedback")}</h3>
                {feedback.feedback.in_strategy !== undefined &&
                  feedback.feedback.in_strategy !== null && (
                    <p>
                      {t(
                        feedback.feedback.in_strategy
                          ? "study.inStrategy"
                          : "study.outOfStrategy",
                      )}
                    </p>
                  )}
                {feedback.feedback.max_gap !== undefined && (
                  <p>
                    {t("study.maxGap", {
                      n: decimal(feedback.feedback.max_gap, 1),
                    })}
                  </p>
                )}
                {feedback.expected && (
                  <div className="study-answer-grid">
                    {Object.entries(feedback.expected).map(([action, f]) => (
                      <div key={action}>
                        <strong>
                          {q.options?.find((a) => a.code === action)?.label ||
                            action}
                        </strong>
                        <span>{decimal(f * 100, 2)}%</span>
                        {feedback.feedback.differences && (
                          <small>
                            {decimal(feedback.feedback.differences[action], 1)}{" "}
                            pp
                          </small>
                        )}
                      </div>
                    ))}
                  </div>
                )}
                {feedback.observed && (
                  <p>
                    {t("study.originalAction")}: {feedback.observed}
                  </p>
                )}
                {feedback.source_note && (
                  <p className="study-user-note">{feedback.source_note}</p>
                )}
                {feedback.answer.note && (
                  <p className="study-user-note">
                    {t("study.yourNote")}: {feedback.answer.note}
                  </p>
                )}
                {feedback.replay_hand !== null && (
                  <button
                    className="text-button"
                    onClick={() =>
                      open(feedback.replay_hand!, feedback.reference?.seq || 0)
                    }
                  >
                    {t("study.fullReplay")}
                  </button>
                )}
                <p className="muted">{t("study.rateHelp")}</p>
                <div className="study-row-actions">
                  <button
                    className="button subtle"
                    disabled={operation.busy}
                    onClick={() => void operation.run(() => rate("repeat"))}
                  >
                    {t("study.repeat")}
                  </button>
                  <button
                    className="button primary"
                    disabled={operation.busy}
                    onClick={() => void operation.run(() => rate("advance"))}
                  >
                    {t("study.advance")}
                  </button>
                </div>
              </div>
            )}
          </section>
        )
      )}
      {!!state.data?.recent.length && (
        <section className="panel study-recent">
          <h3>{t("study.recentPractice")}</h3>
          <ul>
            {state.data.recent.map((r, i) => (
              <li key={`${r.created}-${i}`}>
                <span>{r.reference?.hand_id || t("study.gtoExercise")}</span>
                <span>
                  {t(r.rating === "repeat" ? "study.repeat" : "study.advance")}
                </span>
                {r.feedback.max_gap !== undefined && (
                  <span>{decimal(r.feedback.max_gap, 1)} pp</span>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}
      <p className="study-footnote">{t("study.trainerCaveat")}</p>
    </div>
  );
}
