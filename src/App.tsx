import { useTranslation } from "react-i18next";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  lazy,
  useRef,
  Suspense,
  useState,
} from "react";
import { getVersion } from "@tauri-apps/api/app";
import { useLocation, useNavigate, useSearchParams } from "react-router";
import {
  Cards,
  ChartLineUp,
  CheckCircle,
  Database,
  DownloadSimple,
  FunnelSimple,
  GraduationCap,
  ListChecks,
  Plus,
  X,
} from "@phosphor-icons/react";
import { api, desktop, errorText, useRpc } from "./api";
import type {
  Filter,
  Health,
  Job,
  Profile,
  Report,
  SavedFilter,
} from "./types";
import { number, CUES, ACTIONS, STAT_LABELS } from "./format";
import { ErrorBanner, Field, Skeleton } from "./components/UI";
import type { MdaTab } from "./components/MdaWorkspace";
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
import { AppShell, type AppNavItem } from "./components/AppShell";
import {
  RouteTabs,
  RouteErrorBoundary,
  TabPanel,
  WorkspaceHeading,
} from "./components/RouteUI";
import {
  DEFAULT_SECTIONS,
  filterFromSearchParams,
  filterToSearchParams,
  isSafeReturnPath,
  parseRoute,
  routeSearch,
  workspaceHref,
  type Workspace,
} from "./routes";

const MdaWorkspace = lazy(() =>
  import("./components/MdaWorkspace").then(({ MdaWorkspace }) => ({
    default: MdaWorkspace,
  })),
);
const AiCoach = lazy(() =>
  import("./components/AiCoach").then(({ AiCoach }) => ({
    default: AiCoach,
  })),
);
const StudyWorkspace = lazy(() =>
  import("./components/StudyWorkspace").then(({ StudyWorkspace }) => ({
    default: StudyWorkspace,
  })),
);

