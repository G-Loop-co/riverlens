import { useTranslation } from "react-i18next";
import { diagnosticMessage } from "../diagnostics";
import { useEffect, useMemo, useState } from "react";
import {
  ArrowCounterClockwise,
  CaretLeft,
  CaretRight,
  CheckCircle,
  Eye,
  FloppyDisk,
  Pause,
  Play,
  SkipBack,
  SkipForward,
  X,
} from "@phosphor-icons/react";
import { api, errorText, useRpc } from "../api";
import type { HandDetail } from "../types";
import { ACTIONS, EV_REASONS, money } from "../format";
import { fromUnits, replay } from "../replay";
import { ErrorBanner, Field, Skeleton } from "./UI";
import { Dialog } from "./Dialog";

export function Card({
  card,
  hidden = false,
}: {
  card?: string;
  hidden?: boolean;
}) {
  const { t } = useTranslation();
  const suit = card?.[1];
  const symbol = ({ s: "♠", h: "♥", d: "♦", c: "♣" } as Record<string, string>)[
    suit || ""
  ];
  return (
    <span
      role="img"
      className={`playing-card ${hidden || !card ? "card-back" : ""} suit-${suit}`}
      aria-label={hidden || !card ? t("未知底牌") : card}
    >
      {!hidden && card ? (
        <>
          <strong>{card[0]}</strong>
          <small>{symbol}</small>
        </>
      ) : (
        <span className="card-pattern" />
      )}
    </span>
  );
}

