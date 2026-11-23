import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useRpc } from "../api";
import { decimal, number, time, currencySymbol, money } from "../format";
import type { Filter } from "../types";
import {
  ranges,
  metricIds,
  cohorts,
  priorities,
  assess,
  allowsOverallBand,
  type Counts,
  type Metric,
  type Cohort,
  type ReviewSummary,
} from "../benchmarks";
import { ErrorBanner } from "./UI";
import "./benchmark-review.css";

type Drill = (filter: Partial<Filter>) => void;
function Rate({ counts: [hits, n] }: { counts: Counts }) {
  return <>{n ? decimal((100 * hits) / n, 1) + "%" : "—"}</>;
}

export function LastSession({
  latest,
  drill,
}: {
  latest: NonNullable<ReviewSummary["latest"]>;
  drill: Drill;
}) {
  const { t } = useTranslation();
  return (
    <section className="session-summary">
      <div>
        <h2>{t("review.last")}</h2>
        <p>
          {time(latest.time)} · {number(latest.hands)} {t("review.hands")} ·{" "}
          {latest.currency} {currencySymbol(latest.currency)}
          {money(latest.net, true)}
        </p>
      </div>
      <button
        className="button primary"
        onClick={() =>
          drill({ session: latest.session, currency: latest.currency })
        }
      >
        {t("review.openSession")}
      </button>
    </section>
  );
}

function BlindSpot({
  group,
  filter,
  revision,
  open,
}: {
  group: Cohort;
  filter: Filter;
  revision: number;
  open: (id: number, seq: number) => void;
}) {
  const { t } = useTranslation();
  const [size, setSize] = useState("2.5");
  const query = useRpc<{
    n: number;
    raises: number;
    calls: number;
    rows: {
      id: number;
      seq: number;
      cards: string;
      action: string;
      hand_id: string;
    }[];
  }>(
    {
      op: "review_blind_spot",
      filter: {
        ...filter,
        currency: group.currency,
        stakes: group.stakes,
        game: group.game,
        position: "BB",
      },
      open_bb: size,
    },
    revision,
  );
  return (
    <section className="spot-reference">
      <h4>{t("review.blindSpot")}</h4>
      <label>
        {t("review.openSize")}{" "}
        <select value={size} onChange={(e) => setSize(e.target.value)}>
          {["2", "2.5", "3"].map((s) => (
            <option key={s} value={s}>
              {s} bb
            </option>
          ))}
        </select>
      </label>
      {size === "2.5" ? (
        <p>
          {t("review.model")}{" "}
          <a
            href="https://solvepoker.app/nlhe-stack-depth"
            target="_blank"
            rel="noreferrer"
          >
            SolvePoker · CC BY 4.0
          </a>
        </p>
      ) : (
        <p>{t("review.noSize")}</p>
      )}
      {query.error ? (
        <ErrorBanner>{query.error}</ErrorBanner>
      ) : (
        query.data && (
          <>
            <p>
              {t("review.spotActual", {
                n: query.data.n,
                rate: query.data.n
                  ? decimal((100 * query.data.raises) / query.data.n, 1) + "%"
                  : "—",
                defend: query.data.n
                  ? decimal(
                      (100 * (query.data.raises + query.data.calls)) /
                        query.data.n,
                      1,
                    ) + "%"
                  : "—",
              })}
            </p>
            <div className="spot-examples">
              {query.data.rows.map((r) => (
                <button
                  key={r.id + ":" + r.seq}
                  className="text-button"
                  onClick={() => open(r.id, r.seq)}
                >
                  {r.cards} · {t("study." + r.action)} · #{r.hand_id}
                </button>
              ))}
            </div>
          </>
        )
      )}
    </section>
  );
}

