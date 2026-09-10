import { useTranslation } from "react-i18next";
import { useEffect, useRef, useState } from "react";
import {
  ArrowRight,
  ArrowUpRight,
  Check,
  DownloadSimple,
  MagnifyingGlass,
} from "@phosphor-icons/react";
import { api, chooseOutput, desktop, errorText } from "../api";
import type { Cursor, Filter, HandPage, HandRow } from "../types";
import { number, time } from "../format";
import { Empty, ErrorBanner, Signed, Skeleton } from "./UI";
import { readPreference, writePreference } from "../preferences";

export function HandsView({
  filter,
  revision,
  open,
  notify,
}: {
  filter: Filter;
  revision: number;
  open: (id: number) => void;
  notify: (v: string) => void;
}) {
  const { t } = useTranslation();
  const [storedRows, setRows] = useState<HandRow[]>([]);
  const [rowsKey, setRowsKey] = useState("");
  const [cursor, setCursor] = useState<Cursor | null>(null);
  const [sort, setSort] = useState(() => {
    const value = readPreference("riverlens-hand-sort", "recent");
    return ["recent", "loss", "win"].includes(value) ? value : "recent";
  });
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [exporting, setExporting] = useState(false);
  const key = JSON.stringify({
    ...filter,
    search: search || filter.search || undefined,
  });
  const queryKey = `${key}/${sort}/${revision}`;
  const currentQuery = useRef(queryKey);
  currentQuery.current = queryKey;
  const morePending = useRef(false);
  const rows = rowsKey === queryKey ? storedRows : [];
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    setSelected(new Set());
    api<HandPage>({ op: "hands", filter: JSON.parse(key), sort, limit: 60 })
      .then((page) => {
        if (active) {
          setRows(page.rows);
          setRowsKey(queryKey);
          setCursor(page.next_cursor);
        }
      })
      .catch((e) => {
        if (active) {
          setRows([]);
          setCursor(null);
          setRowsKey(queryKey);
          setError(errorText(e));
        }
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [key, sort, revision]);
  useEffect(
    () => () => {
      currentQuery.current = "unmounted";
    },
    [],
  );
  async function more() {
    if (morePending.current || !cursor || loading || rowsKey !== queryKey)
      return;
    morePending.current = true;
    const expected = queryKey;
    setLoading(true);
    try {
      const page = await api<HandPage>({
        op: "hands",
        filter: JSON.parse(key),
        sort,
        cursor,
        limit: 60,
      });
      if (currentQuery.current === expected) {
        setRows((r) => [...r, ...page.rows]);
        setCursor(page.next_cursor);
      }
    } catch (e) {
      if (currentQuery.current === expected) setError(errorText(e));
    } finally {
      morePending.current = false;
      if (currentQuery.current === expected) setLoading(false);
    }
  }
  async function exportFile(format: string) {
    if (exporting || loading || !rows.length) return;
    setError("");
    setExporting(true);
    const exportFilter = JSON.parse(key);
    const exportSelected = [...selected];
    const ext = format === "csv" ? "csv" : "txt";
    try {
      const path = desktop
        ? await chooseOutput(ext, `riverlens-${Date.now()}.${ext}`)
        : window.prompt(t("輸出本機檔案完整路徑（不覆寫現有檔案）"));
      if (!path) return;
      const result = await api<{ hands: number }>({
        op: "export",
        filter: exportFilter,
        path,
        format,
        selected: exportSelected,
      });
      notify(
        t("已匯出 {{v0}} 手至 {{v1}}", {
          count: result.hands,
          v0: number(result.hands),
          v1: path,
        }),
      );
    } catch (e) {
      setError(errorText(e));
    } finally {
      setExporting(false);
    }
  }
  function toggle(id: number) {
    if (!selected.has(id) && selected.size >= 1000) {
      setError(
        t("每次最多選取 1,000 手牌。取消勾選可匯出全部符合條件的手牌。"),
      );
      return;
    }
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }
  return (
    <section className="panel hands-panel">
      <div className="panel-heading">
        <div>
          <span className="section-kicker">{t("HAND LIBRARY")}</span>
          <h2>{t("每一手都有跡可尋")}</h2>
        </div>
        <div className="inline-controls">
          <button
            className="button subtle"
            disabled={exporting || loading || !rows.length}
            onClick={() => void exportFile("csv")}
          >
            <DownloadSimple size={16} />
            {t("CSV")}
          </button>
          <button
            className="button subtle"
            disabled={exporting || loading || !rows.length}
            onClick={() => void exportFile("hh")}
          >
            {t("匯出 HH{{v0}}", {
              v0: selected.size ? ` (${selected.size})` : "",
            })}
          </button>
        </div>
      </div>
      <div className="table-toolbar">
        <label className="search">
          <MagnifyingGlass size={17} />
          <input
            aria-label={t("搜尋 Hand ID")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t("搜尋 Hand ID")}
          />
        </label>
        <span className="muted">
          {t("已載入 {{v0}} 手", {
            count: rows.length,
            v0: number(rows.length),
          })}
        </span>
        <select
          aria-label={t("手牌排序")}
          value={sort}
          onChange={(e) => {
            setSort(e.target.value);
            writePreference("riverlens-hand-sort", e.target.value);
          }}
        >
          <option value="recent">{t("最近牌局")}</option>
          <option value="loss">{t("最大虧損")}</option>
          <option value="win">{t("最大盈利")}</option>
        </select>
      </div>
      {error && <ErrorBanner>{error}</ErrorBanner>}
      {(loading || rowsKey !== queryKey) && !rows.length ? (
        <Skeleton />
      ) : !rows.length ? (
        <Empty
          title={t("未有符合條件嘅手牌")}
          description={t("調整篩選，或先匯入你嘅牌譜。")}
        />
      ) : (
        <div className="table-scroll">
          <table
            className="data-table hands-table"
            aria-label={t("手牌資料庫")}
          >
            <thead>
              <tr>
                <th>
                  <input
                    aria-label={t("選取目前已載入手牌")}
                    type="checkbox"
                    checked={rows.length > 0 && selected.size === rows.length}
                    disabled={rows.length > 1000}
                    onChange={(e) =>
                      setSelected(
                        e.target.checked
                          ? new Set(rows.map((r) => r.id))
                          : new Set(),
                      )
                    }
                  />
                </th>
                <th>{t("起手牌")}</th>
                <th>{t("時間 · HKT")}</th>
                <th>{t("位置")}</th>
                <th>{t("底池")}</th>
                <th>{t("盲注")}</th>
                <th>{t("盈虧 USD")}</th>
                <th>{t("Net bb")}</th>
                <th>
                  <span className="sr-only">{t("開啟手牌")}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  key={row.id}
                  tabIndex={0}
                  onClick={() => open(row.id)}
                  onKeyDown={(e) => {
                    if (
                      e.target === e.currentTarget &&
                      (e.key === "Enter" || e.key === " ")
                    ) {
                      e.preventDefault();
                      open(row.id);
                    }
                  }}
                >
                  <td onClick={(e) => e.stopPropagation()}>
                    <input
                      aria-label={t("選取 {{v0}}", { v0: row.hand_id })}
                      type="checkbox"
                      checked={selected.has(row.id)}
                      onChange={() => toggle(row.id)}
                    />
                  </td>
                  <td>
                    <span
                      className={`hand-label ${row.hand_class.length === 2 ? "pair" : row.hand_class.endsWith("s") ? "suited" : ""}`}
                    >
                      {row.hand_class || "??"}
                    </span>
                  </td>
                  <td>
                    <strong className="hand-time">{time(row.played_at)}</strong>
                    <small className="hand-id mono">#{row.hand_id}</small>
                  </td>
                  <td>
                    <span className="position-badge">{row.position}</span>
                  </td>
                  <td>
                    <span className="subtle-badge">{t(row.pot_type)}</span>
                    {row.game === "Rush" && (
                      <small className="rush-label">{t("RUSH")}</small>
                    )}
                  </td>
                  <td className="mono muted">{row.stakes}</td>
                  <td>
                    <Signed value={Number(row.net)} />
                  </td>
                  <td>
                    <Signed value={row.net_bb} digits={1} />
                  </td>
                  <td>
                    {row.reviewed ? (
                      <Check size={17} className="positive" />
                    ) : (
                      <ArrowUpRight size={15} />
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="table-foot">
        <span>
          {selected.size
            ? t("已選 {{v0}} 手；匯出只包含所選手牌", {
                count: selected.size,
                v0: selected.size,
              })
            : t("點擊手牌開啟 Replayer")}
        </span>
        {cursor && rowsKey === queryKey && (
          <button
            className="text-button"
            onClick={() => void more()}
            disabled={loading}
          >
            {loading ? t("載入中…") : t("載入更多")}
            <ArrowRight size={14} />
          </button>
        )}
      </div>
    </section>
  );
}
