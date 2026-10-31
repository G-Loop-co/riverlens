import { useEffect, useState } from "react";
import { useRpc } from "../api";
import type { MdaCatalog, MdaFilter, MdaStat, MdaSpot } from "../mda-types";
import { number } from "../format";
import { Field, Skeleton } from "./UI";
import "./mda.css";

type Result<T> = {
  version: string;
  data: { rows: T[]; next_cursor: string | null };
};

export const PRESETS: {
  label: string;
  metric: string;
  filter: MdaFilter;
}[] = [
  {
    label: "BB 面對 open 是否棄牌過多",
    metric: "blind_fold",
    filter: { street: "preflop", position: "BB" },
  },
  {
    label: "Open 後面對 3-bet 是否棄牌過多",
    metric: "fold_three_bet",
    filter: { street: "preflop" },
  },
  {
    label: "翻牌 C-bet 頻率是否偏低",
    metric: "cbet",
    filter: { street: "flop" },
  },
  {
    label: "面對翻牌 C-bet 是否棄牌過多",
    metric: "fold_cbet",
    filter: { street: "flop" },
  },
  {
    label: "River 面對下注是否棄牌過多",
    metric: "fold_bet",
    filter: { street: "river" },
  },
];

export function composeCoachQuestion(
  hypothesis: string,
  stat: Pick<
    MdaStat,
    | "metric"
    | "context"
    | "bb_units"
    | "street"
    | "numerator"
    | "opportunities"
    | "pct"
    | "wilson95"
    | "period_interval_overlap"
  >,
): string {
  const periods =
    stat.period_interval_overlap === null
      ? "分時段證據不足"
      : stat.period_interval_overlap
        ? "前後期區間重疊"
        : "前後期區間不一致";
  return [
    `研究假設：${hypothesis.trim() || "（未填寫，請先依數據提出可檢驗的假設）"}`,
    `指標：${stat.metric}；情境：${stat.context}；street ${stat.street}；級別 BB $${stat.bb_units / 1_000_000}。`,
    `觀察：${stat.numerator}/${stat.opportunities}（${stat.pct.toFixed(1)}%），Wilson 95% ${stat.wilson95[0].toFixed(1)}–${stat.wilson95[1].toFixed(1)}%；${periods}。`,
    "請評估此假設是否成立，說明調整方向、下注尺寸、bluff/value／防守比例、理由與例外，並附匿名手牌證據；分母與樣本限制請一併說明。",
  ].join("\n");
}

const STEP_TITLES = [
  "定義假設",
  "設定情境",
  "檢視統計",
  "手牌驗證",
  "下一步",
];

/** Documented research-guidance threshold for candidate hypotheses. */
const MIN_RESEARCH_OPPORTUNITIES = 200;

