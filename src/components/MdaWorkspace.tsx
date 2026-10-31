import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  api,
  chooseInput,
  chooseOutput,
  desktop,
  errorText,
  useRpc,
} from "../api";
import type {
  MdaCatalog,
  MdaFilter,
  MdaJob,
  MdaRequest,
  MdaSpot,
  MdaStat,
} from "../mda-types";
import { number } from "../format";
import { ErrorBanner, Field, Skeleton } from "./UI";
import { AiCoach } from "./AiCoach";
import { Replayer } from "./Replayer";
import { MdaWizard } from "./MdaWizard";
import "./mda.css";
const mda = <T,>(request: MdaRequest) => api<T>({ op: "mda", request });
type Result<T> = {
  version: string;
  data: {
    rows: T[];
    next_cursor: string | null;
    river_bluff_model?: { pure_bluff_break_even_fold_pct: number };
  };
};
const POSITIONS = ["UTG", "HJ", "CO", "BTN", "SB", "BB", "BTN/SB"];
export function MdaWorkspace() {
  const [revision, setRevision] = useState(0),
    [selected, setSelected] = useState("");
  const [name, setName] = useState("GG 一般桌牌池"),
    [timezone, setTimezone] = useState("UTC"),
    [source, setSource] = useState("");
  const [tab, setTab] = useState("stats"),
    [filter, setFilter] = useState<MdaFilter>({ hu_postflop: true }),
    [group, setGroup] = useState("position");
  const [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [cursor, setCursor] = useState<string | undefined>();
  const [replay, setReplay] = useState<{
      hand_key: string;
      seq?: number;
    } | null>(null),
    [question, setQuestion] = useState("");
  const [issueRaw, setIssueRaw] = useState("");
  const [issueBefore, setIssueBefore] = useState<number | undefined>();
  const list = useRpc<{ datasets: MdaCatalog[] }>(
    { op: "mda", request: { op: "list" } },
    revision,
  );
  const catalog = list.data?.datasets.find((c) => c.dataset.id === selected);
  const jobs = useRpc<{ jobs: MdaJob[]; running: boolean }>(
    selected
      ? { op: "mda", request: { op: "jobs", dataset_id: selected } }
      : null,
    revision,
  );
  const issues = useRpc<{
    rows: { id: number; hand_id: string; code: string; source: string }[];
    next_cursor: number | null;
  }>(
    selected && tab === "imports"
      ? {
          op: "mda",
          request: { op: "issues", dataset_id: selected, before: issueBefore },
        }
      : null,
    revision,
  );
  const toolName =
    tab === "candidates"
      ? "find_mda_candidates"
      : tab === "spots"
        ? "find_mda_spots"
        : "query_mda_stats";
  const query = useRpc<Result<MdaStat | MdaSpot>>(
    catalog && ["stats", "candidates", "spots"].includes(tab)
      ? {
          op: "mda",
          request: {
            op: "query",
            dataset_id: selected,
            name: toolName,
            arguments: {
              version: catalog.version,
              filter,
              ...(tab !== "spots" ? { group } : {}),
              limit: 50,
              ...(cursor ? { cursor } : {}),
            },
          },
        }
      : null,
  );
  useEffect(() => {
    if (!selected && list.data?.datasets.length)
      setSelected(list.data.datasets[0].dataset.id);
  }, [selected, list.data]);
  useEffect(() => {
    setCursor(undefined);
  }, [selected, tab, filter, group]);
  useEffect(() => {
    setIssueBefore(undefined);
    setIssueRaw("");
  }, [selected]);
  useEffect(() => {
    if (!jobs.data?.running) return;
    const timer = setInterval(() => setRevision((v) => v + 1), 2500);
    return () => clearInterval(timer);
  }, [jobs.data?.running]);
  const refresh = () => setRevision((v) => v + 1);
  async function run(action: () => Promise<void>) {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await action();
      refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }
  async function importFiles(folder = false) {
    const paths = await chooseInput(folder);
    if (paths.length) {
      await mda({ op: "start_import", dataset_id: selected, paths });
      setTab("imports");
    }
  }
  function update(patch: Partial<MdaFilter>) {
    setFilter((f) => ({ ...f, ...patch }));
  }
  function rowContext(row: MdaStat): Partial<MdaFilter> {
    const grouped: Partial<MdaFilter> = !row.context
      ? {}
      : group === "position"
        ? { position: row.context }
        : group === "opponent"
          ? { opponent: row.context }
          : group === "pot_type"
            ? { pot_type: row.context }
            : group === "texture"
              ? { texture: row.context }
              : group === "players"
                ? { players: Number(row.context) }
                : {};
    return {
      metric: row.metric,
      street: row.street,
      bb_units: row.bb_units,
      ...grouped,
    };
  }
  const context = catalog
    ? {
        dataset_id: selected,
        version: catalog.version,
        filter,
        cloud_enabled: catalog.dataset.cloud_enabled,
      }
    : undefined;
  return (
    <section className="mda-workspace">
      {(error || list.error || jobs.error || query.error || catalog?.error) && (
        <ErrorBanner>
          {error || list.error || jobs.error || query.error || catalog?.error}
        </ErrorBanner>
      )}
      <div className="ai-panel mda-datasets">
        <div className="section-title">
          <h2>群體資料集</h2>
          <span>GG Regular · NLH 6-max · USD</span>
        </div>
        <div className="mda-controls">
          <Field label="資料集">
            <select
              aria-label="MDA 資料集"
              value={selected}
              onChange={(e) => {
                setSelected(e.target.value);
                setReplay(null);
              }}
            >
              <option value="">選擇資料集</option>
              {list.data?.datasets.map((c) => (
                <option key={c.dataset.id} value={c.dataset.id}>
                  {c.dataset.name} · {number(c.hands)} 手
                </option>
              ))}
            </select>
          </Field>
          <button
            className="button"
            disabled={!selected || busy || !desktop || jobs.data?.running}
            onClick={() => void run(() => importFiles())}
          >
            匯入 TXT／ZIP
          </button>
          <button
            className="button"
            disabled={!selected || busy || !desktop || jobs.data?.running}
            onClick={() => void run(() => importFiles(true))}
          >
            匯入資料夾
          </button>
          <button
            className="button"
            disabled={!selected || busy || !desktop || jobs.data?.running}
            onClick={() =>
              void run(async () => {
                const path = await chooseOutput(
                  "duckdb",
                  "riverlens-mda-backup.duckdb",
                );
                if (path)
                  await mda({ op: "backup", dataset_id: selected, path });
              })
            }
          >
            備份資料集
          </button>
          <button
            className="button"
            disabled={busy || !desktop}
            onClick={() =>
              void run(async () => {
                const path = await open({
                  multiple: false,
                  filters: [{ name: "MDA 備份", extensions: ["duckdb"] }],
                });
                if (typeof path === "string") {
                  const c = await mda<MdaCatalog>({ op: "restore", path });
                  setSelected(c.dataset.id);
                }
              })
            }
          >
            還原為新資料集
          </button>
        </div>
        <details>
          <summary>新增獨立資料集</summary>
          <div className="mda-controls">
            <Field label="名稱">
              <input
                value={name}
                maxLength={200}
                onChange={(e) => setName(e.target.value)}
              />
            </Field>
            <Field label="來源／牌池">
              <input
                value={source}
                maxLength={500}
                onChange={(e) => setSource(e.target.value)}
                placeholder="例如：自有研究資料，NL25"
              />
            </Field>
            <Field label="牌譜來源時區">
              <input
                value={timezone}
                onChange={(e) => setTimezone(e.target.value)}
                list="mda-timezones"
              />
            </Field>
            <datalist id="mda-timezones">
              <option>UTC</option>
              <option>Asia/Hong_Kong</option>
            </datalist>
            <button
              className="button primary"
              disabled={busy || !name.trim()}
              onClick={() =>
                void run(async () => {
                  const c = await mda<MdaCatalog>({
                    op: "create",
                    name,
                    timezone,
                    source,
                  });
                  setSelected(c.dataset.id);
                })
              }
            >
              建立資料集
            </button>
          </div>
          <p>
            時區用於解析來源時間。各級別獨立統計；未知底牌保持未知。HHDealer
            相容性待真實樣本驗收。
          </p>
        </details>
        {catalog && (
          <p>
            {number(catalog.hands)} 有效手牌 · {number(catalog.quarantined)}{" "}
            隔離 · {catalog.dataset.timezone} ·{" "}
            <small className="mono">{catalog.version}</small>
          </p>
        )}
      </div>
      <details className="ai-panel">
        <summary>平台限制及資料分享</summary>
        <p>
          GG SEP 第 7／19 條限制收集他人牌譜與系統化收集／分享
          MDA，可能導致停權、封禁或資金沒收。
          <a
            href="https://ggpoker.com/network/security-ecology-policy/"
            target="_blank"
            rel="noreferrer"
          >
            官方政策
          </a>
        </p>
        <p>
          分享只包含群體彙總及少量匿名案例。資料版本改變時，當次 AI 分析停止。
        </p>
        {catalog && (
          <div className="mda-controls">
            {(
              [
                ["cloud_enabled", "允許此資料集供雲端 AI Coach 分析"],
                ["mcp_enabled", "允許此外部 MCP 存取此資料集"],
              ] as const
            ).map(([key, label]) => (
              <label key={key} className="check-label">
                <input
                  type="checkbox"
                  checked={catalog.dataset[key]}
                  disabled={busy}
                  onChange={(e) =>
                    void run(async () => {
                      await mda({
                        op: "share",
                        dataset_id: selected,
                        cloud_enabled:
                          key === "cloud_enabled"
                            ? e.target.checked
                            : catalog.dataset.cloud_enabled,
                        mcp_enabled:
                          key === "mcp_enabled"
                            ? e.target.checked
                            : catalog.dataset.mcp_enabled,
                      });
                    })
                  }
                />
                {label}
              </label>
            ))}
          </div>
        )}
        <p>
          外部 MCP 另需在 AI 連接頁啟用本機連接。資料集 ID：
          <code>{selected || "尚未選擇"}</code>
        </p>
      </details>
      {catalog && (
        <>
          <nav className="ai-tabs" aria-label="MDA 工作區">
            {[
              ["wizard", "研究嚮導"],
              ["stats", "群體統計"],
              ["candidates", "研究候選"],
              ["spots", "手牌回查"],
              ["coach", "AI 剝削分析"],
              ["imports", "匯入與隔離"],
            ].map(([id, label]) => (
              <button
                key={id}
                className={`button ${tab === id ? "primary" : ""}`}
                aria-pressed={tab === id}
                onClick={() => setTab(id)}
              >
                {label}
              </button>
            ))}
          </nav>
          {!["imports", "wizard"].includes(tab) && (
            <div className="ai-panel mda-filters">
              <div className="mda-controls">
                <Field label="級別（BB）">
                  <select
                    value={filter.bb_units || ""}
                    onChange={(e) =>
                      update({
                        bb_units: e.target.value
                          ? Number(e.target.value)
                          : undefined,
                      })
                    }
                  >
                    <option value="">各級別分開顯示</option>
                    {catalog.stakes.map((s) => (
                      <option key={s.bb_units} value={s.bb_units}>
                        ${s.bb_units / 1_000_000}
                      </option>
                    ))}
                  </select>
                </Field>
                {(
                  [
                    ["position", "決策位置", POSITIONS],
                    ["opponent", "進攻者位置", POSITIONS],
                    ["street", "街道", ["preflop", "flop", "turn", "river"]],
                    [
                      "pot_type",
                      "底池",
                      ["unopened", "limped", "SRP", "3-bet", "4-bet+"],
                    ],
                    [
                      "texture",
                      "當刻牌面",
                      ["rainbow", "two-tone", "monotone"],
                    ],
                  ] as const
                ).map(([key, label, options]) => (
                  <Field key={key} label={label}>
                    <select
                      value={filter[key] || ""}
                      onChange={(e) =>
                        update({ [key]: e.target.value || undefined })
                      }
                    >
                      <option value="">全部</option>
                      {options.map((o) => (
                        <option key={o}>{o}</option>
                      ))}
                    </select>
                  </Field>
                ))}
                <Field label="指標">
                  <select
                    value={filter.metric || ""}
                    onChange={(e) =>
                      update({ metric: e.target.value || undefined })
                    }
                  >
                    <option value="">全部指標</option>
                    {catalog.metrics.map((m) => (
                      <option key={m.key} value={m.key}>
                        {m.key}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="分組">
                  <select
                    value={group}
                    onChange={(e) => setGroup(e.target.value)}
                  >
                    {[
                      "all",
                      "position",
                      "opponent",
                      "pot_type",
                      "texture",
                      "players",
                    ].map((g) => (
                      <option key={g}>{g}</option>
                    ))}
                  </select>
                </Field>
              </div>
              <details>
                <summary>日期、有效籌碼及下注尺寸</summary>
                <div className="mda-controls">
                  {(
                    [
                      ["date_from", "來源日期由"],
                      ["date_to", "至"],
                    ] as const
                  ).map(([key, label]) => (
                    <Field key={key} label={label}>
                      <input
                        type="date"
                        value={filter[key] || ""}
                        onChange={(e) =>
                          update({ [key]: e.target.value || undefined })
                        }
                      />
                    </Field>
                  ))}
                  {(
                    [
                      ["effective_min", "剩餘有效 BB ≥"],
                      ["effective_max", "剩餘有效 BB ≤"],
                      ["bet_min", "面對 Bet/P % ≥"],
                      ["bet_max", "面對 Bet/P % ≤"],
                    ] as const
                  ).map(([key, label]) => (
                    <Field key={key} label={label}>
                      <input
                        type="number"
                        min="0"
                        value={filter[key] ?? ""}
                        onChange={(e) =>
                          update({
                            [key]: e.target.value
                              ? Number(e.target.value)
                              : undefined,
                          })
                        }
                      />
                    </Field>
                  ))}
                  <Field label="當刻完整牌面">
                    <input
                      value={filter.board || ""}
                      onChange={(e) =>
                        update({ board: e.target.value || undefined })
                      }
                      placeholder="Ks 7d 2c"
                    />
                  </Field>
                  <Field label="人數">
                    <select
                      value={filter.players || ""}
                      onChange={(e) =>
                        update({
                          players: e.target.value
                            ? Number(e.target.value)
                            : undefined,
                        })
                      }
                    >
                      <option value="">預設 HU 翻後</option>
                      {[2, 3, 4, 5, 6].map((n) => (
                        <option key={n}>{n}</option>
                      ))}
                    </select>
                  </Field>
                  <label className="check-label">
                    <input
                      type="checkbox"
                      checked={filter.hu_postflop !== false}
                      onChange={(e) =>
                        update({ hu_postflop: e.target.checked })
                      }
                    />
                    翻後只看 HU
                  </label>
                  <button
                    className="text-button"
                    onClick={() => setFilter({ hu_postflop: true })}
                  >
                    重設情境
                  </button>
                </div>
              </details>
            </div>
          )}
          {query.loading && <Skeleton rows={4} />}
          {query.data && (
            <div className="ai-panel">
              <p>
                百分比按總分子／合法機會計算。Wilson 95%
                為描述性區間；同玩家的決策不獨立。前後期區間重疊僅表示描述性一致性。
              </p>
              <div className="table-scroll">
                <table className="mda-table">
                  <thead>
                    <tr>
                      {tab === "spots" ? (
                        <>
                          <th>匿名手牌</th>
                          <th>情境</th>
                          <th>當刻牌面</th>
                          <th>行動／尺寸</th>
                        </>
                      ) : (
                        <>
                          <th>情境／指標</th>
                          <th>分子／機會</th>
                          <th>%／Wilson 95%</th>
                          <th>分時段／底牌覆蓋</th>
                          <th>驗證</th>
                        </>
                      )}
                    </tr>
                  </thead>
                  <tbody>
                    {query.data.data.rows.map((r, index) =>
                      "hand_key" in r ? (
                        <tr key={`${r.hand_key}:${r.seq}`}>
                          <td>
                            <button
                              className="text-button mono"
                              onClick={() =>
                                setReplay({ hand_key: r.hand_key, seq: r.seq })
                              }
                            >
                              {r.hand_key.slice(0, 8)} · {r.seq}
                            </button>
                          </td>
                          <td>
                            {r.position} vs {r.opponent} · {r.street} ·{" "}
                            {r.pot_type} · {r.players}人
                          </td>
                          <td>{r.board || "—"}</td>
                          <td>
                            {r.action} · {r.facing_bet_pct?.toFixed(1) ?? "—"}%
                            · {r.effective_bb.toFixed(1)}bb
                          </td>
                        </tr>
                      ) : (
                        <tr key={index}>
                          <td>
                            ${r.bb_units / 1_000_000} · {r.street} · {r.context}
                            <br />
                            <strong>{r.metric}</strong>
                            {r.classification && (
                              <p>
                                {r.classification}
                                <br />
                                {r.adjustment}
                              </p>
                            )}
                          </td>
                          <td>
                            {number(r.numerator)} / {number(r.opportunities)}
                          </td>
                          <td>
                            {r.pct.toFixed(1)}%<br />
                            {r.wilson95[0].toFixed(1)}–
                            {r.wilson95[1].toFixed(1)}%
                          </td>
                          <td>
                            {r.periods
                              .map((p) => `${p.numerator}/${p.opportunities}`)
                              .join(" → ")}
                            <br />
                            {r.period_interval_overlap === null
                              ? "分時段證據不足"
                              : r.period_interval_overlap
                                ? "區間重疊"
                                : "時段不一致"}
                            <br />
                            攤牌底牌{" "}
                            {r.showdown_coverage_pct?.toFixed(1) ?? "—"}%
                          </td>
                          <td>
                            <button
                              className="text-button"
                              onClick={() => {
                                update(rowContext(r));
                                setTab("spots");
                              }}
                            >
                              回查
                            </button>
                            <button
                              className="text-button"
                              onClick={() => {
                                setQuestion(
                                  `分析 ${r.metric}，${r.street}，${r.context}，BB $${r.bb_units / 1_000_000}；說明 sizing、bluff/value/防守方向、例外與匿名證據。`,
                                );
                                update(rowContext(r));
                                setTab("coach");
                              }}
                            >
                              問 AI
                            </button>
                          </td>
                        </tr>
                      ),
                    )}
                  </tbody>
                </table>
              </div>
              {!query.data.data.rows.length && (
                <p>
                  {tab === "candidates"
                    ? "此情境未有至少 200 個合法機會。"
                    : "此情境暫無合法機會。"}
                </p>
              )}
              {query.data.data.next_cursor && (
                <button
                  className="button"
                  onClick={() => setCursor(query.data!.data.next_cursor!)}
                >
                  下一頁
                </button>
              )}
              <p>攤牌樣本有選擇偏差，底牌覆蓋不能代表完整 range。</p>
              {query.data.data.river_bluff_model && (
                <p>
                  固定尺寸 HU river pure-bluff 門檻：
                  {query.data.data.river_bluff_model.pure_bluff_break_even_fold_pct.toFixed(
                    1,
                  )}
                  % fold。假設零攤牌 equity、對手只 fold/call、沒有 rake
                  等成本；不是 action EV。
                </p>
              )}
            </div>
          )}
          {tab === "wizard" && (
            <MdaWizard
              catalog={catalog}
              datasetId={selected}
              startReplay={(hand_key, seq) => setReplay({ hand_key, seq })}
              goCoach={(question) => {
                setQuestion(question);
                setTab("coach");
              }}
            />
          )}
          {tab === "imports" && (
            <div className="ai-panel">
              <h2>匯入進度</h2>
              {jobs.data?.jobs.map((j) => (
                <article key={j.id} className="mda-job">
                  <strong>{j.state}</strong> · {j.source_index}/
                  {j.sources.length} 來源 · {number(j.scanned)} 已讀 ·{" "}
                  {number(j.inserted)} 新增 · {number(j.duplicates)} 重複 ·{" "}
                  {number(j.quarantined)} 隔離 · {number(j.conflicts)} 衝突 ·{" "}
                  {(j.elapsed_ms / 1000).toFixed(1)}秒
                  {j.message && <p>{j.message}</p>}
                  {j.state === "running" ? (
                    <button
                      className="button"
                      onClick={() =>
                        void run(async () => {
                          await mda({ op: "pause", dataset_id: selected });
                        })
                      }
                    >
                      暫停
                    </button>
                  ) : (
                    ["paused", "interrupted", "failed"].includes(j.state) && (
                      <button
                        className="button"
                        disabled={busy || jobs.data?.running}
                        onClick={() =>
                          void run(async () => {
                            await mda({
                              op: "resume",
                              dataset_id: selected,
                              job_id: j.id,
                            });
                          })
                        }
                      >
                        從 checkpoint 續傳
                      </button>
                    )
                  )}
                </article>
              ))}
              <h3>隔離及內容衝突</h3>
              {issues.data?.rows.map((i) => (
                <p key={i.id}>
                  {i.hand_id} · {i.code}{" "}
                  <button
                    className="text-button"
                    onClick={() =>
                      void run(async () => {
                        setIssueRaw(
                          (
                            await mda<{ raw: string }>({
                              op: "issue_raw",
                              dataset_id: selected,
                              id: i.id,
                            })
                          ).raw,
                        );
                      })
                    }
                  >
                    本機原文
                  </button>
                </p>
              ))}
              {issues.data?.rows.length === 100 &&
                issues.data.next_cursor !== null && (
                  <button
                    className="button"
                    onClick={() => {
                      setIssueBefore(issues.data!.next_cursor!);
                      setIssueRaw("");
                    }}
                  >
                    下一頁隔離記錄
                  </button>
                )}
              {issueBefore !== undefined && (
                <button
                  className="button"
                  onClick={() => setIssueBefore(undefined)}
                >
                  最新隔離記錄
                </button>
              )}
              {issueRaw && <pre className="raw-hand">{issueRaw}</pre>}
            </div>
          )}
          {tab === "coach" &&
            (context ? (
              <>
                {!catalog.dataset.cloud_enabled && (
                  <div className="ai-panel">
                    <p>
                      在「平台限制及資料分享」啟用此資料集的雲端 AI Coach
                      後才可分析。 本機學習草稿可在分享關閉時檢視、編輯及回查。
                    </p>
                  </div>
                )}
                <AiCoach
                  key={`${selected}:${catalog.version}:${catalog.dataset.cloud_enabled}:${JSON.stringify(filter)}`}
                  mda={context}
                  filter={{}}
                  openHand={() => {}}
                  openMdaHand={(hand_key, seq) => setReplay({ hand_key, seq })}
                  question={
                    question ||
                    "請找這個情境的群體研究候選，說明如何調整下注尺寸、bluff/value 與防守，附分母、例外及匿名手牌證據。"
                  }
                />
              </>
            ) : (
              <div className="ai-panel">
                <p>
                  在「平台限制及資料分享」明確啟用此資料集的雲端 AI
                  Coach，然後選擇既有 provider／model。
                </p>
              </div>
            ))}
          {replay && context && (
            <Replayer
              id={0}
              population={{
                dataset_id: selected,
                version: context.version,
                hand_key: replay.hand_key,
              }}
              initialStep={replay.seq}
              close={() => setReplay(null)}
              saved={() => {}}
            />
          )}
        </>
      )}
    </section>
  );
}