export function Replayer({
  id,
  close,
  saved,
  initialStep = 0,
}: {
  id: number;
  close: () => void;
  saved: () => void;
  initialStep?: number;
}) {
  const { t } = useTranslation();
  const result = useRpc<HandDetail>({ op: "hand", id });
  const h = result.data?.hand;
  const [step, setStep] = useState(0),
    [playing, setPlaying] = useState(false),
    [speed, setSpeed] = useState(1);
  const [tab, setTab] = useState("replay"),
    [reveal, setReveal] = useState(false);
  const [note, setNote] = useState(""),
    [tags, setTags] = useState(""),
    [reviewed, setReviewed] = useState(false);
  const [saving, setSaving] = useState(false),
    [error, setError] = useState(""),
    [clean, setClean] = useState(true);
  useEffect(() => {
    if (result.data) {
      const a = result.data.annotation;
      setNote(a.note);
      setTags(a.tags.join(", "));
      setReviewed(a.reviewed);
      setStep(Math.min(Math.max(initialStep,0),result.data.hand.actions.length));
      setPlaying(false);
      setClean(true);
    }
  }, [result.data,initialStep]);
  useEffect(() => {
    if (!h || !playing) return;
    const t = setInterval(
      () =>
        setStep((s) => {
          if (s >= h.actions.length) {
            setPlaying(false);
            return s;
          }
          return s + 1;
        }),
      850 / speed,
    );
    return () => clearInterval(t);
  }, [h, playing, speed]);
  useEffect(() => {
    function key(e: KeyboardEvent) {
      if (
        (e.target as HTMLElement).matches(
          "input,textarea,select,[contenteditable=true]",
        ) ||
        tab !== "replay"
      )
        return;
      if (!h) return;
      if (e.key === "ArrowRight") {
        e.preventDefault();
        setPlaying(false);
        setStep((s) => Math.min(s + 1, h.actions.length));
      }
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        setPlaying(false);
        setStep((s) => Math.max(s - 1, 0));
      }
      if (e.code === "Space") {
        if ((e.target as HTMLElement).closest("button")) return;
        e.preventDefault();
        if (step === h.actions.length) setStep(0);
        setPlaying((p) => !p);
      }
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });
  const frame = useMemo(() => (h ? replay(h, step) : null), [h, step]);
  function attemptClose() {
    if (saving) return;
    if (!clean && !window.confirm(t("筆記尚未儲存。關閉並放棄呢次編輯？")))
      return;
    close();
  }
  function go(s: number) {
    setStep(s);
    setPlaying(false);
  }
  async function save() {
    if (saving) return;
    setSaving(true);
    setError("");
    try {
      await api({
        op: "save_annotation",
        id,
        annotation: {
          note,
          tags: tags
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
          reviewed,
        },
      });
      setClean(true);
      saved();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  }
  const heroIndex = h?.players.findIndex((p) => p.hero) || 0;
  const last = h?.actions[step - 1];
  return (
    <Dialog label={t("Hand Replayer")} close={attemptClose} drawer>
      <aside className="replayer-drawer" onClick={(e) => e.stopPropagation()}>
        <div className="replayer-heading">
          <div>
            <span className="section-kicker">{t("HAND REPLAYER")}</span>
            <h2>
              {h ? (
                <>
                  <span className="mono">{h.hand_class}</span>
                  <span className="position-badge">{h.position}</span>
                </>
              ) : (
                t("載入牌局…")
              )}
            </h2>
          </div>
          <button
            className="icon-button"
            aria-label={t("關閉回放")}
            onClick={attemptClose}
            disabled={saving}
          >
            <X size={21} />
          </button>
        </div>
        {result.error && <ErrorBanner>{result.error}</ErrorBanner>}
        {result.error ? null : !h || !frame ? (
          <Skeleton rows={8} />
        ) : (
          <>
            <div className="hand-context">
              <span>
                {h.brand} · {h.game} · ${h.sb}/${h.bb}
              </span>
              <span className="mono">
                {h.local_time} {h.timezone}
              </span>
            </div>
            <div className="replayer-tabs">
              {[
                ["replay", t("逐步回放")],
                ["ledger", t("帳本與 EV")],
                ["raw", t("原始牌譜")],
              ].map(([k, l]) => (
                <button
                  key={k}
                  className={tab === k ? "active" : ""}
                  aria-pressed={tab === k}
                  onClick={() => setTab(k)}
                >
                  {t(l)}
                </button>
              ))}
            </div>
            <div className="replayer-body">
              {h.status !== "valid" && (
                <ErrorBanner>
                  {t("此手已隔離：{{v0}}", {
                    v0: h.issues
                      .map((i) => diagnosticMessage(i.message))
                      .join("; "),
                  })}
                </ErrorBanner>
              )}
              {tab === "replay" && (
                <>
                  <div className="table-stage">
                    <div className="poker-felt">
                      <span className="felt-watermark" aria-hidden="true">
                        {t("RIVERLENS")}
                      </span>
                      <div className="board-area">
                        <span className="pot-label">
                          {step === h.actions.length
                            ? t("剩餘底池／費用")
                            : t("底池")}{" "}
                          <strong className="mono">
                            ${money(fromUnits(frame.pot))}
                          </strong>
                        </span>
                        {frame.boards.map(
                          (board, index) =>
                            board?.length > 0 && (
                              <div className="board-cards" key={index}>
                                {frame.boards.length > 1 && (
                                  <small>{index + 1}</small>
                                )}
                                {board.map((card, i) => (
                                  <Card card={card} key={`${i}-${card}`} />
                                ))}
                              </div>
                            ),
                        )}
                      </div>
                    </div>
                    {h.players.map((p, i) => {
                      const relative =
                        (i - heroIndex + h.players.length) % h.players.length;
                      const angle =
                        Math.PI / 2 +
                        (relative * 2 * Math.PI) / h.players.length;
                      const x = 50 + 40 * Math.cos(angle),
                        y = 49 + 36 * Math.sin(angle);
                      const active = last?.actor === p.seat;
                      return (
                        <div
                          className={`table-player ${p.hero ? "is-hero" : ""} ${frame.folded.has(p.seat) ? "folded" : ""} ${active ? "acting" : ""}`}
                          key={p.seat}
                          style={{ left: `${x}%`, top: `${y}%` }}
                        >
                          <div className="player-cards">
                            <Card
                              card={p.cards[0]}
                              hidden={!(reveal || frame.shown.has(p.seat))}
                            />
                            <Card
                              card={p.cards[1]}
                              hidden={!(reveal || frame.shown.has(p.seat))}
                            />
                          </div>
                          <div className="player-box">
                            <div>
                              <span>
                                {p.hero ? "Hero" : p.name.slice(0, 8)}
                              </span>
                              <small>{p.position}</small>
                              {p.seat === h.button && (
                                <b className="dealer-button">{t("D")}</b>
                              )}
                            </div>
                            <strong className="mono">
                              ${money(fromUnits(frame.stacks[p.seat]))}
                            </strong>
                          </div>
                          {(frame.contributions[p.seat] || 0n) > 0n && (
                            <span className="player-contribution mono">
                              ${money(fromUnits(frame.contributions[p.seat]))}
                            </span>
                          )}
                        </div>
                      );
                    })}
                  </div>
                  <div className="action-caption">
                    {last ? (
                      <>
                        <span className="subtle-badge">{t(last.street)}</span>
                        <strong>
                          {last.actor
                            ? h.players.find((p) => p.seat === last.actor)?.hero
                              ? "Hero"
                              : h.players.find((p) => p.seat === last.actor)
                                  ?.position
                            : ""}{" "}
                          {t(ACTIONS[last.kind] || last.kind)}
                        </strong>
                        {Number(last.amount) > 0 && (
                          <span className="mono">
                            $
                            {money(
                              last.kind === "raise" ? last.to : last.amount,
                            )}
                          </span>
                        )}
                        {last.all_in && (
                          <span className="all-in-badge">{t("ALL-IN")}</span>
                        )}
                      </>
                    ) : (
                      <span className="muted">
                        {t("準備回放 · ← → 逐步 · Space 播放")}
                      </span>
                    )}
                  </div>
                  <input
                    className="timeline"
                    aria-label={t("回放進度")}
                    type="range"
                    min="0"
                    max={h.actions.length}
                    value={step}
                    onChange={(e) => go(Number(e.target.value))}
                  />
                  <div className="replay-controls">
                    <button
                      className="icon-button"
                      aria-label={t("回到開始")}
                      onClick={() => go(0)}
                    >
                      <SkipBack size={19} />
                    </button>
                    <button
                      className="icon-button"
                      aria-label={t("上一步")}
                      onClick={() => go(Math.max(0, step - 1))}
                    >
                      <CaretLeft size={20} />
                    </button>
                    <button
                      className="play-button"
                      aria-label={playing ? t("暫停") : t("播放")}
                      onClick={() => {
                        if (step === h.actions.length) setStep(0);
                        setPlaying((p) => !p);
                      }}
                    >
                      {playing ? (
                        <Pause size={21} weight="fill" />
                      ) : (
                        <Play size={21} weight="fill" />
                      )}
                    </button>
                    <button
                      className="icon-button"
                      aria-label={t("下一步")}
                      onClick={() => go(Math.min(h.actions.length, step + 1))}
                    >
                      <CaretRight size={20} />
                    </button>
                    <button
                      className="icon-button"
                      aria-label={t("跳到結束")}
                      onClick={() => go(h.actions.length)}
                    >
                      <SkipForward size={19} />
                    </button>
                    <span className="filter-spacer" />
                    <select
                      aria-label={t("播放速度")}
                      value={speed}
                      onChange={(e) => setSpeed(Number(e.target.value))}
                    >
                      <option value="0.5">0.5×</option>
                      <option value="1">1×</option>
                      <option value="2">2×</option>
                    </select>
                    <span className="mono muted">
                      {step}/{h.actions.length}
                    </span>
                  </div>
                  <div className="street-jumps">
                    <button onClick={() => go(0)}>{t("Preflop")}</button>
                    {h.actions
                      .filter((a) => a.kind === "board")
                      .map((a) => (
                        <button
                          className={
                            frame.street === a.street &&
                            frame.runout === a.runout
                              ? "active"
                              : ""
                          }
                          key={a.seq}
                          onClick={() => go(a.seq + 1)}
                        >
                          {a.runout ? `#${a.runout + 1} ` : ""}
                          {t(a.street)}
                        </button>
                      ))}
                    <button
                      className={reveal ? "active" : ""}
                      aria-pressed={reveal}
                      onClick={() => setReveal((v) => !v)}
                    >
                      <Eye size={14} />
                      {t("已知底牌")}
                    </button>
                  </div>
                  <div className="action-log">
                    {h.actions.map((a) => (
                      <button
                        key={a.seq}
                        className={step === a.seq + 1 ? "active" : ""}
                        onClick={() => go(a.seq + 1)}
                      >
                        <small className="mono">
                          {String(a.seq + 1).padStart(2, "0")}
                        </small>
                        <span>
                          {a.kind === "board"
                            ? `${a.runout ? `#${a.runout + 1} ` : ""}${t(a.street)}`
                            : a.actor === h.hero_seat
                              ? "Hero"
                              : h.players.find((p) => p.seat === a.actor)
                                  ?.position}
                        </span>
                        <strong>{t(ACTIONS[a.kind] || a.kind)}</strong>
                        <span className="mono">
                          {Number(a.amount) > 0
                            ? `$${money(a.kind === "raise" ? a.to : a.amount)}`
                            : a.kind === "show" &&
                                a.actor !== h.hero_seat &&
                                !reveal &&
                                step < a.seq + 1
                              ? "?? ??"
                              : a.cards.join(" ")}
                        </span>
                      </button>
                    ))}
                  </div>
                </>
              )}
              {tab === "raw" && (
                <pre className="raw-hand" tabIndex={0}>
                  {h.raw}
                </pre>
              )}
              {tab === "ledger" && (
                <div className="ledger">
                  <h3>{t("Hero 現金流")}</h3>
                  {[
                    [t("投入"), h.invested],
                    [t("退回未跟注"), h.returned],
                    [t("底池派彩"), h.collected],
                    [t("Cashout 收款"), h.cashout],
                    ["Cashout Risk", h.cashout_risk],
                    [t("牌局淨結果"), h.net],
                  ].map(([k, v]) => (
                    <div key={k}>
                      <span>{k}</span>
                      <strong className="mono">${money(v)}</strong>
                    </div>
                  ))}
                  <h3>{t("底池與全桌費用")}</h3>
                  {h.pots.map((p, i) => (
                    <div key={i}>
                      <span>
                        {t("{{v0}} · {{v1}} 位有資格", {
                          count: p.eligible.length,
                          v0: i
                            ? t("Side pot {{v0}}", { v0: i })
                            : t("Main pot"),
                          v1: p.eligible.length,
                        })}
                      </span>
                      <strong className="mono">${money(p.amount)}</strong>
                    </div>
                  ))}
                  {Object.entries(h.fees).map(([k, v]) => (
                    <div key={k}>
                      <span>{k}</span>
                      <strong className="mono">${money(v)}</strong>
                    </div>
                  ))}
                  <p>
                    {t(
                      "以上費用屬全桌，唔係 Hero 個人 rake；派彩已包含相應扣款。",
                    )}
                  </p>
                  <h3>{t("All-in equity adjustment")}</h3>
                  <div>
                    <span>{t("狀態")}</span>
                    <strong>{t(h.ev_status)}</strong>
                  </div>
                  {h.ev_reason && (
                    <p>{t(EV_REASONS[h.ev_reason] || h.ev_reason)}</p>
                  )}
                  {h.equity !== null && (
                    <div>
                      <span>{t("鎖定時 Equity")}</span>
                      <strong>{(h.equity * 100).toFixed(3)}%</strong>
                    </div>
                  )}
                  {h.adjusted_net !== null && (
                    <div>
                      <span>{t("Adjusted net")}</span>
                      <strong>${money(h.adjusted_net, true)}</strong>
                    </div>
                  )}
                  <p>
                    {t(
                      "根據實際已知底牌，枚舉 all-in 之後嘅發牌結果。不是 GTO 最佳行動或 decision EV。",
                    )}
                  </p>
                </div>
              )}
              <section className="hand-notes">
                <div className="section-title">
                  <h3>{t("你的複盤筆記")}</h3>
                  <span className={`note-status ${clean ? "" : "dirty"}`}>
                    {clean ? t("已儲存") : t("未儲存")}
                  </span>
                </div>
                <Field label={t("筆記")}>
                  <textarea
                    disabled={saving}
                    value={note}
                    onChange={(e) => {
                      setNote(e.target.value);
                      setClean(false);
                    }}
                    placeholder={t("當時嘅 range、下注理由，下次想確認嘅問題…")}
                    rows={3}
                  />
                </Field>
                <Field
                  label={t("標籤")}
                  help={t("以逗號分隔，例如：3-bet pot, river call")}
                >
                  <input
                    disabled={saving}
                    value={tags}
                    onChange={(e) => {
                      setTags(e.target.value);
                      setClean(false);
                    }}
                    placeholder={t("新增標籤")}
                  />
                </Field>
                <label className="check-label">
                  <input
                    type="checkbox"
                    disabled={saving}
                    checked={reviewed}
                    onChange={(e) => {
                      setReviewed(e.target.checked);
                      setClean(false);
                    }}
                  />
                  <CheckCircle size={17} />
                  {t("標記為已複盤")}
                </label>
                {error && <ErrorBanner>{error}</ErrorBanner>}
                <button
                  className="button primary full-width"
                  onClick={() => void save()}
                  disabled={saving}
                >
                  <FloppyDisk size={17} />
                  {saving ? t("儲存中…") : t("儲存筆記與標籤")}
                </button>
              </section>
            </div>
            <div className="replayer-footer">
              <span className="mono">#{h.id}</span>
              <button
                className="text-button"
                onClick={() => {
                  go(0);
                  setTab("replay");
                }}
              >
                <ArrowCounterClockwise size={14} />
                {t("重新回放")}
              </button>
            </div>
          </>
        )}
      </aside>
    </Dialog>
  );
}
