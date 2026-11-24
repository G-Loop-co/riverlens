import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { desktop, errorText } from "../api";
import { readPreference, writePreference } from "../preferences";

export const AUTO_UPDATE_KEY = "riverlens-auto-check-updates";
type InstallBlocker = () => string | null | Promise<string | null>;
type Phase =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "installing"
  | "restart";

type PendingUpdate = {
  update: Update;
  downloaded: boolean;
  installed: boolean;
};

function release(candidate: PendingUpdate | null) {
  if (candidate) void candidate.update.close().catch(() => {});
}

export function useAppUpdates(getInstallBlocker: InstallBlocker) {
  const [automatic, setAutomatic] = useState(
    () => readPreference(AUTO_UPDATE_KEY, "true") !== "false",
  );
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState(false);
  const [percent, setPercent] = useState<number | null>(null);
  const candidate = useRef<PendingUpdate | null>(null);
  const blocker = useRef(getInstallBlocker);
  blocker.current = getInstallBlocker;
  const enabled = useRef(automatic);
  const busy = useRef(false);
  const mounted = useRef(false);

  const checkNow = useCallback(async (background = false) => {
    if (!desktop || busy.current || candidate.current) return;
    busy.current = true;
    setPhase("checking");
    setError("");
    try {
      const update = await check({ timeout: 15_000 });
      if (!mounted.current || (background && !enabled.current)) {
        if (update) await update.close();
        if (mounted.current) setPhase("idle");
        return;
      }
      if (update) {
        candidate.current = { update, downloaded: false, installed: false };
        setPhase("available");
        setNotice(true);
      } else {
        setPhase("current");
        setNotice(false);
      }
    } catch (e) {
      if (mounted.current) {
        setPhase("idle");
        setError(errorText(e));
      }
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (!busy.current) release(candidate.current);
      candidate.current = null;
    };
  }, []);

  useEffect(() => {
    if (!desktop || !automatic) return;
    const timer = window.setTimeout(() => void checkNow(true), 0);
    return () => window.clearTimeout(timer);
  }, [automatic, checkNow]);

  function changeAutomatic(value: boolean) {
    enabled.current = value;
    setAutomatic(value);
    writePreference(AUTO_UPDATE_KEY, String(value));
  }

  function later() {
    if (busy.current) return;
    setNotice(false);
    // After installation, retain the restart action in Settings.
    if (candidate.current?.installed) return;
    release(candidate.current);
    candidate.current = null;
    setPhase("idle");
    setPercent(null);
    setError("");
  }

  async function install() {
    const pending = candidate.current;
    if (!desktop || !pending || busy.current) return;
    busy.current = true;
    setError("");
    try {
      const reason = await blocker.current();
      if (!mounted.current) return;
      if (reason) {
        setError(reason);
        return;
      }
      if (!pending.downloaded) {
        setPhase("downloading");
        setPercent(null);
        let received = 0;
        let total = 0;
        await pending.update.download((event) => {
          if (!mounted.current) return;
          if (event.event === "Started") total = event.data.contentLength ?? 0;
          if (event.event === "Progress") {
            received += event.data.chunkLength;
            if (total > 0)
              setPercent(Math.min(100, Math.floor((100 * received) / total)));
          }
        });
        pending.downloaded = true;
      }
      if (!mounted.current) return;
      // Lock new work while the final health check runs. Windows can exit inside install().
      setPhase("installing");
      const lastReason = await blocker.current();
      if (!mounted.current) return;
      if (lastReason) {
        setError(lastReason);
        setPhase(pending.installed ? "restart" : "available");
        return;
      }
      if (!pending.installed) {
        await pending.update.install();
        pending.installed = true;
      }
      await relaunch();
    } catch (e) {
      if (!mounted.current) return;
      setError(errorText(e));
      setNotice(true);
      if (pending.installed) {
        setPhase("restart");
      } else {
        // Recheck the feed after a failed transfer/signature/installation; a publisher
        // may have corrected the signature or replaced the advertised version.
        release(pending);
        candidate.current = null;
        setPhase("idle");
      }
    } finally {
      busy.current = false;
      if (!mounted.current) release(pending);
    }
  }

  return {
    automatic,
    changeAutomatic,
    phase,
    error,
    notice,
    percent,
    update: candidate.current?.update,
    installing: phase === "installing",
    checkNow,
    install,
    later,
  };
}

export type AppUpdates = ReturnType<typeof useAppUpdates>;

function UpdateActions({
  updates,
  showLater,
}: {
  updates: AppUpdates;
  showLater: boolean;
}) {
  const { t } = useTranslation();
  const { phase, update, percent, error } = updates;
  const working =
    phase === "checking" || phase === "downloading" || phase === "installing";
  return (
    <div className="settings-action">
      <div>
        <div role="status">
          {phase === "checking" && t("updates.checking")}
          {phase === "current" && t("updates.current")}
          {phase === "downloading" && t("updates.downloading")}
          {phase === "installing" && t("updates.installing")}
          {phase === "restart" && t("updates.restartNeeded")}
        </div>
        {phase === "downloading" && (
          <progress
            aria-label={t("updates.downloading")}
            max={100}
            value={percent ?? undefined}
          />
        )}
        {error && <p role="alert">{error}</p>}
      </div>
      <div className="topbar-actions">
        {!working && showLater && (
          <button className="button subtle" onClick={updates.later}>
            {t("updates.later")}
          </button>
        )}
        {update ? (
          <button
            className="button primary"
            disabled={working}
            onClick={() => void updates.install()}
          >
            {phase === "restart" ? t("updates.restart") : t("updates.install")}
          </button>
        ) : (
          <button
            className="button subtle"
            disabled={working}
            onClick={() => void updates.checkNow()}
          >
            {error ? t("重試") : t("updates.check")}
          </button>
        )}
      </div>
    </div>
  );
}

function ReleaseDetails({ update }: { update: Update }) {
  const { t } = useTranslation();
  const notes = update.body?.trim();
  return (
    <div className="settings-action">
      <div>
        <h3>{t("updates.available", { version: update.version })}</h3>
        {notes && (
          <p
            style={{
              whiteSpace: "pre-line",
              overflowWrap: "anywhere",
              maxHeight: "9rem",
              overflow: "auto",
            }}
            tabIndex={0}
          >
            {notes}
          </p>
        )}
      </div>
    </div>
  );
}

export function UpdateNotice({ updates }: { updates: AppUpdates }) {
  const { t } = useTranslation();
  if (!desktop || !updates.notice) return null;
  return (
    <section className="panel" aria-label={t("updates.title")}>
      {updates.update ? (
        <ReleaseDetails update={updates.update} />
      ) : (
        <div className="panel-heading">
          <h2>{t("updates.title")}</h2>
        </div>
      )}
      <UpdateActions updates={updates} showLater />
    </section>
  );
}

export function UpdateSettings({
  updates,
  version,
}: {
  updates: AppUpdates;
  version?: string;
}) {
  const { t } = useTranslation();
  if (!desktop) return null;
  return (
    <section className="panel" aria-label={t("updates.title")}>
      <div className="panel-heading">
        <h2>{t("updates.title")}</h2>
        {version && <span>{t("updates.version", { version })}</span>}
      </div>
      <div className="settings-action">
        <label>
          <input
            type="checkbox"
            checked={updates.automatic}
            onChange={(e) => updates.changeAutomatic(e.target.checked)}
          />{" "}
          {t("updates.automatic")}
        </label>
      </div>
      {updates.update && <ReleaseDetails update={updates.update} />}
      <UpdateActions updates={updates} showLater={false} />
    </section>
  );
}