export default function App() {
  const { t } = useTranslation();
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const parsedRoute = parseRoute(location.pathname);
  const route = parsedRoute ?? {
    workspace: "today" as Workspace,
    section: DEFAULT_SECTIONS.today,
    pathname: workspaceHref("today"),
  };
  const filter = useMemo(
    () =>
      normalizeFilter({
        currency: "USD",
        ...readFilter(),
        ...filterFromSearchParams(searchParams),
      }),
    [searchParams],
  );
  const [revision, setRevision] = useState(0);
  const [appVersion, setAppVersion] = useState<string>();
  const [importOpen, setImportOpen] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [aiQuestion, setAiQuestion] = useState("");
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
  const content = useRef<HTMLElement>(null);
  const scroll = useRef<Record<string, number>>({});
  const returnTo =
    typeof location.state?.from === "string" &&
    isSafeReturnPath(location.state.from)
      ? location.state.from
      : null;
  const routeKey = `${route.workspace}/${route.section}`;
  const profiles = useRpc<Profile[]>({ op: "profiles" }, revision);
  const health = useRpc<Health>({ op: "health" }, revision);
  const jobs = useRpc<Job[]>({ op: "jobs" }, revision);
  const saved = useRpc<SavedFilter[]>({ op: "saved_filters" }, revision);
  const overall = useRpc<Report>(
    { op: "overview", filter: { currency: filter.currency }, group: "game" },
    revision,
  );
  const refresh = () => setRevision((v) => v + 1);
  useEffect(() => {
    if (!parsedRoute) {
      navigate(`/today${routeSearch(filter)}`, { replace: true });
    }
  }, [parsedRoute, navigate, filter]);
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(""), 4500);
    return () => clearTimeout(id);
  }, [toast]);
  const running =
    jobs.data?.some((j) => j.state === "running") ||
    health.data?.equity_running ||
    health.data?.study_running;
  useEffect(() => {
    if (!desktop) return;
    let active = true;
    getVersion()
      .then((version) => {
        if (active) setAppVersion(`v${version}`);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => setRevision((v) => v + 1), 2500);
    return () => clearInterval(id);
  }, [running]);
  function goWorkspace(workspace: Workspace) {
    const pathname = workspaceHref(workspace);
    const extras =
      workspace === "research"
        ? { dataset: searchParams.get("dataset") ?? undefined }
        : workspace === "learn"
          ? {
              pack: searchParams.get("pack") ?? undefined,
              node: searchParams.get("node") ?? undefined,
            }
          : {};
    navigate({ pathname, search: routeSearch(filter, extras) });
  }

  function goSection(workspace: Workspace, section: string) {
    const pathname = workspaceHref(workspace, section);
    const extras =
      workspace === "research"
        ? { dataset: searchParams.get("dataset") ?? undefined }
        : workspace === "learn"
          ? {
              pack: searchParams.get("pack") ?? undefined,
              node: searchParams.get("node") ?? undefined,
            }
          : {};
    navigate({ pathname, search: routeSearch(filter, extras) });
  }

  function update(patch: Partial<Filter>) {
    const next = normalizeFilter({ ...filter, ...patch });
    writePreference("riverlens-filter", JSON.stringify(next));
    setSearchParams(filterToSearchParams(next), { replace: true });
  }

  function drill(patch: Partial<Filter>, sort?: "win" | "loss") {
    const next = normalizeFilter({ ...filter, ...patch });
    if (sort) writePreference("riverlens-hand-sort", sort);
    writePreference("riverlens-filter", JSON.stringify(next));
    navigate(
      {
        pathname: workspaceHref("hands", "list"),
        search: routeSearch(next),
      },
      { state: { from: location.pathname + location.search } },
    );
  }

  function openHand(id: number, seq?: number) {
    const from =
      parseRoute(location.pathname)?.pathname ?? workspaceHref("today");
    navigate({
      pathname: `/hands/replay/${id}`,
      search: routeSearch(filter, { step: seq, from }),
    });
  }

  function closeReplay() {
    const from = searchParams.get("from");
    const pathname =
      from && isSafeReturnPath(from) ? from : workspaceHref("hands", "list");
    navigate({ pathname, search: routeSearch(filter) });
  }

  const syncReplayStep = useCallback(
    (seq: number) => {
      const from = searchParams.get("from");
      setSearchParams(
        routeSearch(filter, {
          step: seq,
          from: from ?? parseRoute(location.pathname)?.pathname,
        }),
        { replace: true },
      );
    },
    [filter, location.pathname, searchParams, setSearchParams],
  );

  useLayoutEffect(() => {
    const main = content.current;
    if (!main) return;
    const top = scroll.current[routeKey] || 0;
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
  }, [routeKey]);
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
    tournament_id: t("賽事編號"),
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
  const navItems: AppNavItem[] = [
    {
      workspace: "today",
      label: "牌局總覽",
      icon: (
        <ChartLineUp
          size={20}
          weight={route.workspace === "today" ? "duotone" : "regular"}
        />
      ),
    },
    {
      workspace: "hands",
      label: "牌局覆盤",
      icon: (
        <Cards
          size={20}
          weight={route.workspace === "hands" ? "duotone" : "regular"}
        />
      ),
      detail: overall.data ? number(overall.data.hands) : undefined,
    },
    {
      workspace: "learn",
      label: "學習工作台",
      icon: (
        <GraduationCap
          size={20}
          weight={route.workspace === "learn" ? "duotone" : "regular"}
        />
      ),
    },
    {
      workspace: "research",
      label: "MDA 群體分析",
      icon: (
        <Database
          size={20}
          weight={route.workspace === "research" ? "duotone" : "regular"}
        />
      ),
    },
    {
      workspace: "coach",
      label: "AI 教練",
      icon: (
        <ListChecks
          size={20}
          weight={route.workspace === "coach" ? "duotone" : "regular"}
        />
      ),
    },
    {
      workspace: "data",
      label: "資料中心",
      icon: <DownloadSimple size={20} />,
      busy: jobs.data?.some((j) => j.state === "running"),
    },
  ];
  const headings = {
    "today/dashboard": [
      "YOUR GAME, IN FOCUS",
      "牌局總覽",
      "從結果到決策，掌握每一手牌。",
    ],
    "hands/list": [
      "RIVERLENS / HANDS",
      "牌局覆盤",
      "篩選與標記手牌，逐步檢視決策過程。",
    ],
    "hands/matrix": [
      "RIVERLENS / HANDS",
      "起手牌矩陣",
      "呈現實際觀察的 range，每個組合均可追溯。",
    ],
    "hands/review": [
      "RIVERLENS / HANDS",
      "複盤工作台",
      "識別需要檢討的情境，記錄決策依據。",
    ],
    "learn/spots": [
      "RIVERLENS / LEARN",
      "學習工作台",
      "從情境探索到每日練習，建立可追溯的覆盤流程。",
    ],
    "learn/progress": [
      "RIVERLENS / LEARN",
      "學習工作台",
      "從情境探索到每日練習，建立可追溯的覆盤流程。",
    ],
    "learn/leaks": [
      "RIVERLENS / LEARN",
      "學習工作台",
      "從情境探索到每日練習，建立可追溯的覆盤流程。",
    ],
    "learn/strategy": [
      "RIVERLENS / LEARN",
      "學習工作台",
      "從情境探索到每日練習，建立可追溯的覆盤流程。",
    ],
    "learn/range": [
      "RIVERLENS / LEARN",
      "學習工作台",
      "從情境探索到每日練習，建立可追溯的覆盤流程。",
    ],
    "learn/trainer": [
      "RIVERLENS / LEARN",
      "學習工作台",
      "從情境探索到每日練習，建立可追溯的覆盤流程。",
    ],
    "research/stats": [
      "RIVERLENS / RESEARCH",
      "MDA 群體分析",
      "匯入牌池、查群體傾向，再用手牌驗證調整方向。",
    ],
    "research/wizard": [
      "RIVERLENS / RESEARCH",
      "MDA 群體分析",
      "匯入牌池、查群體傾向，再用手牌驗證調整方向。",
    ],
    "research/candidates": [
      "RIVERLENS / RESEARCH",
      "MDA 群體分析",
      "匯入牌池、查群體傾向，再用手牌驗證調整方向。",
    ],
    "research/evidence": [
      "RIVERLENS / RESEARCH",
      "MDA 群體分析",
      "匯入牌池、查群體傾向，再用手牌驗證調整方向。",
    ],
    "research/ai": [
      "RIVERLENS / RESEARCH",
      "MDA 群體分析",
      "匯入牌池、查群體傾向，再用手牌驗證調整方向。",
    ],
    "research/import": [
      "RIVERLENS / RESEARCH",
      "MDA 群體分析",
      "匯入牌池、查群體傾向，再用手牌驗證調整方向。",
    ],
    "coach/situation": [
      "RIVERLENS / COACH",
      "AI 教練",
      "用資料理解局面，把建議變成練習。",
    ],
    "coach/chat": [
      "RIVERLENS / COACH",
      "AI 教練",
      "用資料理解局面，把建議變成練習。",
    ],
    "coach/library": [
      "RIVERLENS / COACH",
      "AI 教練",
      "用資料理解局面，把建議變成練習。",
    ],
    "coach/connections": [
      "RIVERLENS / COACH",
      "AI 教練",
      "用資料理解局面，把建議變成練習。",
    ],
    "data/import": [
      "RIVERLENS / DATA",
      "資料中心",
      "匯入進度、來源設定與資料安全集中管理。",
    ],
    "data/sources": [
      "RIVERLENS / DATA",
      "資料中心",
      "匯入進度、來源設定與資料安全集中管理。",
    ],
    "data/preferences": [
      "RIVERLENS / DATA",
      "資料中心",
      "匯入進度、來源設定與資料安全集中管理。",
    ],
  } as const;
  const heading =
    headings[routeKey as keyof typeof headings] ?? headings["today/dashboard"];
  const showFilter = ["today", "hands", "coach"].includes(route.workspace);
  const showImport =
    route.workspace === "today" ||
    (route.workspace === "hands" && route.section === "list");

  return (
    <>
      <AppShell
        nav={navItems}
        activeWorkspace={route.workspace}
        onNavigate={goWorkspace}
        healthError={Boolean(health.error)}
        version={appVersion}
        onRefresh={refresh}
        mainRef={content}
        actions={
          <>
            {route.workspace !== "coach" && route.workspace !== "research" && (
              <button className="button" onClick={() => goWorkspace("coach")}>
                {t("問 AI")}
              </button>
            )}
            <LanguageSelect />
          </>
        }
      >
        <RouteErrorBoundary key={route.workspace}>
          <WorkspaceHeading
            eyebrow={heading[0]}
            title={heading[1]}
            description={heading[2]}
            action={
              showImport ? (
                <button
                  className="button primary"
                  onClick={() => setImportOpen(true)}
                >
                  <Plus size={17} weight="bold" />
                  {t("匯入牌譜")}
                </button>
              ) : undefined
            }
          />
          {route.workspace === "today" && (
            <div className="context-cards" aria-label={t("下一步行動")}>
              <button onClick={() => goSection("hands", "review")}>
                <strong>{t("下一步覆盤")}</strong>
                <small>{t("檢查未完成複盤與關鍵情境")}</small>
              </button>
              <button onClick={() => goSection("data", "import")}>
                <strong>{t("匯入進度")}</strong>
                <small>
                  {jobs.data?.some((j) => j.state === "running")
                    ? t("匯入中")
                    : t("查看匯入狀態與資料問題")}
                </small>
              </button>
              <button onClick={() => goSection("learn", "trainer")}>
                <strong>{t("學習練習")}</strong>
                <small>
                  {health.data?.study?.complete
                    ? t("開啟 Trainer 與到期題目")
                    : t("等待索引完成後開始練習")}
                </small>
              </button>
            </div>
          )}
          {route.workspace === "data" && (
            <RouteTabs
              label="資料中心"
              panelId="data-workspace"
              value={route.section as "import" | "sources" | "preferences"}
              onChange={(section) => goSection("data", section)}
              items={[
                { value: "import", label: "匯入中心" },
                { value: "sources", label: "資料與設定" },
                { value: "preferences", label: "介面偏好" },
              ]}
            />
          )}
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
          {showFilter && (
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
                  aria-label={t("幣別篩選")}
                  value={filter.currency || ""}
                  onChange={(e) =>
                    update({
                      currency: e.target.value,
                      game:
                        e.target.value === "CHIPS"
                          ? "Tournament"
                          : filter.game === "Tournament"
                            ? undefined
                            : filter.game,
                      tournament_id: undefined,
                      stakes: undefined,
                    })
                  }
                >
                  <option value="">{t("全部幣別 · 金額須分開查看")}</option>
                  {["USD", "EUR", "GBP", "CAD", "USDT", "CHIPS"].map(
                    (currency) => (
                      <option key={currency} value={currency}>
                        {currency === "CHIPS"
                          ? t("CHIPS · 錦標賽籌碼")
                          : currency}
                      </option>
                    ),
                  )}
                </select>
                <select
                  aria-label={t("牌局類型")}
                  value={filter.game || ""}
                  onChange={(e) =>
                    update({
                      game: e.target.value,
                      currency:
                        e.target.value === "Tournament"
                          ? "CHIPS"
                          : filter.currency === "CHIPS" && e.target.value
                            ? "USD"
                            : filter.currency,
                      tournament_id: undefined,
                      stakes: undefined,
                    })
                  }
                >
                  <option value="">{t("全部牌局類型")}</option>
                  <option value="Cash">{t("Cash")}</option>
                  <option value="Rush">{t("Rush")}</option>
                  <option value="Tournament">{t("MTT／錦標賽")}</option>
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
                      {s.replace("/", " / ")}
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
                      if (s) update(normalizeFilter(s.filter));
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
                    onClick={() =>
                      update(
                        Object.fromEntries(
                          Object.keys(filter).map((key) => [key, undefined]),
                        ) as Partial<Filter>,
                      )
                    }
                  >
                    {t("清除全部")}
                  </button>
                </div>
              )}
            </>
          )}
          {route.workspace === "hands" && (
            <RouteTabs
              label="牌局覆盤"
              panelId="hands-workspace"
              value={route.section as "list" | "matrix" | "review"}
              onChange={(section) => goSection("hands", section)}
              items={[
                { value: "list", label: "手牌資料庫" },
                { value: "matrix", label: "起手牌矩陣" },
                { value: "review", label: "複盤工作台" },
              ]}
            />
          )}
          {route.workspace === "research" && (
            <Suspense fallback={<Skeleton rows={6} />}>
              <MdaWorkspace
                tab={
                  (route.section === "evidence"
                    ? "spots"
                    : route.section) as MdaTab
                }
                onTabChange={(tab) =>
                  goSection("research", tab === "spots" ? "evidence" : tab)
                }
              />
            </Suspense>
          )}
          {route.workspace === "coach" && (
            <Suspense fallback={<Skeleton rows={6} />}>
              <AiCoach
                filter={filter}
                question={aiQuestion}
                openHand={openHand}
                tab={
                  route.section === "connections"
                    ? "connect"
                    : route.section === "chat"
                      ? "spots"
                      : route.section === "library"
                        ? "library"
                        : "coach"
                }
                onTabChange={(tab) =>
                  goSection(
                    "coach",
                    tab === "connect"
                      ? "connections"
                      : tab === "spots"
                        ? "chat"
                        : tab === "library"
                          ? "library"
                          : "situation",
                  )
                }
              />
            </Suspense>
          )}
          {route.workspace === "today" && (
            <Overview
              filter={filter}
              revision={revision}
              drill={drill}
              onImport={() => setImportOpen(true)}
              compare={compare}
              setCompare={setCompare}
            />
          )}
          {route.workspace === "hands" && (
            <TabPanel id="hands-workspace" activeTab={route.section}>
              {returnTo && (
                <button
                  className="text-button"
                  style={{ marginBottom: 16 }}
                  onClick={() => {
                    navigate(returnTo);
                  }}
                >
                  {t("← 返回報表及原本篩選")}
                </button>
              )}
              {route.section === "list" && (
                <HandsView
                  filter={filter}
                  revision={revision}
                  open={openHand}
                  notify={setToast}
                />
              )}
              {route.section === "matrix" && (
                <RangeView
                  filter={filter}
                  revision={revision}
                  drill={drill}
                  view={rangeView}
                  setView={setRangeView}
                />
              )}
              {route.section === "review" && (
                <ReviewView filter={filter} revision={revision} drill={drill} />
              )}
            </TabPanel>
          )}
          {route.workspace === "learn" && (
            <Suspense fallback={<Skeleton rows={6} />}>
              <StudyWorkspace
                profiles={profiles.data || []}
                health={health.data || undefined}
                revision={revision}
                refresh={refresh}
                open={openHand}
                tab={route.section as never}
                onTabChange={(tab) => goSection("learn", tab)}
                notify={setToast}
              />
            </Suspense>
          )}
          {route.workspace === "data" && (
            <TabPanel id="data-workspace" activeTab={route.section}>
              {route.section === "import" && (
                <Imports
                  jobs={jobs.data || []}
                  revision={revision}
                  refresh={refresh}
                  openHand={openHand}
                  notify={setToast}
                  onImport={() => setImportOpen(true)}
                />
              )}
              {route.section !== "import" && (
                <Settings
                  profiles={profiles.data || []}
                  health={health.data}
                  refresh={refresh}
                  notify={setToast}
                  section={
                    route.section === "preferences" ? "preferences" : "sources"
                  }
                />
              )}
            </TabPanel>
          )}
        </RouteErrorBoundary>
      </AppShell>
      {importOpen && (
        <ImportDialog
          profiles={profiles.data || []}
          close={() => setImportOpen(false)}
          imported={() => {
            setImportOpen(false);
            refresh();
            goWorkspace("data");
            setToast(t("背景匯入已開始"));
          }}
        />
      )}
      {Boolean(route.handId) && (
        <Replayer
          id={route.handId ?? 0}
          initialStep={Math.max(0, Number(searchParams.get("step") ?? 0))}
          onStep={syncReplayStep}
          askAi={(seq) => {
            setAiQuestion(
              `${t("複盤手牌")} #${route.handId}，${t("決策")} ${seq}`,
            );
            goWorkspace("coach");
          }}
          close={closeReplay}
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
    </>
  );
}
