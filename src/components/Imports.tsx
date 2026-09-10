import { useTranslation } from "react-i18next";
import { useEffect, useState } from "react";
import {
  ArrowRight,
  CheckCircle,
  FileText,
  FolderOpen,
  Pause,
  Play,
  UploadSimple,
  WarningCircle,
  X,
} from "@phosphor-icons/react";
import { api, chooseInput, desktop, errorText, useRpc } from "../api";
import type { ImportIssue, Job, Profile } from "../types";
import { number, time } from "../format";
import { Empty, ErrorBanner, Field } from "./UI";
import { diagnosticMessage } from "../diagnostics";
import { Dialog } from "./Dialog";

export function ImportDialog({
  profiles,
  close,
  imported,
}: {
  profiles: Profile[];
  close: () => void;
  imported: () => void;
}) {
  const { t } = useTranslation();
  const [paths, setPaths] = useState<string[]>([]),
    [typed, setTyped] = useState(""),
    [profileId, setProfileId] = useState(profiles[0]?.id || "");
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const profile = profiles.find((p) => p.id === profileId);
  useEffect(() => {
    if (!profileId && profiles[0]) setProfileId(profiles[0].id);
  }, [profileId, profiles]);
  async function select(directory = false) {
    try {
      const p = await chooseInput(directory);
      if (p.length) setPaths(p);
    } catch (e) {
      setError(errorText(e));
    }
  }
  async function start() {
    if (!profile || busy) return;
    const selected = desktop
      ? paths
      : typed
          .split("\n")
          .map((p) => p.trim())
          .filter(Boolean);
    if (!selected.length) return;
    setBusy(true);
    setError("");
    try {
      await api<Job>({ op: "start_import", paths: selected, profile });
      imported();
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  }
  return (
    <Dialog
      label={t("匯入牌譜")}
      close={() => {
        if (!busy) close();
      }}
    >
      <section
        className="modal import-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-heading">
          <div>
            <span className="section-kicker">{t("IMPORT HAND HISTORIES")}</span>
            <h2>{t("把牌局帶入 RiverLens")}</h2>
          </div>
          <button
            className="icon-button"
            onClick={close}
            disabled={busy}
            aria-label={t("關閉匯入")}
          >
            <X size={21} />
          </button>
        </div>
        <p className="muted">
          {t("選取 PokerCraft 匯出嘅 TXT 或 ZIP。原始檔案保持不變。")}
        </p>
        <Field label={t("來源設定")}>
          <select
            value={profileId}
            disabled={busy}
            onChange={(e) => setProfileId(e.target.value)}
          >
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} · {p.brand}
              </option>
            ))}
          </select>
        </Field>
        {!profiles.length && (
          <ErrorBanner>
            {t("尚無來源設定，請先至「資料與設定」新增來源。")}
          </ErrorBanner>
        )}
        {profile && (
          <div className="import-profile">
            <span>
              {t("Hero ")}
              <strong>{profile.hero}</strong>
            </span>
            <span>
              {t("原始時區 ")}
              <strong>{profile.timezone}</strong>
            </span>
          </div>
        )}
        {desktop ? (
          <div className="import-dropzone">
            <UploadSimple size={32} weight="duotone" />
            <h3>{t("選取已匯出嘅牌譜")}</h3>
            <p>{t("TXT、ZIP 或包含牌譜嘅資料夾")}</p>
            <div>
              <button
                className="button primary"
                disabled={busy}
                onClick={() => void select()}
              >
                <FileText size={16} />
                {t("選擇檔案")}
              </button>
              <button
                className="button subtle"
                disabled={busy}
                onClick={() => void select(true)}
              >
                <FolderOpen size={16} />
                {t("資料夾")}
              </button>
            </div>
            {paths.map((p) => (
              <small className="selected-path" key={p}>
                {p}
              </small>
            ))}
          </div>
        ) : (
          <Field
            label={t("本機完整路徑（開發預覽）")}
            help={t(
              "每行一個 TXT、ZIP 或資料夾；由同一部電腦嘅 Rust 引擎讀取。",
            )}
          >
            <textarea
              autoFocus
              disabled={busy}
              rows={4}
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              placeholder={t("/Users/你的名稱/Downloads/牌譜.zip")}
            />
          </Field>
        )}
        <div className="import-checks">
          <span>
            <CheckCircle size={16} />
            {t("重複手牌自動略過")}
          </span>
          <span>
            <CheckCircle size={16} />
            {t("異常牌局保留並標示")}
          </span>
          <span>
            <CheckCircle size={16} />
            {t("全程本機處理")}
          </span>
        </div>
        {error && <ErrorBanner>{error}</ErrorBanner>}
        <button
          className="button primary full-width"
          disabled={
            busy || !profile || (desktop ? !paths.length : !typed.trim())
          }
          onClick={() => void start()}
        >
          {busy ? t("建立工作中…") : t("開始匯入")}
          <ArrowRight size={17} />
        </button>
        <p className="helper">
          {t("只匯入自己已完成嘅牌局；本程式不連接 PokerCraft 或遊戲客戶端。")}
        </p>
      </section>
    </Dialog>
  );
}
export function Imports({
  jobs,
  revision,
  refresh,
  openHand,
  notify,
  onImport,
}: {
  jobs: Job[];
  revision: number;
  refresh: () => void;
  openHand: (id: number) => void;
  notify: (s: string) => void;
  onImport: () => void;
}) {
  const { t } = useTranslation();
  const [issuePages, setIssuePages] = useState<(number | null)[]>([null]);
  const issues = useRpc<{
    rows: ImportIssue[];
    total: number;
    next_cursor: number | null;
  }>({ op: "issues", before: issuePages[issuePages.length - 1] }, revision);
  const [error, setError] = useState(""),
    [raw, setRaw] = useState<string | null>(null);
  async function act(op: "cancel_import" | "resume_import", id: string) {
    try {
      await api({ op, id });
      refresh();
      notify(
        op === "cancel_import"
          ? t("會在目前小批次完成後暫停")
          : t("續匯已開始；已存入嘅手牌不會重複"),
      );
    } catch (e) {
      setError(errorText(e));
    }
  }
  async function viewIssue(issue: ImportIssue) {
    if (issue.hand_row) {
      openHand(issue.hand_row);
      return;
    }
    try {
      const r = await api<{ raw: string | null }>({
        op: "issue_raw",
        id: issue.id,
      });
      setRaw(r.raw || issue.message);
    } catch (e) {
      setError(errorText(e));
    }
  }
  const states: Record<string, string> = {
    running: t("匯入中"),
    complete: t("完成"),
    cancelled: t("已暫停"),
    interrupted: t("已中斷"),
    failed: t("需要處理"),
  };
  return (
    <div className="imports-view">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("IMPORT HISTORY")}</span>
            <h2>{t("匯入紀錄")}</h2>
          </div>
          <span className="subtle-badge">
            {t("{{v0}} 個工作", { count: jobs.length, v0: jobs.length })}
          </span>
        </div>
        {!jobs.length ? (
          <Empty
            title={t("資料庫準備就緒")}
            description={t(
              "由 Natural8／GGPoker PokerCraft 匯出自己嘅牌譜，再匯入呢度。",
            )}
            action={
              <button className="button primary" onClick={onImport}>
                {t("選擇牌譜")}
              </button>
            }
          />
        ) : (
          <div className="import-jobs">
            {jobs.map((j) => (
              <article className="import-job" key={j.id}>
                <div className="job-heading">
                  <span
                    className={`job-icon ${j.state === "complete" ? "positive" : ""}`}
                  >
                    {j.state === "complete" ? (
                      <CheckCircle size={23} />
                    ) : (
                      <FileText size={23} />
                    )}
                  </span>
                  <div>
                    <h3>{j.profile.name}</h3>
                    <small>{time(j.started_at)} · HKT</small>
                  </div>
                  <span className={`job-state state-${j.state}`}>
                    {states[j.state] || j.state}
                  </span>
                </div>
                <div className="job-progress">
                  <i
                    style={{
                      width: `${j.files_total ? (j.files_done / j.files_total) * 100 : 0}%`,
                    }}
                  />
                </div>
                <div className="job-numbers">
                  <span>
                    <strong>{number(j.scanned)}</strong>
                    {t(" 掃描手數")}
                  </span>
                  <span>
                    <strong>{number(j.inserted)}</strong>
                    {t(" 新增")}
                  </span>
                  <span>
                    <strong>{number(j.duplicates)}</strong>
                    {t(" 重複")}
                  </span>
                  <span className={j.conflicts ? "warning" : ""}>
                    <strong>{number(j.conflicts)}</strong>
                    {t(" 衝突")}
                  </span>
                  <span className={j.quarantined ? "warning" : ""}>
                    <strong>{number(j.quarantined)}</strong>
                    {t(" 隔離／不支援")}
                  </span>
                  <span>
                    <strong>
                      {j.files_done}/{j.files_total}
                    </strong>
                    {t(" 檔案")}
                  </span>
                </div>
                <div className="job-bottom">
                  <small className="mono" title={j.current_file}>
                    {j.current_file.split("/").pop() ||
                      j.paths[0]?.split("/").pop()}
                  </small>
                  {j.state === "running" && (
                    <button
                      className="text-button"
                      onClick={() => void act("cancel_import", j.id)}
                    >
                      <Pause size={14} />
                      {t("暫停")}
                    </button>
                  )}
                  {["cancelled", "interrupted", "failed"].includes(j.state) && (
                    <button
                      className="text-button"
                      onClick={() => void act("resume_import", j.id)}
                    >
                      <Play size={14} />
                      {t("續匯")}
                    </button>
                  )}
                  {j.state === "complete" && (
                    <small>
                      {t("{{v0}}s", { v0: (j.elapsed_ms / 1000).toFixed(1) })}
                    </small>
                  )}
                </div>
                {j.message && <ErrorBanner>{j.message}</ErrorBanner>}
              </article>
            ))}
          </div>
        )}
        {jobs.length > 0 && (
          <p className="panel-footnote">
            {t(
              "續匯會重新掃描尚未完成的檔案，略過已匯入手牌；工作計數包含重新掃描的紀錄。",
            )}
          </p>
        )}
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("DATA QUALITY")}</span>
            <h2>{t("需要留意嘅原始資料")}</h2>
          </div>
          <span className="subtle-badge">
            {t("{{v0}} 項", {
              count: issues.data?.total || 0,
              v0: issues.data?.total || 0,
            })}
          </span>
        </div>
        {issues.error && <ErrorBanner>{issues.error}</ErrorBanner>}
        {issues.data?.rows.length ? (
          <div className="issue-list">
            {issues.data.rows.map((i) => (
              <button
                className="issue-row"
                key={i.id}
                onClick={() => void viewIssue(i)}
              >
                <WarningCircle size={20} className="warning" />
                <div>
                  <strong>
                    {t(`issue.${i.code}`, { defaultValue: i.code })}
                  </strong>
                  <p>{diagnosticMessage(i.message)}</p>
                  <small className="mono">
                    {i.hand_id ? `#${i.hand_id}` : i.file.split("/").pop()}
                  </small>
                </div>
                <ArrowRight size={16} />
              </button>
            ))}
          </div>
        ) : !issues.error && !issues.loading ? (
          <div className="quality-clear">
            <CheckCircle size={21} />
            {t("目前未有記錄到資料問題")}
          </div>
        ) : null}
        {issues.data &&
          (issuePages.length > 1 || issues.data.next_cursor != null) && (
            <div className="table-foot">
              <button
                className="text-button"
                disabled={issuePages.length === 1 || issues.loading}
                onClick={() => setIssuePages((p) => p.slice(0, -1))}
              >
                {t("上一頁")}
              </button>
              <span>
                {t("第 {{v0}} 頁 · 每頁最多 300 項", { v0: issuePages.length })}
              </span>
              <button
                className="text-button"
                disabled={issues.data.next_cursor == null || issues.loading}
                onClick={() =>
                  setIssuePages((p) => [...p, issues.data!.next_cursor])
                }
              >
                {t("下一頁")}
              </button>
            </div>
          )}
        <p className="panel-footnote">
          {t(
            "異常原文不會被修改。被隔離牌局不進入核心統計；重複 Hand ID 但內容不同會保留衝突記錄。",
          )}
        </p>
      </section>
      {raw !== null && (
        <Dialog label={t("異常原文")} close={() => setRaw(null)}>
          <section className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-heading">
              <h2>{t("原始資料")}</h2>
              <button
                className="icon-button"
                onClick={() => setRaw(null)}
                aria-label={t("關閉")}
              >
                <X />
              </button>
            </div>
            <pre className="raw-hand" tabIndex={0}>
              {raw}
            </pre>
          </section>
        </Dialog>
      )}
    </div>
  );
}
