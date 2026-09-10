import { useTranslation } from "react-i18next";
import { useMemo } from "react";
import { ArrowUpRight, Info } from "@phosphor-icons/react";
import { useRpc } from "../api";
import type { Filter, MatrixCell } from "../types";
import { decimal, money, number, STAT_LABELS } from "../format";
import { ErrorBanner, Signed, Skeleton } from "./UI";

const ranks = "AKQJT98765432".split("");
export function RangeView({
  filter,
  revision,
  drill,
  view,
  setView,
}: {
  filter: Filter;
  revision: number;
  drill: (f: Partial<Filter>) => void;
  view: { mode: string; stat: string; hover: string };
  setView: (view: { mode: string; stat: string; hover: string }) => void;
}) {
  const { t } = useTranslation();
  const { mode, stat, hover } = view;
  const setMode = (mode: string) => setView({ ...view, mode });
  const setStat = (stat: string) => setView({ ...view, stat });
  const setHover = (hover: string) => setView({ ...view, hover });
  const openHand = (hand: string) =>
    drill({
      hand_class: hand,
      ...(mode === "frequency" ? { stat, stat_mode: "opportunities" } : {}),
    });
  const result = useRpc<MatrixCell[]>({ op: "matrix", filter, stat }, revision);
  const map = useMemo(
    () => new Map(result.data?.map((r) => [r.hand, r])),
    [result.data],
  );
  const max = Math.max(1, ...(result.data || []).map((r) => r.hands));
  const current = map.get(hover);
  if (result.error) return <ErrorBanner>{result.error}</ErrorBanner>;
  return (
    <div className="range-layout">
      <section className="panel range-panel">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("OBSERVED RANGE")}</span>
            <h2>{t("13 × 13 起手牌矩陣")}</h2>
          </div>
          <span className="subtle-badge">{t("Hero only")}</span>
        </div>
        <div className="range-controls">
          <div className="segmented">
            {[
              ["hands", t("手數")],
              ["net", "Net bb"],
              ["bb100", "bb/100"],
              ["frequency", t("行動頻率")],
            ].map(([k, l]) => (
              <button
                className={mode === k ? "active" : ""}
                aria-pressed={mode === k}
                onClick={() => setMode(k)}
                key={k}
              >
                {t(l)}
              </button>
            ))}
          </div>
          {mode === "frequency" && (
            <select
              aria-label={t("矩陣統計")}
              value={stat}
              onChange={(e) => setStat(e.target.value)}
            >
              {[
                ["rfi", "RFI"],
                ["vpip", "VPIP"],
                ["pfr", "PFR"],
                ["three_bet", "3-bet"],
                ["cbet", "Flop C-bet"],
              ].map(([k, l]) => (
                <option value={k} key={k}>
                  {t(l)}
                </option>
              ))}
            </select>
          )}
        </div>
        {!result.data ? (
          <Skeleton rows={8} />
        ) : (
          <div className="range-matrix">
            {ranks.flatMap((r, row) =>
              ranks.map((c, col) => {
                const hand =
                  row === col
                    ? `${r}${c}`
                    : row < col
                      ? `${r}${c}s`
                      : `${c}${r}o`;
                const cell = map.get(hand);
                const frequency =
                  cell && cell.opportunities
                    ? (cell.numerator / cell.opportunities) * 100
                    : null;
                const value = !cell
                  ? "—"
                  : mode === "hands"
                    ? String(cell.hands)
                    : mode === "net"
                      ? decimal(cell.net_bb, 1)
                      : mode === "bb100"
                        ? decimal(cell.bb100, 0)
                        : frequency === null
                          ? "—"
                          : `${decimal(frequency, 0)}%`;
                const intensity = !cell
                  ? 0
                  : mode === "hands"
                    ? cell.hands / max
                    : mode === "frequency"
                      ? (frequency || 0) / 100
                      : Math.min(
                          Math.abs(mode === "net" ? cell.net_bb : cell.bb100) /
                            100,
                          1,
                        );
                const negative =
                  cell && ["net", "bb100"].includes(mode) && cell.net_bb < 0;
                const bg = cell
                  ? negative
                    ? `rgba(190,118,105,${0.09 + intensity * 0.4})`
                    : `rgba(114,173,141,${0.07 + intensity * 0.48})`
                  : undefined;
                return (
                  <button
                    key={hand}
                    className={`range-cell ${!cell ? "no-sample" : ""} ${hover === hand ? "focused" : ""}`}
                    style={{ background: bg }}
                    onMouseEnter={() => setHover(hand)}
                    onFocus={() => setHover(hand)}
                    onClick={() => openHand(hand)}
                    title={t("{{v0}} · {{v1}}", {
                      v0: hand,
                      v1: cell
                        ? t("{{v0}} 手 / {{v1}} 次行動 / {{v2}} 次機會", {
                            v0: cell.hands,
                            v1: cell.numerator,
                            v2: cell.opportunities,
                          })
                        : t("沒有樣本"),
                    })}
                  >
                    <strong>{hand}</strong>
                    <small className="mono">{value}</small>
                  </button>
                );
              }),
            )}
          </div>
        )}
        <div className="matrix-legend">
          <span>
            {t(["net", "bb100"].includes(mode) ? "較低／虧損" : "較少")}
          </span>
          <i />
          <i />
          <i />
          <i />
          <span>
            {t(["net", "bb100"].includes(mode) ? "較高／盈利" : "較多")}
          </span>
          <span className="filter-spacer" />
          <span>
            {t(
              mode === "frequency"
                ? "灰色 = 無樣本；「—」= 無機會"
                : "灰色 = 無樣本",
            )}
          </span>
        </div>
      </section>
      <aside className="range-detail" aria-label={t("HAND DETAILS")}>
        <span className="section-kicker">{t("HAND DETAILS")}</span>
        <h2>{hover}</h2>
        <span className="muted">
          {hover.length === 2
            ? t("Pocket pair · 6 combos")
            : hover.endsWith("s")
              ? t("Suited · 4 combos")
              : t("Offsuit · 12 combos")}
        </span>
        <div className="detail-metric">
          <span>{t("歷史手數")}</span>
          <strong className="mono">{number(current?.hands || 0)}</strong>
        </div>
        <div className="detail-metric">
          <span>{t("牌局盈虧 USD")}</span>
          <strong className="mono">
            {current ? `$${money(current.net, true)}` : "—"}
          </strong>
        </div>
        <div className="detail-metric">
          <span>{t("Win rate")}</span>
          <strong>
            {current ? (
              <Signed value={current.bb100} suffix=" bb/100" digits={1} />
            ) : (
              "—"
            )}
          </strong>
        </div>
        <div className="detail-metric">
          <span>
            {t("{{v0}} 行動／機會", { v0: STAT_LABELS[stat] || stat })}
          </span>
          <strong className="mono">
            {current ? `${current.numerator} / ${current.opportunities}` : "—"}
          </strong>
        </div>
        <button className="button primary" onClick={() => openHand(hover)}>
          {t("查看 {{v0}} 手牌", { v0: hover })}
          <ArrowUpRight size={17} />
        </button>
        <p className="info-copy">
          <Info size={16} />
          {t(
            "此矩陣描述你嘅歷史樣本，並非建議 range。理論 combos 不代表實際觀察次數；無樣本不等於 Fold 100%。",
          )}
        </p>
      </aside>
    </div>
  );
}
