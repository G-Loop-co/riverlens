import { useTranslation } from "react-i18next";
import { useMemo, useState } from "react";
import {
  ArrowDownRight,
  ArrowRight,
  ArrowUpRight,
  CaretRight,
  DownloadSimple,
  Info,
  PushPin,
  X,
} from "@phosphor-icons/react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { api, chooseOutput, desktop, errorText, useRpc } from "../api";
import type { Filter, Report } from "../types";
import { decimal, EV_REASONS, money, number } from "../format";
import { Empty, ErrorBanner, Signed, Skeleton } from "./UI";
import { CoreStats } from "./CoreStats";
import { readPreference, writePreference } from "../preferences";

export function Overview({
  filter,
  revision,
  drill,
  onImport,
  compare,
  setCompare,
}: {
  filter: Filter;
  revision: number;
  drill: (f: Partial<Filter>) => void;
  onImport: () => void;
  compare: Filter | null;
  setCompare: (f: Filter | null) => void;
}) {
  const { t } = useTranslation();
  const [group, setGroupState] = useState(() => {
    const value = readPreference("riverlens-report-group", "date");
    return ["date", "position", "stakes", "session", "game"].includes(value)
      ? value
      : "date";
  });
  const [exportStatus, setExportStatus] = useState("");
  const [exporting, setExporting] = useState(false);
  async function exportReport() {
    if (exporting) return;
    setExporting(true);
    setExportStatus("");
    try {
      const path = desktop
        ? await chooseOutput(
            "csv",
            `riverlens-report-${group}-${Date.now()}.csv`,
          )
        : prompt(t("CSV 報表輸出完整路徑"));
      if (!path) return;
      await api({
        op: "export",
        format: `report:${group}`,
        path,
        filter,
        selected: [],
      });
      setExportStatus(t("CSV 報表已匯出"));
    } catch (e) {
      setExportStatus(errorText(e));
    } finally {
      setExporting(false);
    }
  }
  function setGroup(value: string) {
    setGroupState(value);
    writePreference("riverlens-report-group", value);
  }
  const [lines, setLines] = useState({
    net: true,
    sd: false,
    nsd: false,
    adjusted: true,
  });
  const data = useRpc<Report>({ op: "overview", filter, group }, revision);
  const graph = useRpc<Report>(
    group === "date" ? null : { op: "overview", filter, group: "date" },
    revision,
  );
  const pinned = useRpc<Report>(
    compare ? { op: "overview", filter: compare, group } : null,
    revision,
  );
  const report = data.data;
  const chart = useMemo(() => {
    const rows = (group === "date" ? report : graph.data)?.groups || [];
    const total = { net: 0, sd: 0, nsd: 0, adjusted: 0 };
    return rows.map((row) => {
      for (const k of Object.keys(total) as (keyof typeof total)[])
        total[k] += Number(row[k]);
      return { date: row.key.slice(5), fullDate: row.key, ...total };
    });
  }, [report, graph.data, group]);
  if (data.error) return <ErrorBanner>{data.error}</ErrorBanner>;
  if (!report) return <Skeleton rows={6} />;
  if (!report.hands)
    return (
      <>
        <Empty
          title={t("由第一手牌開始")}
          description={t(
            "匯入 PokerCraft 匯出嘅 TXT 或 ZIP，建立自己的牌局資料庫。亦可以清除篩選查看其他手牌。",
          )}
          action={
            <button className="button primary" onClick={onImport}>
              {t("匯入牌譜")}
              <ArrowRight size={17} />
            </button>
          }
        />
        {!!report.coverage.quarantined && (
          <div className="empty-issues">
            <p>
              {t("目前條件下有 {{v0}} 手已隔離。可開啟手牌查看問題與原文。", {
                v0: report.coverage.quarantined,
              })}
            </p>
            <button
              className="button subtle"
              onClick={() => drill({ status: "quarantined" })}
            >
              {t("查看已隔離手牌")}
            </button>
          </div>
        )}
      </>
    );
  const valid = report.coverage.valid || 0,
    quarantine = report.coverage.quarantined || 0;
  const currencies = report.currency;
  function groupDrill(key: string) {
    drill(group === "date" ? { source_date: key } : { [group]: key });
  }

  return (
    <div className="overview-view">
      {(report.groups_truncated || graph.data?.groups_truncated) && (
        <ErrorBanner>
          {t(
            "分組超過 2,000 組，表格／曲線只顯示首 2,000 組；總數仍包含全部有效牌局。請縮窄日期查看完整明細。",
          )}
        </ErrorBanner>
      )}
      <div className="metrics-strip">
        <div className="metric main-metric">
          <span>
            {t("牌局盈虧 ")}
            <span className="metric-unit">{currencies}</span>
          </span>
          <strong className={Number(report.net) < 0 ? "negative" : "positive"}>
            <span className="currency-prefix">$</span>
            {money(report.net, true)}
          </strong>
          <small>
            {Number(report.net) < 0 ? (
              <ArrowDownRight size={14} />
            ) : (
              <ArrowUpRight size={14} />
            )}
            {t("已核對牌局結果")}
          </small>
        </div>
        <div className="metric">
          <span>{t("Win rate")}</span>
          <strong>
            <Signed value={report.bb100} digits={2} />
            <span className="value-unit">{t("bb/100")}</span>
          </strong>
          <small>{t("每手按所屬 BB 換算")}</small>
        </div>
        <div className="metric">
          <span>{t("已分析手數")}</span>
          <strong>
            {number(report.hands)}
            <span className="value-unit">{t("hands")}</span>
          </strong>
          <small>
            <span className="mini-dot" />
            {quarantine
              ? t("{{v0}} 手資料異常已排除", {
                  count: quarantine,
                  v0: number(quarantine),
                })
              : t("每手均可追溯原文")}
          </small>
        </div>
        <div className="metric">
          <span>
            {t("已套用 All-in 調整 ")}
            <Info size={13} />
          </span>
          <strong>
            <span className="currency-prefix">$</span>
            {money(report.adjusted, true)}
          </strong>
          <small>
            {t("{{v0}} 已計算 · {{v1}} 待計算 · {{v2}} 排除", {
              v0: report.ev.complete,
              v1: report.ev.pending,
              v2: report.ev.excluded,
            })}
          </small>
        </div>
      </div>
      {compare && (
        <div className="comparison-bar">
          <PushPin size={17} weight="fill" />
          <div>
            <strong>{t("已固定比較報表")}</strong>
            <span>
              {pinned.data
                ? t("{{v0}} 手 · ${{v1}} · {{v2}} bb/100", {
                    count: pinned.data.hands,
                    v0: number(pinned.data.hands),
                    v1: money(pinned.data.net, true),
                    v2: decimal(pinned.data.bb100, 2),
                  })
                : pinned.error
                  ? t("載入失敗")
                  : t("載入中…")}
            </span>
          </div>
          <small>{t("更改上方篩選，對照兩組自身樣本。")}</small>
          <button
            className="icon-button"
            aria-label={t("取消固定比較")}
            onClick={() => setCompare(null)}
          >
            <X size={16} />
          </button>
        </div>
      )}
      <div className="analysis-grid">
        <section className="panel profit-panel">
          <div className="panel-heading">
            <div>
              <span className="section-kicker">{t("PERFORMANCE")}</span>
              <h2>{t("盈利走勢")}</h2>
            </div>
            <div className="inline-controls">
              <span className="subtle-badge">{t("USD")}</span>
              <button
                className="icon-button"
                title={t("固定目前報表作比較")}
                aria-label={t("固定比較報表")}
                onClick={() => setCompare({ ...filter })}
              >
                <PushPin size={17} />
              </button>
            </div>
          </div>
          <div className="chart-legends">
            {(
              [
                ["net", t("實際盈虧"), "#79bb9c"],
                ["adjusted", t("已套用 All-in 調整"), "#d2b57a"],
                ["sd", "Showdown", "#9aaebb"],
                ["nsd", "Non-showdown", "#cb8d83"],
              ] as const
            ).map(([key, label, color]) => (
              <button
                key={key}
                className={!lines[key] ? "muted" : ""}
                aria-pressed={lines[key]}
                onClick={() => setLines((s) => ({ ...s, [key]: !s[key] }))}
              >
                <i style={{ background: color }} />
                {t(label)}
              </button>
            ))}
          </div>
          <div className="profit-chart">
            {graph.error ? (
              <ErrorBanner>{t("目前無法載入曲線")}</ErrorBanner>
            ) : (
              <ResponsiveContainer width="100%" height="100%">
                <LineChart
                  data={chart}
                  margin={{ left: -15, right: 16, top: 10, bottom: 6 }}
                >
                  <CartesianGrid
                    stroke="#2b3430"
                    vertical={false}
                    strokeDasharray="3 5"
                  />
                  <XAxis
                    dataKey="date"
                    tick={{ fill: "#819087", fontSize: 10 }}
                    tickLine={false}
                    axisLine={false}
                    minTickGap={32}
                  />
                  <YAxis
                    tick={{
                      fill: "#819087",
                      fontSize: 10,
                      fontFamily: "Geist Mono",
                    }}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(v) => `$${v}`}
                  />
                  <ReferenceLine y={0} stroke="#4b5951" />
                  <Tooltip
                    contentStyle={{
                      background: "#202923",
                      border: "1px solid #425247",
                      borderRadius: 8,
                      fontSize: 12,
                    }}
                    labelStyle={{ color: "#c6d0ca" }}
                    formatter={(value, name) => [
                      `$${money(Number(value))}`,
                      (
                        {
                          net: t("實際盈虧"),
                          sd: t("Showdown"),
                          nsd: t("Non-showdown"),
                          adjusted: t("已套用 All-in 調整"),
                        } as Record<string, string>
                      )[String(name)],
                    ]}
                  />
                  {lines.net && (
                    <Line
                      dataKey="net"
                      type="linear"
                      stroke="#79bb9c"
                      strokeWidth={2.4}
                      dot={false}
                      isAnimationActive={false}
                    />
                  )}
                  {lines.adjusted && (
                    <Line
                      dataKey="adjusted"
                      type="linear"
                      stroke="#d2b57a"
                      strokeWidth={1.7}
                      strokeDasharray="5 4"
                      dot={false}
                      isAnimationActive={false}
                    />
                  )}
                  {lines.sd && (
                    <Line
                      dataKey="sd"
                      stroke="#9aaebb"
                      strokeWidth={1.5}
                      dot={false}
                      isAnimationActive={false}
                    />
                  )}
                  {lines.nsd && (
                    <Line
                      dataKey="nsd"
                      stroke="#cb8d83"
                      strokeWidth={1.5}
                      dot={false}
                      isAnimationActive={false}
                    />
                  )}
                </LineChart>
              </ResponsiveContainer>
            )}
          </div>
          <div className="chart-note">
            <Info size={14} />
            <span>
              {t(
                "調整曲線只套用已完成嘅適用牌局；未涵蓋 {{v0}} 手，待計算 {{v1}} 手。",
                { v0: report.ev.excluded, v1: report.ev.pending },
              )}
            </span>
          </div>
        </section>
        <aside className="focus-panel" aria-label={t("下一次複盤")}>
          <span className="section-kicker">{t("STUDY FOCUS")}</span>
          <h2>{t("下一次複盤")}</h2>
          <p>
            {t("由具體情境開始，")}
            <br />
            {t("每個數字都連到你的手牌。")}
          </p>
          <button onClick={() => drill({ cue: "call_three_bet" })}>
            <span className="focus-index">01</span>
            <div>
              <strong>{t("Call 3-bet pots")}</strong>
              <small>{t("位置與有效籌碼")}</small>
            </div>
            <CaretRight size={16} />
          </button>
          <button onClick={() => drill({ cue: "river_call" })}>
            <span className="focus-index">02</span>
            <div>
              <strong>{t("River Calls")}</strong>
              <small>{t("同時回看贏牌與輸牌")}</small>
            </div>
            <CaretRight size={16} />
          </button>
          <button onClick={() => drill({ reviewed: false })}>
            <span className="focus-index">03</span>
            <div>
              <strong>{t("未完成複盤")}</strong>
              <small>{t("留下下一次要睇嘅筆記")}</small>
            </div>
            <CaretRight size={16} />
          </button>
          <div className="focus-bottom">
            <span className="mini-dot" />
            {t("提示係檢討入口，唔係策略評分。")}
          </div>
        </aside>
      </div>
      <CoreStats stats={report.stats} drill={drill} />
      <section className="panel breakdown-panel">
        <div
          style={{
            padding: "12px 22px 0",
            display: "flex",
            justifyContent: "flex-end",
            gap: 12,
            alignItems: "center",
          }}
        >
          <span className="muted" role="status">
            {exportStatus}
          </span>
          <button
            className="text-button"
            disabled={exporting}
            onClick={() => void exportReport()}
          >
            <DownloadSimple size={15} />
            {t("匯出此報表 CSV")}
          </button>
        </div>
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("BREAKDOWN")}</span>
            <h2>{t("細看你的牌局")}</h2>
          </div>
          <div className="segmented">
            {[
              ["date", t("日期")],
              ["position", t("位置")],
              ["stakes", t("盲注")],
              ["session", "Session"],
              ["game", t("牌局")],
            ].map(([key, label]) => (
              <button
                className={group === key ? "active" : ""}
                aria-pressed={group === key}
                onClick={() => setGroup(key)}
                key={key}
              >
                {t(label)}
              </button>
            ))}
          </div>
        </div>
        <div className="table-scroll">
          <table className="data-table" aria-label={t("分類報表")}>
            <thead>
              <tr>
                <th>{group === "date" ? t("日期 · 來源時區") : t("分類")}</th>
                <th>{t("手數")}</th>
                <th>{t("盈虧 USD")}</th>
                <th>{t("Net bb")}</th>
                <th>{t("bb/100")}</th>
                <th>
                  <span className="sr-only">{t("查看明細")}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {report.groups.map((row) => (
                <tr
                  key={row.key}
                  tabIndex={0}
                  onClick={() => groupDrill(row.key)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      groupDrill(row.key);
                    }
                  }}
                >
                  <td className="group-key">
                    {group === "session" ? row.key.split("/").pop() : row.key}
                  </td>
                  <td className="mono muted">{number(row.hands)}</td>
                  <td>
                    <Signed value={Number(row.net)} />
                  </td>
                  <td>
                    <Signed value={row.net_bb} digits={1} />
                  </td>
                  <td>
                    <Signed value={row.bb100} />
                  </td>
                  <td>
                    <ArrowUpRight size={14} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="table-foot">
          <span>
            {t("{{v0}} 手有效樣本 · {{v1}} 手排除", {
              v0: number(valid),
              v1: number(quarantine),
            })}
          </span>
          <span>{t("全部資料保留於本機")}</span>
        </div>
      </section>
      <details className="methodology">
        <summary>{t("統計定義與 All-in 適用範圍")}</summary>
        <div className="definition-grid">
          {report.stats.map((s) => (
            <div key={s.id}>
              <strong>{s.label}</strong>
              <p>{t(s.definition)}</p>
              <button
                className="text-button"
                onClick={() => drill({ stat: s.id, stat_mode: "numerator" })}
              >
                {t("行動 {{v0}}", { v0: s.numerator })}
              </button>{" "}
              /{" "}
              <button
                className="text-button"
                onClick={() =>
                  drill({ stat: s.id, stat_mode: "opportunities" })
                }
              >
                {t("機會 {{v0}}", { v0: s.opportunities })}
              </button>
            </div>
          ))}
        </div>
        <p>
          {t(
            "EV 排除：{{v0}}。All-in equity adjustment 並非 GTO decision EV。",
            {
              v0:
                Object.entries(report.ev.reasons)
                  .map(([r, n]) =>
                    t("{{v0}} {{v1}} 手", {
                      count: n,
                      v0: t(EV_REASONS[r] || r),
                      v1: n,
                    }),
                  )
                  .join("；") || t("目前沒有排除項目"),
            },
          )}
        </p>
      </details>
    </div>
  );
}