export function BenchmarkReview({
  data,
  filter,
  revision,
  drill,
  open,
}: {
  data: ReviewSummary;
  filter: Filter;
  revision: number;
  drill: Drill;
  open: (id: number, step: number) => void;
}) {
  const { t } = useTranslation();
  const groups = cohorts(data.positions);
  const [selection, setSelection] = useState(""),
    [expanded, setExpanded] = useState<Metric | null>(null),
    [spot, setSpot] = useState(false);
  const group = groups.find((g) => g.key === selection) ?? groups[0];
  const broad = allowsOverallBand(filter),
    top = priorities(groups, filter);
  function select(key: string, id: Metric | null) {
    setSelection(key);
    setExpanded(id);
    setSpot(false);
  }
  const context = (g: Cohort) => [t(g.game), g.currency, g.stakes].join(" · ");
  function handList(id: Metric, mode: string, position?: string) {
    if (group)
      drill({
        game: group.game,
        currency: group.currency,
        stakes: group.stakes,
        player_count: 6,
        position: position ?? filter.position,
        stat: id,
        stat_mode: mode,
      });
  }
  if (!group) return null;
  return (
    <section className="panel benchmark-review" aria-label={t("review.title")}>
      <div className="benchmark-header">
        <div>
          <h2>{t("review.title")}</h2>
          <p>{t("review.coverage")}</p>
        </div>
        <label>
          {t("review.cohort")}{" "}
          <select
            value={group.key}
            onChange={(e) => select(e.target.value, null)}
          >
            {groups.map((g) => (
              <option key={g.key} value={g.key}>
                {context(g)} · {number(g.hands)} {t("review.hands")}
              </option>
            ))}
          </select>
        </label>
      </div>
      {top.length > 0 && (
        <div className="benchmark-priorities">
          <h3>{t("review.priority")}</h3>
          {top.map((r) => (
            <button
              className="button subtle"
              key={r.group.key + r.id}
              onClick={() => select(r.group.key, r.id)}
            >
              {t("bench." + r.id)} · {context(r.group)} ·{" "}
              {t(r.gap > 0 ? "review.above" : "review.below", {
                n: decimal(Math.abs(r.gap), 1),
              })}
            </button>
          ))}
        </div>
      )}
      <div className="benchmark-grid">
        {metricIds.map((id) => {
          const counts = group.metrics[id],
            [low, high] = ranges[id],
            a = assess(id, counts);
          return (
            <button
              key={id}
              className={"benchmark-card" + (expanded === id ? " chosen" : "")}
              aria-expanded={expanded === id}
              onClick={() => select(group.key, expanded === id ? null : id)}
            >
              <strong>{t("bench." + id)}</strong>
              <span className="bench-rate">
                <Rate counts={counts} />
              </span>
              <span className="bench-scale" aria-hidden="true">
                <i style={{ left: low + "%", width: high - low + "%" }} />
                {a.value !== null && <b style={{ left: a.value + "%" }} />}
              </span>
              <span>{t("review.band", { low, high })}</span>
              <small>
                {number(counts[0])} / {number(counts[1])}{" "}
                {t("review.opportunities")}
              </small>
              <small>
                {!broad
                  ? t("review.contextOnly")
                  : a.value === null
                    ? t("review.noOpportunities")
                    : a.gap === 0
                      ? t("review.inside")
                      : t(a.gap > 0 ? "review.above" : "review.below", {
                          n: decimal(Math.abs(a.gap), 1),
                        })}
                {broad && counts[1] > 0 && counts[1] < 100
                  ? " · " + t("review.small")
                  : ""}
              </small>
            </button>
          );
        })}
      </div>
      {expanded && (
        <section className="benchmark-detail">
          <h3>{t("bench." + expanded)}</h3>
          <p>{t("bench.tip." + expanded)}</p>
          <p>
            {t(
              expanded === "pfr" || expanded === "vpip"
                ? "review.preDenom"
                : "review.actionDenom",
            )}
          </p>
          <div className="inline-controls">
            <button
              className="button"
              onClick={() => handList(expanded, "numerator")}
            >
              {t("review.hits")}
            </button>
            <button
              className="button"
              onClick={() => handList(expanded, "benchmark_missed")}
            >
              {t("review.missed")}
            </button>
            <button
              className="text-button"
              onClick={() => handList(expanded, "benchmark_opportunities")}
            >
              {t("review.allOpp")}
            </button>
          </div>
          <h4>{t("review.bySeat")}</h4>
          <div className="bench-seats">
            {group.positions.map((p) => (
              <button
                className="button subtle"
                key={p.position}
                onClick={() =>
                  handList(expanded, "benchmark_opportunities", p.position)
                }
              >
                {p.position} <Rate counts={p.metrics[expanded]} /> ·{" "}
                {p.metrics[expanded][0]}/{p.metrics[expanded][1]}
              </button>
            ))}
          </div>
          {expanded === "three_bet" &&
            (!filter.position || filter.position === "BB") && (
              <>
                <button
                  className="text-button"
                  aria-expanded={spot}
                  onClick={() => setSpot(!spot)}
                >
                  {t("review.blindSpot")}
                </button>
                {spot && (
                  <BlindSpot
                    group={group}
                    filter={filter}
                    revision={revision}
                    open={open}
                  />
                )}
              </>
            )}
        </section>
      )}
      <details className="benchmark-source">
        <summary>{t("review.source")}</summary>
        <p>
          {t("review.scope")}{" "}
          <a
            href="https://flopmortem.com/stats"
            target="_blank"
            rel="noreferrer"
          >
            Flopmortem
          </a>{" "}
          ·{" "}
          <a
            href="https://www.pokalab.com/learn/what-is-a-good-vpip"
            target="_blank"
            rel="noreferrer"
          >
            Pokalab (VPIP)
          </a>
        </p>
        <p>{t("review.selectionRule")}</p>
      </details>
    </section>
  );
}
