import { useTranslation } from "react-i18next";
import { useState } from "react";
import { ArrowUpRight, Info } from "@phosphor-icons/react";
import type { Filter, Stat } from "../types";
import { decimal, number } from "../format";

const groups = [
  {
    id: "preflop",
    title: "翻前",
    subtitle: "入池與加注",
    stats: ["vpip", "pfr", "rfi", "three_bet", "fold_three_bet"],
  },
  {
    id: "blinds",
    title: "盲位防守",
    subtitle: "面對 Open Raise",
    stats: ["blind_fold", "blind_call", "blind_raise"],
  },
  {
    id: "postflop",
    title: "翻後",
    subtitle: "Flop 持續下注",
    stats: ["cbet", "fold_cbet"],
  },
  {
    id: "showdown",
    title: "攤牌與贏池",
    subtitle: "到達與勝出頻率",
    stats: ["wtsd", "wsd", "wwsf"],
  },
];

export function CoreStats({
  stats,
  drill,
}: {
  stats: Stat[];
  drill: (filter: Partial<Filter>) => void;
}) {
  const { t } = useTranslation();
  const [showDefinitions, setShowDefinitions] = useState(false);
  const byId = new Map(stats.map((stat) => [stat.id, stat]));
  const visibleGroups = groups.map((group) => ({
    ...group,
    stats: group.stats.flatMap((id) => {
      const stat = byId.get(id);
      return stat ? [stat] : [];
    }),
  }));
  const count = visibleGroups.reduce(
    (sum, group) => sum + group.stats.length,
    0,
  );

  return (
    <section className="core-stats" aria-labelledby="core-stats-heading">
      <div className="core-stats-heading">
        <div>
          <h2 id="core-stats-heading">
            {t("核心統計 ")}
            <span className="core-stats-count mono">{count}</span>
          </h2>
          <p>{t("點擊比率查看機會手牌；分子、分母亦可分別查閱。")}</p>
        </div>
        <button
          className="text-button core-definitions-toggle"
          aria-pressed={showDefinitions}
          onClick={() => setShowDefinitions(!showDefinitions)}
        >
          <Info size={15} />
          {showDefinitions ? t("收起定義") : t("顯示定義")}
        </button>
      </div>
      <div className="core-stat-groups">
        {visibleGroups
          .filter((group) => group.stats.length > 0)
          .map((group) => (
            <section
              className={`core-stat-group core-stat-group-${group.id}`}
              key={group.id}
              aria-labelledby={`stat-group-${group.id}`}
            >
              <div className="core-stat-group-heading">
                <h3 id={`stat-group-${group.id}`}>{t(group.title)}</h3>
                <span>{t(group.subtitle)}</span>
              </div>
              <div className="core-stat-metrics">
                {group.stats.map((stat) => (
                  <article
                    className="core-stat"
                    key={stat.id}
                    aria-label={stat.label}
                  >
                    <button
                      className="core-stat-value"
                      title={t(stat.definition)}
                      aria-label={t("{{v0}} {{v1}}{{v2}}，查看機會手牌", {
                        v0: stat.label,
                        v1: decimal(stat.value),
                        v2: stat.value === null ? "" : "%",
                      })}
                      onClick={() =>
                        drill({ stat: stat.id, stat_mode: "opportunities" })
                      }
                    >
                      <span className="core-stat-label">
                        {stat.label} <ArrowUpRight size={13} />
                      </span>
                      <strong className="mono">
                        {decimal(stat.value)}
                        {stat.value !== null && <small>%</small>}
                      </strong>
                    </button>
                    <div className="core-stat-sample">
                      <button
                        aria-label={t("{{v0}}：查看 {{v1}} 手符合條件牌局", {
                          count: stat.numerator,
                          v0: stat.label,
                          v1: number(stat.numerator),
                        })}
                        title={t("查看分子：符合條件嘅手牌")}
                        onClick={() =>
                          drill({ stat: stat.id, stat_mode: "numerator" })
                        }
                      >
                        {t("符合 ")}
                        <span className="mono">{number(stat.numerator)}</span>
                      </button>
                      <span aria-hidden="true">/</span>
                      <button
                        aria-label={t("{{v0}}：查看 {{v1}} 手機會牌局", {
                          count: stat.opportunities,
                          v0: stat.label,
                          v1: number(stat.opportunities),
                        })}
                        title={t("查看分母：所有同類機會")}
                        onClick={() =>
                          drill({ stat: stat.id, stat_mode: "opportunities" })
                        }
                      >
                        {t("機會 ")}
                        <span className="mono">
                          {number(stat.opportunities)}
                        </span>
                      </button>
                    </div>
                    {showDefinitions && (
                      <p className="core-stat-definition">
                        {t(stat.definition)}
                      </p>
                    )}
                  </article>
                ))}
              </div>
            </section>
          ))}
      </div>
      <p className="core-stats-footnote">
        {t(
          "各項機會分母獨立計算；零機會顯示「—」。頻率與樣本只供檢討，唔係策略評分。",
        )}
      </p>
    </section>
  );
}
