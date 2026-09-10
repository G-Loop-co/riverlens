import { useTranslation } from "react-i18next";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  ArrowsClockwise,
  Cards,
  ChartLineUp,
  CheckCircle,
  Database,
  DownloadSimple,
  FunnelSimple,
  GridFour,
  HardDrives,
  ListChecks,
  Plus,
  ShieldCheck,
  SlidersHorizontal,
  X,
} from "@phosphor-icons/react";
import { api, errorText, useRpc } from "./api";
import type {
  Filter,
  Health,
  Job,
  Profile,
  Report,
  SavedFilter,
} from "./types";
import { number, CUES, ACTIONS, STAT_LABELS } from "./format";
import { ErrorBanner, Field } from "./components/UI";
import { Overview } from "./components/Overview";
import { HandsView } from "./components/HandsView";
import { RangeView } from "./components/RangeView";
import { ReviewView } from "./components/ReviewView";
import { Imports, ImportDialog } from "./components/Imports";
import { Settings } from "./components/Settings";
import { Replayer } from "./components/Replayer";
import { AdvancedFilters } from "./components/Filters";
import { LanguageSelect } from "./components/LanguageSelect";
import { Dialog } from "./components/Dialog";
import { normalizeFilter, readFilter, writePreference } from "./preferences";

type Page = "overview" | "hands" | "ranges" | "review" | "imports" | "settings";
const nav = [
  ["overview", "牌局總覽", ChartLineUp],
  ["hands", "手牌資料庫", Cards],
  ["ranges", "起手牌矩陣", GridFour],
  ["review", "複盤工作台", ListChecks],
] as const;
const titles: Record<Page, [string, string]> = {
  overview: ["牌局總覽", "從結果到決策，掌握每一手牌。"],
  hands: ["手牌資料庫", "篩選、標記，再逐步回看你嘅行動。"],
  ranges: ["起手牌矩陣", "你實際打過嘅 range；每格都可追溯。"],
  review: ["複盤工作台", "找出值得重看嘅情境，建立自己的筆記。"],
  imports: ["匯入中心", "所有來源、進度與資料問題，集中處理。"],
  settings: ["資料與設定", "牌譜留喺你嘅電腦，備份由你掌握。"],
};

