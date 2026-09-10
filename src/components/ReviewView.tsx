import { useTranslation } from "react-i18next";
import {
  ArrowRight,
  ArrowUpRight,
  BookmarkSimple,
} from "@phosphor-icons/react";
import { useRpc } from "../api";
import type { Filter, Report } from "../types";
import { CUES, decimal, money, number } from "../format";
function Cue({
  id,
  title,
  description,
  filter,
  revision,
  open,
}: {
  id: string;
  title: string;
  description: string;
  filter: Filter;
  revision: number;
  open: () => void;
}) {
  const { t } = useTranslation();
  const result = useRpc<Report>(
    { op: "overview", filter: { ...filter, cue: id }, group: "position" },
    revision,
  );
  return (
    <button className="review-cue" onClick={open}>
      <span className="cue-icon">
        <BookmarkSimple size={22} />
      </span>
      <div>
        <h3>{t(title)}</h3>
        <p>{t(description)}</p>
        <div className="cue-numbers">
          {result.data ? (
            <>
              <span>
                {t("{{v0}} 手", {
                  count: result.data.hands,
                  v0: number(result.data.hands),
                })}
              </span>
              <span>${money(result.data.net, true)}</span>
              <span>
                {t("{{v0}} bb/100", { v0: decimal(result.data.bb100) })}
              </span>
            </>
          ) : (
            <span>{result.error ? t("載入失敗") : t("載入中…")}</span>
          )}
        </div>
        {result.data && result.data.hands < 30 && (
          <small className="muted">{t("樣本較少 · 只作手牌檢討入口")}</small>
        )}
      </div>
      <ArrowUpRight size={19} />
    </button>
  );
}
export function ReviewView({
  filter,
  revision,
  drill,
}: {
  filter: Filter;
  revision: number;
  drill: (f: Partial<Filter>, sort?: "win" | "loss") => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="review-view">
      <div className="review-intro">
        <div>
          <span className="section-kicker">{t("A DELIBERATE REVIEW")}</span>
          <h2>{t("檢討決策，保留你的思路。")}</h2>
          <p>
            {t(
              "以下情境根據你嘅實際行動整理。包括贏牌與輸牌，唔會因結果差就判定你打錯。",
            )}
          </p>
        </div>
        <button
          className="button primary"
          onClick={() => drill({ reviewed: false })}
        >
          {t("繼續未完成複盤")}
          <ArrowRight size={17} />
        </button>
      </div>
      <div className="review-grid">
        {CUES.map(([id, title, description]) => (
          <Cue
            key={id}
            id={id}
            title={t(title)}
            description={t(description)}
            filter={filter}
            revision={revision}
            open={() => drill({ cue: id })}
          />
        ))}
      </div>
      <section className="panel review-notes">
        <div>
          <h3>{t("由最大波動開始")}</h3>
          <p>{t("檢討最大贏牌與最大輸牌，再按位置及底池類型細分。")}</p>
        </div>
        <button
          className="button subtle"
          onClick={() => drill({ result: "win" }, "win")}
        >
          {t("查看贏牌")}
          <ArrowUpRight size={16} />
        </button>
        <button
          className="button subtle"
          onClick={() => drill({ result: "loss" }, "loss")}
        >
          {t("查看輸牌")}
          <ArrowUpRight size={16} />
        </button>
      </section>
    </div>
  );
}
