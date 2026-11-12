import {
  ArrowsClockwise,
  Cards,
  DownloadSimple,
  HardDrives,
  ShieldCheck,
} from "@phosphor-icons/react";
import type { RefObject, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { Workspace } from "../routes";
import { StatusPill } from "./RouteUI";

export interface AppNavItem {
  workspace: Workspace;
  label: string;
  icon: ReactNode;
  detail?: string;
  busy?: boolean;
}

export function AppShell({
  nav,
  activeWorkspace,
  onNavigate,
  healthError,
  version,
  actions,
  mainRef,
  onRefresh,
  children,
}: {
  nav: readonly AppNavItem[];
  activeWorkspace: Workspace;
  onNavigate: (workspace: Workspace) => void;
  healthError?: boolean;
  version?: string;
  onRefresh: () => void;
  mainRef?: RefObject<HTMLElement | null>;
  actions: ReactNode;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const analysisNav = nav.filter((item) =>
    ["today", "hands", "learn", "research", "coach"].includes(item.workspace),
  );
  const dataNav = nav.filter((item) => item.workspace === "data");
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        {t("跳至主要內容")}
      </a>
      <aside className="sidebar" aria-label={t("主要導覽")}>
        <button
          className="brand"
          onClick={() => onNavigate("today")}
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
        <div className="nav-caption">{t("工作區")}</div>
        <nav>
          {analysisNav.map((item) => (
            <button
              key={item.workspace}
              onClick={() => onNavigate(item.workspace)}
              className={`nav-item ${activeWorkspace === item.workspace ? "active" : ""}`}
              aria-current={
                activeWorkspace === item.workspace ? "page" : undefined
              }
            >
              {item.icon}
              <span>{t(item.label)}</span>
              {item.detail && <small>{item.detail}</small>}
              {item.busy && <i className="status-dot" aria-hidden="true" />}
            </button>
          ))}
        </nav>
        <div className="nav-caption manage">{t("資料")}</div>
        {dataNav.map((item) => (
          <button
            key={item.workspace}
            className={`nav-item ${activeWorkspace === item.workspace ? "active" : ""}`}
            aria-current={
              activeWorkspace === item.workspace ? "page" : undefined
            }
            onClick={() => onNavigate(item.workspace)}
          >
            <DownloadSimple size={20} />
            <span>{t(item.label)}</span>
            {item.busy && <i className="status-dot" aria-hidden="true" />}
          </button>
        ))}
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
            {version && <span>{version}</span>}
          </div>
        </div>
      </aside>
      <div className="workspace-main">
        <header className="topbar">
          <div className="breadcrumb">
            {t("WORKSPACE ")}
            <span aria-hidden="true">/</span>
            <strong>{t("RIVERLENS")}</strong>
          </div>
          <div className="topbar-actions">
            {actions}
            <StatusPill tone={healthError ? "offline" : "online"}>
              {healthError ? t("引擎未連接") : t("本機資料庫")}
            </StatusPill>
            <button
              className="icon-button"
              title={t("重新整理")}
              aria-label={t("重新整理")}
              onClick={onRefresh}
            >
              <ArrowsClockwise size={18} />
            </button>
            <span className="avatar small" aria-hidden="true">
              {t("H")}
            </span>
          </div>
        </header>
        <main
          id="main-content"
          tabIndex={-1}
          ref={mainRef}
          className="main-content"
        >
          {children}
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
    </div>
  );
}