export default function App() {
  const { t } = useTranslation();
  const [page, setPage] = useState<Page>("overview");
  const [filter, setFilter] = useState<Filter>(readFilter);
  const [revision, setRevision] = useState(0);
  const [importOpen, setImportOpen] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [selected, setSelected] = useState<number | null>(null);
  const [toast, setToast] = useState("");
  const [globalError, setGlobalError] = useState("");
  const [saveName, setSaveName] = useState<string | null>(null);
  const [savingFilter, setSavingFilter] = useState(false);
  const [saveError, setSaveError] = useState("");
  const [rangeView, setRangeView] = useState({
    mode: "hands",
    stat: "rfi",
    hover: "AA",
  });
  const [compare, setCompare] = useState<Filter | null>(null);
  const [returnTo, setReturnTo] = useState<{
    page: Page;
    filter: Filter;
  } | null>(null);
  const content = useRef<HTMLElement>(null);
  const scroll = useRef<Record<string, number>>({});
  const profiles = useRpc<Profile[]>({ op: "profiles" }, revision);
  const health = useRpc<Health>({ op: "health" }, revision);
  const jobs = useRpc<Job[]>({ op: "jobs" }, revision);
  const saved = useRpc<SavedFilter[]>({ op: "saved_filters" }, revision);
  const overall = useRpc<Report>(
    { op: "overview", filter: {}, group: "game" },
    revision,
  );
  const refresh = () => setRevision((v) => v + 1);
  useEffect(() => {
    writePreference("riverlens-filter", JSON.stringify(filter));
  }, [filter]);
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(""), 4500);
    return () => clearTimeout(id);
  }, [toast]);
  const running =
    jobs.data?.some((j) => j.state === "running") ||
    health.data?.equity_running;
  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => setRevision((v) => v + 1), 2500);
    return () => clearInterval(id);
  }, [running]);
  function navigate(next: Page) {
    if (content.current) scroll.current[page] = content.current.scrollTop;
    setPage(next);
  }
  useLayoutEffect(() => {
    const main = content.current;
    if (!main) return;
    const top = scroll.current[page] || 0;
    if (!top) {
      main.scrollTop = 0;
      return;
    }
    // Restore after asynchronous content replaces the shorter loading placeholder.
    const restore = () => {
      if (main.querySelector(".skeleton")) return;
      main.scrollTop = top;
      stop();
    };
    const observer = new MutationObserver(restore);
    function stop() {
      observer.disconnect();
      main!.removeEventListener("wheel", stop);
      main!.removeEventListener("pointerdown", stop);
      main!.removeEventListener("keydown", stop);
    }
    observer.observe(main, {
      childList: true,
      subtree: true,
      characterData: true,
    });
    main.addEventListener("wheel", stop, { passive: true });
    main.addEventListener("pointerdown", stop);
    main.addEventListener("keydown", stop);
    restore();
    return stop;
  }, [page]);
  function update(patch: Partial<Filter>) {
    setFilter((f) => normalizeFilter({ ...f, ...patch }));
  }
  function drill(patch: Partial<Filter>, sort?: "win" | "loss") {
    if (sort) writePreference("riverlens-hand-sort", sort);
    setReturnTo({ page, filter: { ...filter } });
    update(patch);
    navigate("hands");
  }
  async function saveFilter() {
    if (!saveName?.trim() || savingFilter) return;
    setSavingFilter(true);
    setSaveError("");
    try {
      await api({ op: "save_filter", name: saveName, filter });
      setSaveName(null);
      setToast(t("篩選已儲存"));
      refresh();
    } catch (e) {
      setSaveError(errorText(e));
    } finally {
      setSavingFilter(false);
    }
  }
  const activeFilters = Object.entries(filter).filter(
    ([k, v]) => v !== undefined && !["currency", "timezone"].includes(k),
  );
  const labels: Record<string, string> = {
    profile: t("來源"),
    source_date: t("來源日期"),
    date_from: t("由"),
    date_to: t("至"),
    position: t("位置"),
    game: t("牌局"),
    stakes: t("盲注"),
    player_count: t("人數"),
    pot_type: t("底池"),
    hand_class: t("起手牌"),
    texture: t("牌面"),
    paired: t("配對"),
    high_card: t("最高牌"),
    stack_min: t("籌碼 ≥"),
    stack_max: t("籌碼 ≤"),
    effective_min: t("有效 ≥"),
    effective_max: t("有效 ≤"),
    stat: t("統計"),
    stat_mode: t("樣本"),
    cue: t("情境"),
    tag: t("標籤"),
    reviewed: t("已複盤"),
    status: t("狀態"),
    ev_status: "EV",
    result: t("結果"),
    street: t("街道"),
    action: t("行動"),
    bet_min: "Bet% ≥",
    bet_max: "Bet% ≤",
    session: "Session",
    flop_players: t("Flop 人數"),
    showdown: "Showdown",
    search: "Hand ID",
  };

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        {t("跳至主要內容")}
      </a>
      <aside className="sidebar" aria-label={t("主要導覽")}>
        <button
          className="brand"
          onClick={() => navigate("overview")}
          aria-label={t("RiverLens 首頁")}
        >
          <span className="brand-mark">
            <Cards size={25} weight="duotone" />
          </span>
          <span>
            {t("riverlens")}
            <span className="brand-dot">.</span>
          </span>
        </button>
        <div className="nav-caption">{t("分析")}</div>
        <nav>
          {nav.map(([id, label, Icon]) => (
            <button
              key={id}
              onClick={() => navigate(id)}
              className={`nav-item ${page === id ? "active" : ""}`}
              aria-current={page === id ? "page" : undefined}
            >
              <Icon size={20} weight={page === id ? "duotone" : "regular"} />
              <span>{t(label)}</span>
              {id === "hands" && (
                <small>{overall.data ? number(overall.data.hands) : "—"}</small>
              )}
            </button>
          ))}
        </nav>
        <div className="nav-caption manage">{t("管理")}</div>
        <button
          className={`nav-item ${page === "imports" ? "active" : ""}`}
          aria-current={page === "imports" ? "page" : undefined}
          onClick={() => navigate("imports")}
        >
          <DownloadSimple size={20} />
          <span>{t("匯入中心")}</span>
          {jobs.data?.some((j) => j.state === "running") && (
            <i className="status-dot" />
          )}
        </button>
        <button
          className={`nav-item ${page === "settings" ? "active" : ""}`}
          aria-current={page === "settings" ? "page" : undefined}
          onClick={() => navigate("settings")}
        >
          <SlidersHorizontal size={20} />
          <span>{t("資料與設定")}</span>
        </button>
        <div className="sidebar-bottom">
          <div className="local-status">
            <HardDrives size={20} />
            <div>
              <strong>{t("完全本機")}</strong>
              <small>{t("你的牌譜，只屬於你。")}</small>
            </div>
          </div>
          <div className="version">
            <span>{t("RiverLens")}</span>
            <span>{t("v0.1.0")}</span>
          </div>
        </div>
      </aside>
      <div className="workspace-main">
        <header className="topbar">
          <div className="breadcrumb">
            {t("WORKSPACE ")}
            <span>/</span>{" "}
            <strong>
              {page === "settings" || page === "imports"
                ? t("LIBRARY")
                : t("CASH GAMES")}
            </strong>
          </div>
          <div className="topbar-actions">
            <LanguageSelect />
            <span className="connection">
              <i className={`status-dot ${health.error ? "offline" : ""}`} />
              <span className="connection-label">
                {health.error ? t("引擎未連接") : t("本機資料庫")}
              </span>
            </span>
            <button
              className="icon-button"
              title={t("重新整理")}
              aria-label={t("重新整理")}
              onClick={refresh}
            >
              <ArrowsClockwise size={18} />
            </button>
            <span className="avatar small">{t("H")}</span>
          </div>
        </header>
        <main
          id="main-content"
          tabIndex={-1}
          ref={content}
          className="main-content"
        >
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                {page === "overview"
                  ? t("YOUR GAME, IN FOCUS")
                  : "RIVERLENS / " + page.toUpperCase()}
              </div>
              <h1>{t(titles[page][0])}</h1>
              <p>{t(titles[page][1])}</p>
            </div>
            <button
              className="button primary"
              onClick={() => setImportOpen(true)}
            >
              <Plus size={17} weight="bold" />
              {t("匯入牌譜")}
            </button>
          </div>
          {(globalError || profiles.error) && (
            <ErrorBanner>
              {globalError ||
                t(
                  "本機引擎尚未啟動。桌面版請重新開啟；開發預覽需先執行 npm run serve:core。",
                )}
              <button
                className="text-button"
                onClick={() => {
                  setGlobalError("");
                  refresh();
                }}
              >
                {t("重試")}
              </button>
            </ErrorBanner>
          )}
          {!["imports", "settings"].includes(page) && (
            <>
              <div className="filterbar">
                <div className="filter-select">
                  <Database size={16} />
                  <select
                    aria-label={t("來源篩選")}
                    value={filter.profile || ""}
                    onChange={(e) => update({ profile: e.target.value })}
                  >
                    <option value="">{t("全部來源")}</option>
                    {profiles.data?.map((p) => (
                      <option value={p.id} key={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </div>
                <select
                  aria-label={t("牌局類型")}
                  value={filter.game || ""}
                  onChange={(e) => update({ game: e.target.value })}
                >
                  <option value="">{t("全部 Cash Games")}</option>
                  <option value="Cash">{t("Cash")}</option>
                  <option value="Rush">{t("Rush")}</option>
                </select>
                <select
                  aria-label={t("盲注篩選")}
                  value={filter.stakes || ""}
                  onChange={(e) => update({ stakes: e.target.value })}
                >
                  <option value="">{t("全部盲注")}</option>
                  {[
                    "0.01/0.02",
                    "0.02/0.05",
                    "0.05/0.10",
                    "0.10/0.25",
                    "0.25/0.50",
                    "0.50/1.00",
                    "1.00/2.00",
                  ].map((s) => (
                    <option key={s} value={s}>
                      ${s.replace("/", " / $")}
                    </option>
                  ))}
                </select>
                <select
                  aria-label={t("位置篩選")}
                  value={filter.position || ""}
                  onChange={(e) => update({ position: e.target.value })}
                >
                  <option value="">{t("全部位置")}</option>
                  {[
                    "UTG",
                    "UTG+1",
                    "MP",
                    "HJ",
                    "CO",
                    "BTN",
                    "SB",
                    "BB",
                    "BTN/SB",
                  ].map((p) => (
                    <option key={p}>{p}</option>
                  ))}
                </select>
                <button
                  className={`button subtle ${advanced ? "selected" : ""}`}
                  aria-expanded={advanced}
                  aria-controls="advanced-filters"
                  onClick={() => setAdvanced((v) => !v)}
                >
                  <FunnelSimple size={16} />
                  {t("進階篩選")}
                  {activeFilters.length > 0 && (
                    <span className="count-badge">{activeFilters.length}</span>
                  )}
                </button>
                <div className="filter-spacer" />
                <button
                  className="text-button"
                  onClick={() => {
                    setSaveError("");
                    setSaveName("");
                  }}
                >
                  {t("儲存篩選")}
                </button>
                {saved.data && saved.data.length > 0 && (
                  <select
                    aria-label={t("已儲存篩選")}
                    value=""
                    onChange={(e) => {
                      const s = saved.data?.find(
                        (s) => s.id === Number(e.target.value),
                      );
                      if (s) setFilter(normalizeFilter(s.filter));
                    }}
                  >
                    <option value="">{t("已儲存")}</option>
                    {saved.data.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.name}
                      </option>
                    ))}
                  </select>
                )}
              </div>
              {advanced && <AdvancedFilters filter={filter} update={update} />}
              {activeFilters.length > 0 && (
                <div className="filter-chips">
                  {activeFilters.map(([k, v]) => (
                    <button key={k} onClick={() => update({ [k]: undefined })}>
                      {labels[k] || k}:{" "}
                      {k === "session"
                        ? t("所選 Session")
                        : k === "profile"
                          ? profiles.data?.find((p) => p.id === v)?.name ||
                            String(v)
                          : k === "stat"
                            ? STAT_LABELS[String(v)] || String(v)
                            : k === "cue"
                              ? t(
                                  CUES.find(([id]) => id === v)?.[1] ||
                                    String(v),
                                )
                              : k === "action"
                                ? t(ACTIONS[String(v)] || String(v))
                                : [
                                      "street",
                                      "status",
                                      "ev_status",
                                      "stat_mode",
                                      "reviewed",
                                      "paired",
                                      "showdown",
                                      "result",
                                      "pot_type",
                                      "texture",
                                    ].includes(k)
                                  ? t(String(v))
                                  : String(v)}
                      <X size={11} />
                    </button>
                  ))}
                  <button
                    className="clear-filters"
                    onClick={() => setFilter({})}
                  >
                    {t("清除全部")}
                  </button>
                </div>
              )}
            </>
          )}
          {page === "overview" && (
            <Overview
              filter={filter}
              revision={revision}
              drill={drill}
              onImport={() => setImportOpen(true)}
              compare={compare}
              setCompare={setCompare}
            />
          )}
          {page === "hands" && (
            <>
              {returnTo && (
                <button
                  className="text-button"
                  style={{ marginBottom: 16 }}
                  onClick={() => {
                    setFilter(returnTo.filter);
                    navigate(returnTo.page);
                    setReturnTo(null);
                  }}
                >
                  {t("← 返回報表及原本篩選")}
                </button>
              )}
              <HandsView
                filter={filter}
                revision={revision}
                open={setSelected}
                notify={setToast}
              />
            </>
          )}
          {page === "ranges" && (
            <RangeView
              filter={filter}
              revision={revision}
              drill={drill}
              view={rangeView}
              setView={setRangeView}
            />
          )}
          {page === "review" && (
            <ReviewView filter={filter} revision={revision} drill={drill} />
          )}
          {page === "imports" && (
            <Imports
              jobs={jobs.data || []}
              revision={revision}
              refresh={refresh}
              openHand={setSelected}
              notify={setToast}
              onImport={() => setImportOpen(true)}
            />
          )}
          {page === "settings" && (
            <Settings
              profiles={profiles.data || []}
              health={health.data}
              refresh={refresh}
              notify={setToast}
            />
          )}
          <footer className="main-footer">
            <ShieldCheck size={13} />
            <span>{t("本機賽後複盤")}</span>
            <span className="footer-separator">·</span>
            <span>{t("不連接遊戲客戶端")}</span>
            <span className="footer-spacer" />
            <span>{t("HKT · USD / bb")}</span>
          </footer>
        </main>
      </div>
      {importOpen && (
        <ImportDialog
          profiles={profiles.data || []}
          close={() => setImportOpen(false)}
          imported={() => {
            setImportOpen(false);
            refresh();
            navigate("imports");
            setToast(t("背景匯入已開始"));
          }}
        />
      )}
      {selected !== null && (
        <Replayer
          id={selected}
          close={() => setSelected(null)}
          saved={() => {
            refresh();
            setToast(t("複盤筆記已儲存"));
          }}
        />
      )}
      {saveName !== null && (
        <Dialog
          label={t("儲存篩選")}
          close={() => {
            if (!savingFilter) setSaveName(null);
          }}
        >
          <section
            className="modal compact"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-heading">
              <h2>{t("儲存篩選")}</h2>
              <button
                className="icon-button"
                onClick={() => setSaveName(null)}
                disabled={savingFilter}
                aria-label={t("關閉")}
              >
                <X />
              </button>
            </div>
            <Field label={t("名稱")}>
              <input
                autoFocus
                disabled={savingFilter}
                maxLength={150}
                value={saveName}
                onChange={(e) => setSaveName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveFilter();
                }}
                placeholder={t("例如：BB 面對 3-bet")}
              />
            </Field>
            {saveError && <ErrorBanner>{saveError}</ErrorBanner>}
            <button
              className="button primary"
              disabled={savingFilter || !saveName.trim()}
              onClick={() => void saveFilter()}
            >
              {savingFilter ? t("儲存中…") : t("儲存")}
            </button>
          </section>
        </Dialog>
      )}
      {toast && (
        <div className="toast" role="status">
          <CheckCircle size={18} weight="fill" />
          {toast}
        </div>
      )}
    </div>
  );
}