export function MdaWizard({
  catalog,
  datasetId,
  startReplay,
  goCoach,
}: {
  catalog: MdaCatalog;
  datasetId: string;
  startReplay: (hand_key: string, seq?: number) => void;
  goCoach: (question: string) => void;
}) {
  const [step, setStep] = useState(0);
  const [hypothesis, setHypothesis] = useState("");
  const [metric, setMetric] = useState("");
  const [filter, setFilter] = useState<MdaFilter>({ hu_postflop: true });
  const wizardFilter: MdaFilter = { ...filter, metric: metric || undefined };
  const stat = useRpc<Result<MdaStat>>(
    step >= 2 && metric
      ? {
          op: "mda",
          request: {
            op: "query",
            dataset_id: datasetId,
            name: "query_mda_stats",
            arguments: {
              version: catalog.version,
              filter: wizardFilter,
              limit: 10,
            },
          },
        }
      : null,
  );
  const spots = useRpc<Result<MdaSpot>>(
    step >= 3 && metric
      ? {
          op: "mda",
          request: {
            op: "query",
            dataset_id: datasetId,
            name: "find_mda_spots",
            arguments: {
              version: catalog.version,
              filter: wizardFilter,
              limit: 5,
            },
          },
        }
      : null,
  );
  useEffect(() => setStep(0), [catalog.dataset.id]);
  const row = stat.data?.data.rows[0];
  const canNext = step !== 0 || metric !== "";
  const patch = (p: Partial<MdaFilter>) =>
    setFilter((f) => ({ ...f, ...p }));
  return (
    <section className="ai-panel">
      <h2>MDA 研究嚮導</h2>
      <p>
        按統計紀律逐步完成一次群體研究：定義假設 →
        設定情境 → 檢視 Wilson 區間 → 匿名手牌驗證 →
        帶到 AI 剝削分析。缺基線只是研究假設，不是結論。
      </p>
      <ol className="mda-wizard-steps" aria-label="研究步驟">
        {STEP_TITLES.map((title, i) => (
          <li
            key={title}
            className={step === i ? "active" : step > i ? "done" : ""}
            aria-current={step === i ? "step" : undefined}
          >
            {i + 1}. {title}
          </li>
        ))}
      </ol>
      {step === 0 && (
        <div>
          <Field label="常見研究假設">
            <select
              value=""
              onChange={(e) => {
                const preset = PRESETS.find((p) => p.label === e.target.value);
                if (!preset) return;
                setHypothesis(preset.label);
                setMetric(preset.metric);
                setFilter({ hu_postflop: true, ...preset.filter });
              }}
            >
              <option value="">選擇預設假設…</option>
              {PRESETS.map((p) => (
                <option key={p.label} value={p.label}>
                  {p.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="研究假設（可自行修改）">
            <textarea
              value={hypothesis}
              rows={2}
              onChange={(e) => setHypothesis(e.target.value)}
              placeholder="例如：這個牌池的 BB 面對 open 棄牌率高於合理防守頻率"
            />
          </Field>
          <Field label="指標">
            <select
              value={metric}
              onChange={(e) => setMetric(e.target.value)}
            >
              <option value="">選擇指標…</option>
              {catalog.metrics.map((m) => (
                <option key={m.key} value={m.key}>
                  {m.key} · {m.definition}
                </option>
              ))}
            </select>
          </Field>
        </div>
      )}
      {step === 1 && (
        <div className="mda-controls">
          <Field label="街道">
            <select
              value={filter.street || ""}
              onChange={(e) =>
                patch({ street: e.target.value || undefined })
              }
            >
              <option value="">全部</option>
              {["preflop", "flop", "turn", "river"].map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </Field>
          <Field label="級別（BB）">
            <select
              value={filter.bb_units || ""}
              onChange={(e) =>
                patch({
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
          <Field label="日期由">
            <input
              type="date"
              value={filter.date_from || ""}
              onChange={(e) => patch({ date_from: e.target.value || undefined })}
            />
          </Field>
          <Field label="至">
            <input
              type="date"
              value={filter.date_to || ""}
              onChange={(e) => patch({ date_to: e.target.value || undefined })}
            />
          </Field>
          <label className="check-label">
            <input
              type="checkbox"
              checked={filter.hu_postflop !== false}
              onChange={(e) => patch({ hu_postflop: e.target.checked })}
            />
            翻後只看 HU
          </label>
        </div>
      )}
      {step === 2 && (
        <div>
          {stat.loading && <Skeleton rows={2} />}
          {row ? (
            <>
              {(stat.data?.data.rows.length || 0) > 1 && (
                <p className="study-warning">
                  此情境符合 {number(stat.data?.data.rows.length || 0)}{" "}
                  個級別／情境，以下顯示第一個；可在上一步鎖定級別。
                </p>
              )}
              <div className="mda-controls">
                <div>
                  <small>分子／合法機會</small>
                  <strong>
                    {number(row.numerator)} / {number(row.opportunities)}
                  </strong>
                </div>
                <div>
                  <small>百分比／Wilson 95%</small>
                  <strong>
                    {row.pct.toFixed(1)}%（{row.wilson95[0].toFixed(1)}–
                    {row.wilson95[1].toFixed(1)}%）
                  </strong>
                </div>
                <div>
                  <small>情境</small>
                  <strong>
                    {row.metric} · {row.context || "全部位置"} · {row.street}{" "}
                    · BB $
                    {row.bb_units / 1_000_000}
                  </strong>
                </div>
              </div>
              <p>
                分時段：
                {row.period_interval_overlap === null
                  ? "分時段證據不足"
                  : row.period_interval_overlap
                    ? "前後期區間重疊（描述性一致）"
                    : "前後期區間不一致，先檢查資料組成"}
                ；攤牌底牌覆蓋{" "}
                {row.showdown_coverage_pct?.toFixed(1) ?? "—"}%（有選擇偏差）。
              </p>
              {row.opportunities < 200 && (
                <p className="study-warning">
                機會數 {number(row.opportunities)} 低於{" "}
                {MIN_RESEARCH_OPPORTUNITIES}
                  的建議門檻；先擴大情境或累積資料，再下結論。
                </p>
              )}
            </>
          ) : (
            !stat.loading && (
              <p>
                此指標在目前情境沒有合法機會。回到上一步放寬街道或級別。
              </p>
            )
          )}
          <p className="muted">
            Wilson 95% 為描述性區間；同玩家決策不獨立，不是顯著性檢定。
          </p>
        </div>
      )}
      {step === 3 && (
        <div>
          {spots.loading && <Skeleton rows={2} />}
          {spots.data?.data.rows.length ? (
            <div className="table-scroll">
              <table className="mda-table">
                <thead>
                  <tr>
                    <th>匿名手牌</th>
                    <th>情境</th>
                    <th>當刻牌面</th>
                    <th>行動／尺寸</th>
                  </tr>
                </thead>
                <tbody>
                  {spots.data.data.rows.map((r) => (
                    <tr key={`${r.hand_key}:${r.seq}`}>
                      <td>
                        <button
                          className="text-button mono"
                          onClick={() => startReplay(r.hand_key, r.seq)}
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
                        {r.action} ·{" "}
                        {r.facing_bet_pct?.toFixed(1) ?? "—"}% ·{" "}
                        {r.effective_bb.toFixed(1)}bb
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            !spots.loading && (
              <p>此情境沒有可回查的匿名手牌；可回上一步調整情境。</p>
            )
          )}
          <p className="muted">
            顯示底牌只代表有選擇偏差的可見樣本，不是完整 range。
          </p>
        </div>
      )}
      {step === 4 && (
        <div>
          {row ? (
            <>
              <p>
                假設「{hypothesis.trim() || "未命名"}」已核對：指標 {row.metric}，{" "}
                {number(row.numerator)}/{number(row.opportunities)}（
                {row.pct.toFixed(1)}%）。請在 AI
                剝削分析中評估調整方向，草稿會保存於學習資料。
              </p>
              <button
                className="button primary"
                onClick={() => goCoach(composeCoachQuestion(hypothesis, row))}
              >
                帶到 AI 剝削分析
              </button>
            </>
          ) : (
            <p>尚未取得統計；回到「檢視統計」確認情境後再繼續。</p>
          )}
          {!catalog.dataset.cloud_enabled && (
            <p className="study-warning">
              此資料集尚未啟用雲端 AI
              Coach；請先在上方「平台限制及資料分享」開啟分享，或改用外部 MCP 代理。
            </p>
          )}
        </div>
      )}
      <div className="study-row-actions">
        <button
          className="button"
          disabled={step === 0}
          onClick={() => setStep((s) => Math.max(0, s - 1))}
        >
          上一步
        </button>
        <button
          className="button primary"
          disabled={!canNext || step >= 4}
          onClick={() => setStep((s) => Math.min(4, s + 1))}
        >
          下一步
        </button>
        <button
          className="text-button"
          onClick={() => {
            setStep(0);
            setHypothesis("");
            setMetric("");
            setFilter({ hu_postflop: true });
          }}
        >
          重新開始
        </button>
      </div>
    </section>
  );
}
