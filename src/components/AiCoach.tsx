import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { api, desktop, errorText } from "../api";
import type { Filter, Profile } from "../types";
import { Card } from "./Replayer";
import { ErrorBanner } from "./UI";
import "./ai-coach.css";

import { AI_MODELS, AI_PROVIDERS as PROVIDERS } from "../ai-models";

type Data = Record<string, any>;
type Evidence = {
  evidence_id: string;
  version: string;
  tool: string;
  args: Data;
  data: Data;
};
export type AiDraft = {
  id: string;
  title: string;
  text: string;
  evidence: string[];
  items: { hand: number; seq: number; prompt: string }[];
  organization?: {
    category: string;
    tags: string[];
    sections: { title: string; start: number; end: number; tags: string[] }[];
  };
};
async function tool(name: string, args: Data = {}): Promise<Evidence> {
  return api({ op: "agent_tool", name, arguments: args, share_notes: false });
}
export function AiCoach({
  filter,
  openHand,
  question,
}: {
  filter: Filter;
  openHand: (id: number, seq?: number) => void;
  question?: string;
}) {
  const { t } = useTranslation();
  const [tab, setTab] = useState("coach"),
    [prompt, setPrompt] = useState(question || "");
  const [provider, setProvider] = useState("openai"),
    [model, setModel] = useState("");
  const [profiles, setProfiles] = useState<Profile[]>([]),
    [profile, setProfile] = useState(""),
    [rakeId, setRakeId] = useState(""),
    [treeId, setTreeId] = useState("");
  const [key, setKey] = useState(""),
    [status, setStatus] = useState<Data>({});
  const [consent, setConsent] = useState(false),
    [notes, setNotes] = useState(false);
  const [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [output, setOutput] = useState("");
  const [history, setHistory] = useState<Data[]>([]),
    [evidence, setEvidence] = useState<Evidence[]>([]);
  const [usage, setUsage] = useState<Data[]>([]),
    [learning, setLearning] = useState<Data>({ drafts: [], attempts: [] });
  const [edit, setEdit] = useState<(AiDraft & { revision: number }) | null>(
    null,
  );
  const [practice, setPractice] = useState<Data | null>(null),
    [answer, setAnswer] = useState("");
  const [spots, setSpots] = useState<Evidence | null>(null),
    [path, setPath] = useState<Data[]>([]);
  const [packs, setPacks] = useState<Data[]>([]),
    [search, setSearch] = useState(""),
    [hits, setHits] = useState<Data[]>([]);
  const [category, setCategory] = useState("");
  const [tag, setTag] = useState("");
  const [organizing, setOrganizing] = useState("");
  const sessionId = useRef(crypto.randomUUID());
  const id = useRef(""),
    text = useRef(""),
    historyRef = useRef<Data[]>([]);
  const [listenerReady, setListenerReady] = useState(false);
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(errorText(e));
    }
  };
  const refresh = async () => {
    setProfiles(await api<Profile[]>({ op: "profiles" }));
    const result = await tool("get_learning_progress");
    setLearning(result.data);
    const ps = await tool("list_strategy_packs");
    setPacks(ps.data as unknown as Data[]);
    if (desktop) setStatus(await invoke("ai_status"));
  };
  useEffect(() => {
    void run(refresh);
  }, []);
  useEffect(() => {
    if (question) setPrompt(question);
  }, [question]);
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<Data>("riverlens-ai", ({ payload: e }) => {
      if (e.id !== id.current) return;
      if (e.type === "text") {
        text.current += e.text;
        setOutput(text.current);
      }
      if (e.type === "evidence") setEvidence((v) => [...v, e.evidence]);
      if (e.type === "warning") setError(e.message);
      if (e.type === "usage") setUsage((v) => [...v, e.usage]);
      if (e.type === "error") {
        setError(e.message);
        setBusy(false);
      }
      if (e.type === "done") {
        setBusy(false);
        const h = [
          ...historyRef.current,
          { role: "assistant", text: text.current },
        ];
        setHistory(h);
        historyRef.current = h;
        void refresh().catch((e) => setError(errorText(e)));
      }
      if (e.type === "draft")
        void refresh().catch((e) => setError(errorText(e)));
    }).then((fn) => {
      if (disposed) fn();
      else {
        unlisten = fn;
        setListenerReady(true);
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
      if (id.current) void invoke("ai_cancel", { id: id.current });
    };
  }, []);
  function resetConversation() {
    sessionId.current = crypto.randomUUID();
    setHistory([]);
    historyRef.current = [];
    setOutput("");
    setEvidence([]);
  }
  function changeProvider(value: string) {
    resetConversation();
    setProvider(value);
    setModel("");
  }
  const modelOptions = AI_MODELS[provider] ?? [];
  const validModel = modelOptions.includes(model);
  async function start() {
    if (!consent || !validModel || !prompt.trim() || !listenerReady) return;
    id.current = crypto.randomUUID();
    text.current = "";
    setOutput("");
    setEvidence([]);
    setUsage([]);
    setBusy(true);
    setError("");
    const previous = history.slice(-18);
    historyRef.current = [...previous, { role: "user", text: prompt }];
    try {
      await invoke("ai_chat", {
        chat: {
          id: id.current,
          session_id: sessionId.current,
          provider,
          model: model.trim(),
          prompt,
          filter,
          share_notes: notes,
          history: previous,
        },
      });
    } catch (e) {
      setBusy(false);
      setError(errorText(e));
    }
  }
  async function organizeDraft(draftId: string) {
    setOrganizing(draftId);
    try {
      await invoke("ai_organize", { id: draftId, provider, model });
      await refresh();
    } finally {
      setOrganizing("");
    }
  }
  async function localReport() {
    const e = await tool("query_stats", { filter, group: "position" });
    setEvidence([e]);
    const draft: AiDraft = {
      id: crypto.randomUUID(),
      title: t("目前篩選統計"),
      text: t("引擎統計；尚未加入策略判斷。"),
      evidence: [e.evidence_id],
      items: [],
    };
    await tool("create_report_draft", { draft, version: e.version });
    await refresh();
  }
  async function find(cursor?: unknown) {
    const e = await tool("find_spots", {
      filter,
      path,
      ...(cursor ? { cursor, version: spots?.version } : {}),
      limit: 60,
    });
    setSpots(e);
  }
  async function review(accept: boolean) {
    if (!edit) return;
    const { revision, ...draft } = edit;
    await api({ op: "agent_review", id: draft.id, revision, draft, accept });
    setEdit(null);
    await refresh();
  }
  async function practiceItem(
    draft: string,
    index: number,
    submitted?: string,
  ) {
    const result: Data = await api({
      op: "agent_practice",
      id: draft,
      item: index,
      answer: submitted ?? null,
    });
    setPractice({ ...result, draft, index });
    setAnswer("");
    if (submitted) await refresh();
  }
  const tabs = [
    ["coach", "AI 教練"],
    ["spots", "局面探索"],
    ["library", "學習資料"],
    ["connect", "AI 連接"],
  ] as const;
  return (
    <section className="ai-workspace">
      <nav className="ai-tabs" aria-label={t("AI 工作區")}>
        {tabs.map(([key, label]) => (
          <button
            key={key}
            className={tab === key ? "button primary" : "button"}
            aria-pressed={tab === key}
            onClick={() => setTab(key)}
          >
            {t(label)}
          </button>
        ))}
      </nav>
      {error && <ErrorBanner>{error}</ErrorBanner>}
      {tab === "coach" && (
        <>
          <div className="ai-panel">
            <h2>{t("從你的牌局開始")}</h2>
            <p>{t("先查證據，再給建議。報告與練習以草稿儲存。")}</p>
            <div className="ai-row">
              <label>
                {t("模型供應商")}
                <select
                  value={provider}
                  disabled={busy}
                  onChange={(e) => changeProvider(e.target.value)}
                >
                  {PROVIDERS.map((p) => (
                    <option key={p}>{p}</option>
                  ))}
                </select>
              </label>
              <label>
                {t("模型 ID")}
                <select
                  value={model}
                  disabled={busy}
                  onChange={(e) => {
                    resetConversation();
                    setModel(e.target.value);
                  }}
                >
                  <option value="" disabled>
                    {t("選擇模型")}
                  </option>
                  {modelOptions.map((id) => (
                    <option key={id} value={id}>
                      {id}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <label className="ai-label">
              {t("想改善哪個局面？")}
              <textarea
                rows={3}
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
              />
            </label>
            <div className="ai-row">
              {[
                "分析目前篩選，找出值得複盤的局面。",
                "根據最近的複盤，建立練習草稿。",
                "比較相同條件下前後期的表現。",
              ].map((q) => (
                <button
                  className="button"
                  key={q}
                  onClick={() => setPrompt(t(q))}
                >
                  {t(q)}
                </button>
              ))}
            </div>
            <label className="ai-check">
              <input
                type="checkbox"
                checked={consent}
                onChange={(e) => setConsent(e.target.checked)}
              />
              {t("允許將問題與所需匿名化資料傳送至所選模型供應商。")}
            </label>
            <label className="ai-check">
              <input
                type="checkbox"
                checked={notes}
                disabled={busy}
                onChange={(e) => {
                  resetConversation();
                  setNotes(e.target.checked);
                }}
              />
              {t("另外分享筆記與標籤內容")}
            </label>
            <div className="ai-row">
              <button
                className="button primary"
                disabled={
                  !desktop ||
                  busy ||
                  !consent ||
                  !validModel ||
                  !prompt.trim() ||
                  !listenerReady
                }
                onClick={() => void start()}
              >
                {t("開始分析")}
              </button>
              {busy && (
                <button
                  className="button"
                  onClick={() =>
                    void run(async () => {
                      await invoke("ai_cancel", { id: id.current });
                      id.current = "";
                      setBusy(false);
                    })
                  }
                >
                  {t("取消分析")}
                </button>
              )}
              <button
                className="button"
                disabled={busy}
                onClick={() => void run(localReport)}
              >
                {t("建立本機統計草稿")}
              </button>
              <button
                className="button"
                disabled={busy}
                onClick={() => {
                  resetConversation();
                }}
              >
                {t("新對話")}
              </button>
            </div>
            {!desktop && (
              <p>{t("雲端教練與憑證設定只在桌面版提供。本機分析仍可使用。")}</p>
            )}
            <p className="muted">
              {t("每次最多 12 輪工具查詢；資料變更時停止，避免混用樣本。")}
            </p>
          </div>
          {(output || busy) && (
            <article className="ai-panel" aria-live="polite">
              <h3>{busy ? t("分析中") : t("教練解讀")}</h3>
              <div className="ai-prose">{output}</div>
            </article>
          )}
          {usage.length > 0 && (
            <p>
              {t("已完成模型回合")}: {usage.length} · {t("Token 用量")}:{" "}
              {usage.reduce(
                (s, u) =>
                  s +
                  Number(
                    u.total_tokens ??
                      u.totalTokenCount ??
                      (u.input_tokens || 0) + (u.output_tokens || 0),
                  ),
                0,
              )}
            </p>
          )}
          {evidence.map((e) => (
            <EvidenceCard key={e.evidence_id} e={e} openHand={openHand} />
          ))}
        </>
      )}
      {tab === "spots" && (
        <div className="ai-panel">
          <h2>{t("依序搜尋行動")}</h2>
          <p>{t("沿用頁面篩選；以下行動按先後次序匹配，中間可有其他行動。")}</p>
          {path.map((step, i) => (
            <div className="ai-row" key={i}>
              <label>
                {t("街道")}
                <select
                  value={step.street}
                  onChange={(e) =>
                    setPath((p) =>
                      p.map((s, j) =>
                        i === j ? { ...s, street: e.target.value } : s,
                      ),
                    )
                  }
                >
                  {["preflop", "flop", "turn", "river"].map((x) => (
                    <option key={x}>{x}</option>
                  ))}
                </select>
              </label>
              <label>
                {t("位置")}
                <select
                  value={step.position}
                  onChange={(e) =>
                    setPath((p) =>
                      p.map((s, j) =>
                        i === j ? { ...s, position: e.target.value } : s,
                      ),
                    )
                  }
                >
                  {["UTG", "HJ", "CO", "BTN", "SB", "BB"].map((x) => (
                    <option key={x}>{x}</option>
                  ))}
                </select>
              </label>
              <label>
                {t("行動")}
                <select
                  value={step.kind}
                  onChange={(e) =>
                    setPath((p) =>
                      p.map((s, j) =>
                        i === j ? { ...s, kind: e.target.value } : s,
                      ),
                    )
                  }
                >
                  {["fold", "check", "call", "bet", "raise"].map((x) => (
                    <option key={x}>{x}</option>
                  ))}
                </select>
              </label>
              <button
                className="button"
                onClick={() => setPath((p) => p.filter((_, j) => j !== i))}
              >
                {t("移除")}
              </button>
            </div>
          ))}
          <div className="ai-row">
            <button
              className="button"
              disabled={path.length >= 8}
              onClick={() =>
                setPath((p) => [
                  ...p,
                  { street: "preflop", position: "BTN", kind: "raise" },
                ])
              }
            >
              {t("新增行動")}
            </button>
            <button
              className="button primary"
              onClick={() => void run(() => find())}
            >
              {t("搜尋局面")}
            </button>
          </div>
          {spots && (
            <>
              <EvidenceCard e={spots} openHand={openHand} />
              {spots.data.next_cursor && (
                <button
                  className="button"
                  onClick={() => void run(() => find(spots.data.next_cursor))}
                >
                  {t("下一頁")}
                </button>
              )}
            </>
          )}
        </div>
      )}
      {tab === "library" && (
        <>
          <div className="ai-panel">
            <div className="ai-row">
              <h2>{t("學習資料")}</h2>
              <button className="button" onClick={() => void run(refresh)}>
                {t("重新整理")}
              </button>
            </div>
            <label>
              {t("搜尋報告")}
              <input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </label>
            <button
              className="button"
              onClick={() =>
                void run(async () =>
                  setHits(
                    (await tool("search_learning", { query: search })).data
                      .rows,
                  ),
                )
              }
            >
              {t("搜尋")}
            </button>
            {hits.map((h, i) => (
              <p key={i}>{h.text}</p>
            ))}
            <p>
              {t("練習作答次數")}: {learning.attempts.length}
            </p>
            {learning.drafts.length === 0 && (
              <p>{t("尚未建立草稿。先分析一個局面。")}</p>
            )}
            <p>
              {t(
                "新 AI 回答會自動分類、加 tags 及按主題分段。分類結果可供參考，原文與證據保持完整。",
              )}
            </p>
            <div className="ai-row">
              <label>
                {t("分類")}
                <select
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                >
                  <option value="">{t("全部")}</option>
                  {[
                    ...new Set<string>(
                      learning.drafts.map(
                        (d: Data) =>
                          d.body.organization?.category || t("未分類"),
                      ),
                    ),
                  ]
                    .sort()
                    .map((c) => (
                      <option key={c}>{c}</option>
                    ))}
                </select>
              </label>
              <label>
                Tag
                <select value={tag} onChange={(e) => setTag(e.target.value)}>
                  <option value="">{t("全部")}</option>
                  {[
                    ...new Set<string>(
                      learning.drafts.flatMap(
                        (d: Data) => d.body.organization?.tags || [],
                      ),
                    ),
                  ]
                    .sort()
                    .map((v) => (
                      <option key={v}>{v}</option>
                    ))}
                </select>
              </label>
            </div>
            <p>
              {t(
                "重新分類使用 AI 教練所選供應商及模型；需先同意分享，會傳送該份原文。",
              )}
            </p>
            {learning.drafts
              .filter(
                (d: Data) =>
                  (!category ||
                    (d.body.organization?.category || t("未分類")) ===
                      category) &&
                  (!tag || d.body.organization?.tags?.includes(tag)),
              )
              .map((d: Data) => (
                <article key={d.id} className="ai-draft">
                  <h3>{d.title}</h3>
                  <p>
                    {d.status === "accepted" ? t("已接受") : t("草稿")} ·{" "}
                    {d.body.items.length} {t("練習題")}
                  </p>
                  <div className="ai-row">
                    <strong>
                      {d.body.organization?.category || t("未分類")}
                    </strong>
                    {d.body.organization?.tags.map((v: string) => (
                      <button
                        key={v}
                        className="button"
                        onClick={() => setTag(v)}
                      >
                        #{v}
                      </button>
                    ))}
                  </div>
                  {d.body.organization?.sections.length ? (
                    d.body.organization.sections.map(
                      (section: Data, i: number) => (
                        <details key={i} className="ai-section" open={i === 0}>
                          <summary>{section.title}</summary>
                          <small>{section.tags.join(" · ")}</small>
                          <div className="ai-prose">
                            {d.body.text
                              .split("\n")
                              .slice(section.start, section.end)
                              .join("\n")}
                          </div>
                        </details>
                      ),
                    )
                  ) : (
                    <div className="ai-prose">{d.body.text}</div>
                  )}
                  <button
                    className="button"
                    disabled={
                      !desktop ||
                      !consent ||
                      !validModel ||
                      busy ||
                      !!organizing
                    }
                    onClick={() => void run(() => organizeDraft(d.id))}
                  >
                    {organizing === d.id
                      ? t("分類中…")
                      : t("AI 自動分類與分段")}
                  </button>
                  <button
                    className="button"
                    onClick={() => setEdit({ ...d.body, revision: d.revision })}
                  >
                    {t("編輯與確認")}
                  </button>
                  <details className="ai-section">
                    <summary>
                      {t("來源證據")} · {d.body.evidence.length}
                    </summary>
                    <div className="ai-row">
                      {d.body.evidence.map((evidenceId: string) => (
                        <button
                          className="button"
                          key={evidenceId}
                          onClick={() =>
                            void run(async () => {
                              const saved = await api<Evidence>({
                                op: "agent_evidence",
                                id: evidenceId,
                              });
                              if (
                                !saved ||
                                saved.evidence_id !== evidenceId ||
                                !("data" in saved)
                              )
                                throw new Error(
                                  t("證據格式不完整，請重新整理。"),
                                );
                              setEvidence([saved]);
                              setTab("coach");
                            })
                          }
                        >
                          {t("查看證據")} {evidenceId.slice(2, 10)}
                        </button>
                      ))}
                    </div>
                  </details>
                  {d.status === "accepted" &&
                    d.body.items.map((_: unknown, i: number) => (
                      <button
                        className="button"
                        key={i}
                        onClick={() => void run(() => practiceItem(d.id, i))}
                      >
                        {t("練習")} {i + 1}
                      </button>
                    ))}
                </article>
              ))}
          </div>
          {edit && (
            <section className="ai-panel">
              <h3>{t("編輯草稿")}</h3>
              <label>
                {t("標題")}
                <input
                  value={edit.title}
                  onChange={(e) => setEdit({ ...edit, title: e.target.value })}
                />
              </label>
              <label className="ai-label">
                {t("內容")}
                <textarea
                  rows={8}
                  value={edit.text}
                  onChange={(e) =>
                    setEdit({
                      ...edit,
                      text: e.target.value,
                      organization: undefined,
                    })
                  }
                />
              </label>
              <div className="ai-row">
                <button
                  className="button"
                  onClick={() => void run(() => review(false))}
                >
                  {t("儲存草稿")}
                </button>
                <button
                  className="button primary"
                  onClick={() => void run(() => review(true))}
                >
                  {t("接受並儲存")}
                </button>
                <button className="button" onClick={() => setEdit(null)}>
                  {t("取消")}
                </button>
              </div>
            </section>
          )}
          {practice && (
            <section className="ai-panel">
              <h3>{t("輪到你決定")}</h3>
              <Decision data={practice.context} />
              {!practice.actual_action ? (
                <>
                  <label className="ai-label">
                    {t("你的行動與理由")}
                    <textarea
                      value={answer}
                      onChange={(e) => setAnswer(e.target.value)}
                    />
                  </label>
                  <button
                    className="button primary"
                    disabled={!answer.trim()}
                    onClick={() =>
                      void run(() =>
                        practiceItem(practice.draft, practice.index, answer),
                      )
                    }
                  >
                    {t("提交後查看實際行動")}
                  </button>
                </>
              ) : (
                <>
                  <p>
                    {t("實際行動")}: {practice.actual_action.kind}
                  </p>
                  <p>
                    {practice.evaluation === "exact_preflop"
                      ? t("此題與驗證策略精確匹配。")
                      : t("實際行動不代表最佳解；此題採自評。")}
                  </p>
                  {practice.strategy && (
                    <>
                      <p>
                        {t("所選行動頻率")}:{" "}
                        {practice.answer_frequency === null
                          ? "—"
                          : (practice.answer_frequency * 100).toFixed(1) + "%"}
                      </p>
                      {practice.strategy.frequencies.map((a: Data) => (
                        <p key={a.action}>
                          {a.action}: {(a.frequency * 100).toFixed(1)}%
                        </p>
                      ))}
                    </>
                  )}
                  <button
                    className="button"
                    onClick={() =>
                      openHand(practice.context.hand, practice.context.seq)
                    }
                  >
                    {t("打開回放")}
                  </button>
                </>
              )}
              <button className="button" onClick={() => setPractice(null)}>
                {t("關閉練習")}
              </button>
            </section>
          )}
        </>
      )}
      {tab === "connect" && (
        <div className="ai-panel">
          <h2>{t("模型與外部 Agent")}</h2>
          <p>{t("API key 存於系統憑證庫，不包含在資料庫備份。")}</p>
          <label>
            {t("模型供應商")}
            <select
              value={provider}
              disabled={busy}
              onChange={(e) => changeProvider(e.target.value)}
            >
              {PROVIDERS.map((x) => (
                <option key={x}>{x}</option>
              ))}
            </select>
          </label>
          <p>
            {status.keys?.[provider]
              ? t("已儲存 API key")
              : t("尚未設定 API key")}
          </p>
          <label>
            {t("API key")}
            <input
              type="password"
              autoComplete="off"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
          </label>
          <button
            className="button"
            disabled={!desktop || !key}
            onClick={() =>
              void run(async () => {
                await invoke("ai_key", { provider, key });
                setKey("");
                await refresh();
              })
            }
          >
            {t("儲存 API key")}
          </button>
          <button
            className="button"
            disabled={!desktop}
            onClick={() =>
              void run(async () => {
                await invoke("ai_key", { provider, key: "" });
                setKey("");
                await refresh();
              })
            }
          >
            {t("移除 API key")}
          </button>
          <h3>{t("本機 MCP")}</h3>
          <p>
            {t(
              "外部 agent 可能把讀取結果傳至其模型供應商。只在接受此範圍後啟用。",
            )}
          </p>
          <label className="ai-check">
            <input
              type="checkbox"
              checked={notes}
              disabled={busy}
              onChange={(e) => {
                resetConversation();
                setNotes(e.target.checked);
              }}
            />
            {t("另外分享筆記與標籤內容")}
          </label>
          <button
            className="button"
            disabled={!desktop}
            onClick={() =>
              void run(async () => {
                setStatus({
                  ...status,
                  ...(await invoke<Data>("ai_connect", {
                    enabled: !status.enabled,
                    notes,
                  })),
                });
              })
            }
          >
            {status.enabled ? t("撤銷連接") : t("啟用本機連接")}
          </button>
          {status.enabled && (
            <>
              <p>
                {t(
                  "複製到支援 stdio MCP 的客戶端；重新開啟 RiverLens 後需更新連接設定。",
                )}
              </p>
              <pre className="ai-config">
                {JSON.stringify(status.config, null, 2)}
              </pre>
            </>
          )}
          <h3>{t("翻前策略包")}</h3>
          <p>
            {t("匯入既有 Nexus manifest。未驗證權重及 rake 配置只供參考。")}
          </p>
          <button
            className="button"
            disabled={!desktop}
            onClick={() =>
              void run(async () => {
                const file = await open({
                  multiple: false,
                  filters: [{ name: "JSON", extensions: ["json"] }],
                });
                if (typeof file === "string") {
                  await api({ op: "agent_import_strategy", path: file });
                  await refresh();
                }
              })
            }
          >
            {t("匯入策略包")}
          </button>
          <h3>{t("確認牌局來源配置")}</h3>
          <p>
            {t(
              "僅在已核實時填寫策略包的 rake 與 tree 識別值；此設定不會把參考包升級為精確策略。",
            )}
          </p>
          <div className="ai-row">
            <label>
              {t("資料來源")}
              <select
                value={profile}
                onChange={(e) => setProfile(e.target.value)}
              >
                <option value="">{t("選擇資料來源")}</option>
                {profiles.map((p) => (
                  <option value={p.id} key={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Rake ID
              <input
                value={rakeId}
                onChange={(e) => setRakeId(e.target.value)}
              />
            </label>
            <label>
              Tree ID
              <input
                value={treeId}
                onChange={(e) => setTreeId(e.target.value)}
              />
            </label>
            <button
              className="button"
              disabled={!profile || !rakeId || !treeId}
              onClick={() =>
                void run(async () => {
                  await api({
                    op: "agent_strategy_config",
                    profile,
                    rake_id: rakeId,
                    tree_id: treeId,
                  });
                  await refresh();
                })
              }
            >
              {t("儲存已核實配置")}
            </button>
          </div>
          {packs.map((p) => (
            <p key={p.id}>
              {p.id} · {p.node_count} {t("節點")} · {p.invalid_nodes}{" "}
              {t("權重待核實")} ·{" "}
              {p.validation === "reference_only"
                ? t("僅供參考")
                : t("條件頻率已驗證")}
            </p>
          ))}
        </div>
      )}
    </section>
  );
}
export function EvidenceCard({
  e,
  openHand,
}: {
  e: Evidence;
  openHand: (id: number, seq?: number) => void;
}) {
  const { t } = useTranslation();
  const d = e.data;
  const object = d && !Array.isArray(d) && typeof d === "object" ? d : {};
  const stats = Array.isArray(object.stats) ? object.stats : [];
  const hands =
    e.tool === "find_spots" && Array.isArray(object.rows) ? object.rows : [];
  const hand =
    e.tool === "get_hand"
      ? e.args?.id
      : typeof object.hand === "number"
        ? object.hand
        : object.hand?.id;
  const scalar = (value: unknown) =>
    typeof value === "string" || typeof value === "number"
      ? String(value)
      : "—";
  return (
    <article className="ai-panel">
      <h3>
        {t("資料證據")} · {e.tool}
      </h3>
      <small>
        {e.evidence_id} · {e.version}
      </small>
      {typeof object.hands === "number" && (
        <p>
          {t("手牌")}: {object.hands}
        </p>
      )}
      {!!stats.length && (
        <table>
          <thead>
            <tr>
              <th>{t("指標")}</th>
              <th>{t("次數／機會")}</th>
              <th>%</th>
            </tr>
          </thead>
          <tbody>
            {stats
              .filter((s: unknown) => s && typeof s === "object")
              .map((s: Data, i: number) => (
                <tr key={i}>
                  <td>{scalar(s.label || s.id)}</td>
                  <td>
                    {scalar(s.numerator)} / {scalar(s.opportunities)}
                  </td>
                  <td>
                    {typeof s.value === "number" ? s.value.toFixed(1) : "—"}
                  </td>
                </tr>
              ))}
          </tbody>
        </table>
      )}
      <div className="ai-row">
        {hands
          .filter((r: Data) => r && Number.isSafeInteger(r.id) && r.id > 0)
          .map((r: Data, i: number) => (
            <button className="button" key={i} onClick={() => openHand(r.id)}>
              {scalar(r.position)} {scalar(r.hand_class)} #{r.id}
            </button>
          ))}
      </div>
      {Number.isSafeInteger(hand) && hand > 0 && (
        <button
          className="button"
          onClick={() =>
            openHand(
              hand,
              Number.isSafeInteger(object.seq) ? object.seq : undefined,
            )
          }
        >
          {t("打開回放")} #{hand}
        </button>
      )}
      {object.coverage && (
        <p>
          {t("手牌")}: {scalar(object.coverage.hands)} · {t("有效")}:{" "}
          {scalar(object.coverage.valid)}
        </p>
      )}
      <details open>
        <summary>{t("完整證據內容")}</summary>
        <pre className="ai-config">
          {JSON.stringify(d, null, 2) ?? t("沒有證據內容")}
        </pre>
      </details>
      <details>
        <summary>{t("查詢條件")}</summary>
        <pre className="ai-config">{JSON.stringify(e.args, null, 2)}</pre>
      </details>
    </article>
  );
}
function Decision({ data }: { data: Data }) {
  const { t } = useTranslation();
  return (
    <div>
      <p>
        {data.position} · {data.street}
      </p>
      <div className="ai-row">
        {data.players
          ?.filter((p: Data) => p.hero)
          .flatMap((p: Data) => p.cards)
          .map((c: string) => (
            <Card key={c} card={c} />
          ))}
      </div>
      <table>
        <thead>
          <tr>
            <th>{t("位置")}</th>
            <th>{t("行動")}</th>
            <th>{t("牌面")}</th>
          </tr>
        </thead>
        <tbody>
          {data.actions?.map((a: Data) => (
            <tr key={a.seq}>
              <td>
                {data.players?.find((p: Data) => p.seat === a.actor)
                  ?.position || "—"}
              </td>
              <td>
                {a.kind} {Number(a.amount) / Number(data.bb) || ""}{" "}
                {Number(a.amount) !== 0 ? "bb" : ""}
              </td>
              <td>{a.kind === "board" ? a.cards?.join(" ") : ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
