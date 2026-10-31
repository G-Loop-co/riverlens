import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { study, useStudy, useStudyMutation } from "../study-api";
import type {
  RangeQuizRecord,
  StrategyCell,
  StrategyMatrix,
  StudyItem,
} from "../study-types";
import { decimal, number } from "../format";
import { ErrorBanner, Field } from "./UI";
import { Card } from "./Replayer";

export function quizCells(cells: StrategyCell[]): StrategyCell[] {
  return cells.filter(
    (c) => c.strategy && Object.values(c.strategy).some((v) => v > 0),
  );
}

function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function sampleHands(
  cells: StrategyCell[],
  count: number,
  seed: number,
): StrategyCell[] {
  const pool = quizCells(cells);
  const rand = mulberry32(seed);
  const shuffled = [...pool];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled.slice(0, Math.min(count, shuffled.length));
}

export function scoreFrequencies(
  user: Record<string, number>,
  theory: Record<string, number>,
): { tvd: number; score: number } {
  const keys = new Set([...Object.keys(user), ...Object.keys(theory)]);
  let diff = 0;
  for (const key of keys) {
    diff += Math.abs((user[key] || 0) - (theory[key] || 0));
  }
  const tvd = diff / 2;
  return { tvd, score: Math.round((100 - tvd) * 10) / 10 };
}

export function scoreBand(score: number): "excellent" | "good" | "needsWork" {
  return score >= 95 ? "excellent" : score >= 85 ? "good" : "needsWork";
}

export function displayCards(hand: string): string[] {
  const rank = [...hand.slice(0, 2)];
  if (rank.length < 2) return [];
  return [
    `${rank[0]}s`,
    `${rank[1]}${hand.endsWith("s") ? "s" : "h"}`,
  ];
}

type Entry = {
  hand: string;
  user: Record<string, number>;
  theory: Record<string, number>;
  score: number;
};

