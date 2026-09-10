import { useTranslation } from "react-i18next";
import { useState } from "react";
import {
  ArrowClockwise,
  Database,
  DownloadSimple,
  HardDrives,
  Plus,
  ShieldCheck,
  UploadSimple,
} from "@phosphor-icons/react";
import { api, chooseOutput, desktop, errorText } from "../api";
import type { Health, Profile } from "../types";
import { ErrorBanner, Field } from "./UI";
import { LanguageSelect } from "./LanguageSelect";
import { ThemeSelect } from "./ThemeSelect";

export function Settings({
  profiles,
  health,
  refresh,
  notify,
}: {
  profiles: Profile[];
  health?: Health | null;
  refresh: () => void;
  notify: (s: string) => void;
}) {
  const { t } = useTranslation();
  const [error, setError] = useState(""),
    [busy, setBusy] = useState("");
  const [creating, setCreating] = useState(false);
  const [draft, setDraft] = useState({
    name: "",
    hero: "Hero",
    timezone: "Asia/Hong_Kong",
    brand: "Natural8",
  });
  async function backup() {
    if (busy) return;
    setError("");
    setBusy("backup");
    try {
      const path = desktop
        ? await chooseOutput(
            "db",
            `riverlens-backup-${new Date().toISOString().slice(0, 10)}.db`,
          )
        : prompt(t("備份輸出完整路徑（不覆寫現有檔案）"));
      if (!path) return;
      setBusy("backup");
      await api({ op: "backup", path });
      notify(t("備份已儲存：{{v0}}", { v0: path }));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy("");
    }
  }
  async function restore() {
    if (busy) return;
    setError("");
    setBusy("restore");
    try {
      let path: string | null = null;
      if (desktop) {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const selected = await open({
          multiple: false,
          filters: [{ name: "RiverLens SQLite backup", extensions: ["db"] }],
        });
        if (typeof selected === "string") path = selected;
      } else path = prompt(t("備份 .db 完整路徑"));
      if (!path) return;
      if (
        !confirm(
          t(
            "還原會替換目前資料庫。程式會先自動備份現有資料，再驗證並還原所選備份。繼續？",
          ),
        )
      )
        return;
      setBusy("restore");
      const r = await api<{ safety_backup: string }>({ op: "restore", path });
      notify(t("已還原。原資料備份：{{v0}}", { v0: r.safety_backup }));
      refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy("");
    }
  }
  async function rebuild() {
    if (busy) return;
    setError("");
    if (
      !confirm(
        t(
          "使用目前 parser 從已保存原文重算，並保留筆記。開始前會建立安全備份；大型資料庫需較長時間。繼續？",
        ),
      )
    )
      return;
    setBusy("rebuild");
    try {
      await api({ op: "rebuild" });
      notify(t("原文重算完成"));
      refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy("");
    }
  }
  async function create() {
    if (busy) return;
    setError("");
    setBusy("profile");
    try {
      await api({
        op: "save_profile",
        profile: {
          ...draft,
          name: draft.name.trim(),
          hero: draft.hero.trim(),
          timezone: draft.timezone.trim(),
          id: `profile-${crypto.randomUUID()}`,
        },
      });
      setCreating(false);
      setDraft({
        name: "",
        hero: "Hero",
        timezone: "Asia/Hong_Kong",
        brand: "Natural8",
      });
      refresh();
      notify(t("來源設定已新增"));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy("");
    }
  }
  async function equity() {
    if (busy || !health) return;
    setError("");
    setBusy("equity");
    try {
      await api({ op: "equity_control", paused: !health?.equity_paused });
      refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy("");
    }
  }
  return (
    <div className="settings-view">
      <ThemeSelect />
      <section className="panel">
        <div className="settings-action language-setting">
          <div>
            <h2>{t("介面語言")}</h2>
            <p>
              {t(
                "語言選擇會儲存於本機；不影響牌譜原文、筆記、篩選條件或金額。",
              )}
            </p>
          </div>
          <LanguageSelect />
        </div>
      </section>
      {error && <ErrorBanner>{error}</ErrorBanner>}
      {health?.equity_error && (
        <ErrorBanner>
          {t("Equity 計算已暫停：{{v0}}。可重試「繼續計算」。", {
            v0: health.equity_error,
          })}
        </ErrorBanner>
      )}
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("SOURCE PROFILES")}</span>
            <h2>{t("牌譜來源")}</h2>
          </div>
          <button
            className="button subtle"
            onClick={() => setCreating((v) => !v)}
            disabled={!!busy}
            aria-expanded={creating}
          >
            <Plus size={16} />
            {t("新增來源")}
          </button>
        </div>
        <div className="profile-list">
          {profiles.map((p) => (
            <div className="profile-row" key={p.id}>
              <span className="profile-icon">
                <Database size={22} />
              </span>
              <div>
                <h3>{p.name}</h3>
                <small>
                  {t("{{v0}} · Hero: {{v1}} · {{v2}}", {
                    v0: p.brand,
                    v1: p.hero,
                    v2: p.timezone,
                  })}
                </small>
              </div>
              <span className="subtle-badge">{t("本機")}</span>
            </div>
          ))}
        </div>
        {creating && (
          <fieldset className="profile-form" disabled={!!busy}>
            <Field label={t("顯示名稱")}>
              <input
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                placeholder={t("例如：GGPoker Hero")}
              />
            </Field>
            <Field label={t("品牌")}>
              <select
                value={draft.brand}
                onChange={(e) => setDraft({ ...draft, brand: e.target.value })}
              >
                <option value="Natural8">Natural8</option>
                <option value="GGPoker">GGPoker</option>
              </select>
            </Field>
            <Field label={t("牌譜內 Hero 名稱")}>
              <input
                value={draft.hero}
                onChange={(e) => setDraft({ ...draft, hero: e.target.value })}
              />
            </Field>
            <Field label={t("牌譜內原始時區")}>
              <input
                value={draft.timezone}
                onChange={(e) =>
                  setDraft({ ...draft, timezone: e.target.value })
                }
              />
            </Field>
            <button
              className="button primary"
              disabled={
                !!busy ||
                !draft.name.trim() ||
                !draft.hero.trim() ||
                !draft.timezone.trim()
              }
              onClick={() => void create()}
            >
              {t("建立來源")}
            </button>
          </fieldset>
        )}
        <p className="panel-footnote">
          {t(
            "時區依牌譜內文設定；不使用檔名時間。同一份牌譜應使用同一來源，避免跨 profile 重複計算。",
          )}
        </p>
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("LOCAL DATA")}</span>
            <h2>{t("備份與還原")}</h2>
          </div>
          <HardDrives size={23} />
        </div>
        <div className="settings-action">
          <div>
            <h3>{t("一致性資料庫備份")}</h3>
            <p>{t("包含牌譜原文、統計、筆記、標籤及篩選；不依賴原始 ZIP。")}</p>
          </div>
          <button
            className="button subtle"
            disabled={!!busy}
            onClick={() => void backup()}
          >
            <DownloadSimple size={17} />
            {busy === "backup" ? t("備份中…") : t("建立備份")}
          </button>
        </div>
        <div className="settings-action">
          <div>
            <h3>{t("從備份還原")}</h3>
            <p>
              {t("先驗證資料庫並保存目前資料，再替換；請先完成或暫停匯入。")}
            </p>
          </div>
          <button
            className="button subtle"
            disabled={!!busy || health?.import_active}
            onClick={() => void restore()}
          >
            <UploadSimple size={17} />
            {busy === "restore" ? t("還原中…") : t("選擇備份")}
          </button>
        </div>
        <div className="settings-action">
          <div>
            <h3>{t("從原文重新計算")}</h3>
            <p>{t("以目前 parser 重新解析帳本與統計，保留複盤筆記。")}</p>
          </div>
          <button
            className="button subtle"
            disabled={!!busy || health?.import_active}
            onClick={() => void rebuild()}
          >
            <ArrowClockwise size={17} />
            {busy === "rebuild" ? t("重算中…") : t("重算資料")}
          </button>
        </div>
      </section>
      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="section-kicker">{t("ENGINE")}</span>
            <h2>{t("本機分析引擎")}</h2>
          </div>
          <ShieldCheck size={23} />
        </div>
        <div className="settings-action">
          <div>
            <h3>{t("All-in equity 精確枚舉")}</h3>
            <p>
              {!health
                ? t("未連接")
                : health.equity_paused
                  ? t("已暫停，可隨時續算。")
                  : health?.equity_running
                    ? t("背景計算中，結果逐手保存。")
                    : t("啟用；適用牌局匯入後自動計算。")}
            </p>
          </div>
          <button
            className="button subtle"
            disabled={!!busy || !health}
            onClick={() => void equity()}
          >
            {health?.equity_paused ? t("繼續計算") : t("暫停計算")}
          </button>
        </div>
        <dl className="engine-details">
          <dt>{t("Parser")}</dt>
          <dd>{health?.parser || "—"}</dd>
          <dt>{t("Statistics")}</dt>
          <dd>{health?.stats || "—"}</dd>
          <dt>{t("Schema")}</dt>
          <dd>{health?.schema || "—"}</dd>
          <dt>{t("Database")}</dt>
          <dd className="mono">{health?.database || t("未連接")}</dd>
        </dl>
        <p className="panel-footnote">
          {t(
            "支援範圍：NLHE USD Cash／Rush 格式。Natural8 普通 Cash 已用真實樣本驗證；GGPoker 品牌與 Rush 真實樣本驗收仍待完成。無 live HUD、RTA、雲端同步或 Solver 評分。",
          )}
        </p>
      </section>
    </div>
  );
}
