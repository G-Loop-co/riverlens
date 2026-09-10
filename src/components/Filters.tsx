import { useTranslation } from "react-i18next";
import type { Filter } from "../types";
import { Field } from "./UI";
export function AdvancedFilters({
  filter: f,
  update,
}: {
  filter: Filter;
  update: (v: Partial<Filter>) => void;
}) {
  const { t } = useTranslation();
  const num = (key: keyof Filter, value: string) =>
    update({ [key]: value === "" ? undefined : Number(value) });
  return (
    <div id="advanced-filters" className="advanced-filters">
      <div className="filter-grid">
        <Field label={t("開始日期 · HKT")}>
          <input
            type="date"
            value={f.date_from || ""}
            onChange={(e) => update({ date_from: e.target.value })}
          />
        </Field>
        <Field label={t("結束日期 · HKT")}>
          <input
            type="date"
            value={f.date_to || ""}
            onChange={(e) => update({ date_to: e.target.value })}
          />
        </Field>
        <Field label={t("底池類型")}>
          <select
            value={f.pot_type || ""}
            onChange={(e) => update({ pot_type: e.target.value })}
          >
            <option value="">{t("全部")}</option>
            {["limped", "SRP", "3-bet", "4-bet+", "special"].map((v) => (
              <option key={v} value={v}>
                {typeof v === "string" ? t(v) : v}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("實際參與人數")}>
          <select
            value={f.player_count ?? ""}
            onChange={(e) => num("player_count", e.target.value)}
          >
            <option value="">{t("全部")}</option>
            {[2, 3, 4, 5, 6, 7, 8, 9].map((v) => (
              <option key={v} value={v}>
                {typeof v === "string" ? t(v) : v}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("Hero 起始籌碼 ≥ bb")}>
          <input
            type="number"
            min="0"
            value={f.stack_min ?? ""}
            onChange={(e) => num("stack_min", e.target.value)}
            placeholder={t("不限")}
          />
        </Field>
        <Field label={t("Hero 起始籌碼 ≤ bb")}>
          <input
            type="number"
            min="0"
            value={f.stack_max ?? ""}
            onChange={(e) => num("stack_max", e.target.value)}
            placeholder={t("不限")}
          />
        </Field>
        <Field label={t("HU 有效籌碼 ≥ bb")}>
          <input
            type="number"
            min="0"
            value={f.effective_min ?? ""}
            onChange={(e) => num("effective_min", e.target.value)}
            placeholder={t("僅 HU flop")}
          />
        </Field>
        <Field label={t("HU 有效籌碼 ≤ bb")}>
          <input
            type="number"
            min="0"
            value={f.effective_max ?? ""}
            onChange={(e) => num("effective_max", e.target.value)}
            placeholder={t("僅 HU flop")}
          />
        </Field>
        <Field label={t("Flop 花色")}>
          <select
            value={f.texture || ""}
            onChange={(e) => update({ texture: e.target.value })}
          >
            <option value="">{t("全部")}</option>
            <option value="rainbow">{t("Rainbow")}</option>
            <option value="two-tone">{t("Two-tone")}</option>
            <option value="monotone">{t("Monotone")}</option>
          </select>
        </Field>
        <Field label={t("Flop 配對")}>
          <select
            value={f.paired === undefined ? "" : String(f.paired)}
            onChange={(e) =>
              update({
                paired:
                  e.target.value === "" ? undefined : e.target.value === "true",
              })
            }
          >
            <option value="">{t("全部")}</option>
            <option value="true">{t("Paired")}</option>
            <option value="false">{t("Unpaired")}</option>
          </select>
        </Field>
        <Field label={t("Flop 最高牌")}>
          <select
            value={f.high_card || ""}
            onChange={(e) => update({ high_card: e.target.value })}
          >
            <option value="">{t("全部")}</option>
            {"AKQJT98765432".split("").map((v) => (
              <option key={v} value={v}>
                {typeof v === "string" ? t(v) : v}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("Flop 人數")}>
          <select
            value={f.flop_players ?? ""}
            onChange={(e) => num("flop_players", e.target.value)}
          >
            <option value="">{t("全部")}</option>
            {[2, 3, 4, 5, 6].map((v) => (
              <option key={v} value={v}>
                {typeof v === "string" ? t(v) : v}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("街道")}>
          <select
            value={f.street || ""}
            onChange={(e) => update({ street: e.target.value })}
          >
            <option value="">{t("全部")}</option>
            {["preflop", "flop", "turn", "river"].map((v) => (
              <option key={v} value={v}>
                {typeof v === "string" ? t(v) : v}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("Hero 行動")}>
          <select
            value={f.action || ""}
            onChange={(e) => update({ action: e.target.value })}
          >
            <option value="">{t("全部")}</option>
            {["fold", "check", "call", "bet", "raise"].map((v) => (
              <option key={v} value={v}>
                {typeof v === "string" ? t(v) : v}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("本次投入 ≥ 底池 %")}>
          <input
            type="number"
            min="0"
            value={f.bet_min ?? ""}
            onChange={(e) => num("bet_min", e.target.value)}
            placeholder={t("不限")}
          />
        </Field>
        <Field label={t("本次投入 ≤ 底池 %")}>
          <input
            type="number"
            min="0"
            value={f.bet_max ?? ""}
            onChange={(e) => num("bet_max", e.target.value)}
            placeholder={t("不限")}
          />
        </Field>
        <Field label={t("標籤")}>
          <input
            value={f.tag || ""}
            onChange={(e) => update({ tag: e.target.value })}
            placeholder={t("精確標籤名稱")}
          />
        </Field>
        <Field label={t("複盤狀態")}>
          <select
            value={f.reviewed === undefined ? "" : String(f.reviewed)}
            onChange={(e) =>
              update({
                reviewed:
                  e.target.value === "" ? undefined : e.target.value === "true",
              })
            }
          >
            <option value="">{t("全部")}</option>
            <option value="false">{t("未複盤")}</option>
            <option value="true">{t("已複盤")}</option>
          </select>
        </Field>
        <Field label={t("資料狀態")}>
          <select
            value={f.status || "valid"}
            onChange={(e) => update({ status: e.target.value })}
          >
            <option value="valid">{t("已核對")}</option>
            <option value="quarantined">{t("已隔離")}</option>
          </select>
        </Field>
        <Field label={t("All-in EV")}>
          <select
            value={f.ev_status || ""}
            onChange={(e) => update({ ev_status: e.target.value })}
          >
            <option value="">{t("全部")}</option>
            <option value="complete">{t("已完成")}</option>
            <option value="pending">{t("待計算")}</option>
            <option value="excluded">{t("排除")}</option>
            <option value="not_applicable">{t("非 All-in")}</option>
          </select>
        </Field>
      </div>
      <p className="helper">
        {t(
          "下注比例為本次實際投入 ÷ 行動前底池；多個行動條件必須符合同一次 Hero 行動。HU effective stack 使用參與 flop 雙方嘅起始籌碼。",
        )}
      </p>
    </div>
  );
}