export function RangeDrill({
  pack,
  node,
  revision,
  refresh,
  notify,
  goTrainer,
}: {
  pack: string;
  node: string;
  revision: number;
  refresh: () => void;
  notify: (message: string) => void;
  goTrainer: () => void;
}) {
  const { t } = useTranslation();
  const matrix = useStudy<StrategyMatrix>(
    pack && node ? { op: "matrix", pack, node, filter: {} } : null,
    revision,
  );
  const history = useStudy<StudyItem<RangeQuizRecord>[]>(
    { op: "items", kind: "range_quiz" },
    revision,
  );
  const operation = useStudyMutation();
  const [size, setSize] = useState(10);
  const [questions, setQuestions] = useState<StrategyCell[] | null>(null);
  const [index, setIndex] = useState(0);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [frequencies, setFrequencies] = useState<Record<string, string>>({});
  const current = questions?.[index];
  const actions = matrix.data?.node.actions || [];
  useEffect(() => {
    setQuestions(null);
    setEntries([]);
    setIndex(0);
    setFrequencies({});
  }, [pack, node]);
  const theory = current?.strategy || {};
  const theoryView: Record<string, number> = Object.fromEntries(
    actions.map((a) => [a.code, (theory[a.code] || 0) * 100]),
  );
  const total = actions.reduce(
    (n, a) => n + Number(frequencies[a.code] || 0),
    0,
  );
  const canSubmit =
    actions.length > 0 && Math.abs(total - 100) < 0.1 && !operation.busy;
  function start() {
    const cells = matrix.data?.cells || [];
    const picked = sampleHands(cells, size, Date.now());
    if (picked.length === 0) {
      notify(t("study.rangeEmpty"));
      return;
    }
    setQuestions(picked);
    setIndex(0);
    setEntries([]);
    setFrequencies({});
  }
  function submit() {
    if (!current || !canSubmit) return;
    const user = Object.fromEntries(
      actions.map((a) => [a.code, Number(frequencies[a.code] || 0)]),
    );
    const { score } = scoreFrequencies(user, theoryView);
    setEntries((e) => [
      ...e,
      { hand: current.hand, user, theory: theoryView, score },
    ]);
  }
  function next() {
    if (!questions) return;
    if (index + 1 >= questions.length) return;
    setIndex((i) => i + 1);
    setFrequencies({});
  }
  const avg = entries.length
    ? Math.round(
        (entries.reduce((n, e) => n + e.score, 0) / entries.length) * 10,
      ) / 10
    : 0;
  const complete = questions !== null && entries.length === questions.length;
  return (
    <div className="study-range">
      <section className="panel study-intro">
        <div>
          <span className="section-kicker">RANGE DRILL</span>
          <h2>{t("study.rangeTitle")}</h2>
          <p>{t("study.rangeHelp")}</p>
          <p className="study-warning">{t("study.rangeMixedNote")}</p>
        </div>
        <button className="button subtle" onClick={goTrainer}>
          {t("study.openTrainer")}
        </button>
      </section>
      {(matrix.error || history.error || operation.error) && (
        <ErrorBanner>
          {matrix.error || history.error || operation.error}
        </ErrorBanner>
      )}
      {!pack || !node ? (
        <section className="panel">
          <p className="muted">{t("study.rangeNeedNode")}</p>
        </section>
      ) : matrix.data && quizCells(matrix.data.cells).length === 0 ? (
        <section className="panel">
          <p className="muted">{t("study.rangeEmpty")}</p>
        </section>
      ) : !questions ? (
        <section className="panel study-start">
          <Field label={t("study.rangeQuestions")}>
            <select
              value={size}
              onChange={(e) => setSize(Number(e.target.value))}
            >
              <option value={10}>10</option>
              <option value={20}>20</option>
            </select>
          </Field>
          <button
            className="button primary"
            disabled={!matrix.data || operation.busy}
            onClick={start}
          >
            {t("study.rangeStart")}
          </button>
        </section>
      ) : complete ? (
        <section className="panel study-start">
          <h3>
            {t("study.rangeDone", { n: entries.length, avg: decimal(avg, 1) })}
          </h3>
          <div className="study-metrics">
            <div>
              <span>{t("study.rangeAvg")}</span>
              <strong>{decimal(avg, 1)}</strong>
            </div>
            <div>
              <span>{t("study.rangeBandExcellent")}</span>
              <strong>{entries.filter((e) => e.score >= 95).length}</strong>
            </div>
            <div>
              <span>{t("study.rangeBandNeedsWork")}</span>
              <strong>{entries.filter((e) => e.score < 85).length}</strong>
            </div>
          </div>
          <div className="study-row-actions">
            <button
              className="button primary"
              disabled={operation.busy}
              onClick={() =>
                void operation.run(async () => {
                  await study({
                    op: "save_item",
                    kind: "range_quiz",
                    data: {
                      name: `${pack}:${node}`,
                      pack,
                      node,
                      count: entries.length,
                      avg,
                      created: Math.floor(Date.now() / 1000),
                    },
                  });
                  notify(t("study.rangeSaved"));
                  refresh();
                })
              }
            >
              {t("study.rangeSave")}
            </button>
            <button className="button subtle" onClick={start}>
              {t("study.rangeAgain")}
            </button>
          </div>
        </section>
      ) : current && (
        <section className="panel study-question">
          <div className="study-row-actions">
            <span className="section-kicker">
              {t("study.questionNumber", { n: index + 1, total: questions.length })}
            </span>
            <span className="subtle-badge">{t("study.gtoExercise")}</span>
          </div>
          <progress
            max={questions.length}
            value={index}
            aria-label={t("study.progress")}
          />
          <h2>
            {matrix.data?.node.hero} · {t("study.preflop")}:{" "}
            {matrix.data?.node.path}
          </h2>
          <div className="study-question-cards">
            {displayCards(current.hand).map((card) => (
              <Card key={card} card={card} />
            ))}
            <span className="study-card-divider" />
            <span className="mono">{current.hand}</span>
          </div>
          {current.strategy === null || entries.length <= index ? (
            <>
              <div className="study-frequency-inputs">
                {actions.map((a) => (
                  <Field key={a.code} label={`${a.label} %`}>
                    <input
                      type="number"
                      step="0.1"
                      min="0"
                      max="100"
                      value={frequencies[a.code] || ""}
                      onChange={(e) =>
                        setFrequencies((f) => ({
                          ...f,
                          [a.code]: e.target.value,
                        }))
                      }
                    />
                  </Field>
                ))}
              </div>
              <p className="muted">
                {t("study.rangeTotal", { total: decimal(total, 1) })}
              </p>
              <button
                className="button primary"
                disabled={!canSubmit}
                onClick={submit}
              >
                {t("study.rangeSubmit")}
              </button>
            </>
          ) : (
            (() => {
              const entry = entries[index];
              return (
                <>
                  <table className="study-range-result">
                    <thead>
                      <tr>
                        <th>{t("study.rangeYourAnswer")}</th>
                        <th>{t("study.rangeTheory")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {actions.map((a) => (
                        <tr key={a.code}>
                          <td>
                            {a.label}: {decimal(entry.user[a.code] || 0, 1)}%
                          </td>
                          <td>
                            {a.label}:{" "}
                            {decimal(entry.theory[a.code] || 0, 1)}%
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  <p className="study-warning">
                    {t(`study.rangeBand.${scoreBand(entry.score)}`, {
                      score: decimal(entry.score, 1),
                    })}
                  </p>
                  <button
                    className="button primary"
                    onClick={next}
                  >
                    {index + 1 >= questions.length
                      ? t("study.rangeFinish")
                      : t("study.rangeNext")}
                  </button>
                </>
              );
            })()
          )}
        </section>
      )}
      {history.data && history.data.length > 0 && (
        <section className="panel">
          <h3>{t("study.rangeHistory")}</h3>
          <ul>
            {history.data
              .slice(-5)
              .reverse()
              .map((item) => (
                <li key={item.id}>
                  {new Date(item.data.created * 1000).toLocaleDateString()} ·{" "}
                  {item.data.name} · {number(item.data.count)}{" "}
                  {t("study.rangeQuestionCount")} ·{" "}
                  {t("study.rangeScore")}: {decimal(item.data.avg, 1)}
                </li>
              ))}
          </ul>
        </section>
      )}
    </div>
  );
}
