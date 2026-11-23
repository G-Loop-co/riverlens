import { useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { useRpc } from "../api";
import type {
  BluffHand,
  BluffMoney,
  BluffPage,
  BluffQuery,
  BluffSummary,
} from "../bluff-types";
import type { Health, Profile } from "../types";
import { ErrorBanner, Field, Skeleton } from "./UI";
import "./bluffs.css";

const singles = ["flop_air", "flop_draw", "turn_air", "turn_draw", "river_air"];
const views = ["single", "line", "reached", "next", "pair_draw", "manual"];
const positions = [
  "UTG",
  "UTG+1",
  "MP",
  "HJ",
  "CO",
  "BTN",
  "SB",
  "BB",
  "BTN/SB",
];
const textures = ["unpaired", "paired"].flatMap((pair) =>
  ["rainbow", "two_suit", "three_suit", "four_plus_suit"].map(
    (suit) => pair + "_" + suit,
  ),
);
const dimensions = [
  "none",
  "size",
  "action",
  "response",
  "position",
  "opponent",
  "pot",
  "players",
  "depth",
  "texture",
];

export default function BluffReport({
  revision,
  profiles,
  openHand,
  health,
}: {
  revision: number;
  profiles: Profile[];
  openHand: (id: number, seq: number) => void;
  health: Health | null;
}) {
  const { t, i18n } = useTranslation();
  const [params, setParams] = useSearchParams();
  const read = (key: string, fallback = "") =>
    params.get("b_" + key) ?? fallback;
  const view = read("view", "single");
  const breakdown = read("breakdown", "none");
  const currency = read(
    "currency",
    params.get("currency") === "CHIPS"
      ? "all"
      : params.get("currency") || "all",
  );
  const tableUnit =
    currency !== "all" && read("unit") === "cash" ? currency : "BB";
  const category = read("category");
  const cellBucket = read("bucket");
  function change(key: string, value: string) {
    const p = new URLSearchParams(params);
    p.set("b_" + key, value);
    for (const k of ["category", "bucket", "page"]) p.delete("b_" + k);
    setParams(p, { replace: true });
  }
  function chooseCell(key: string, bucket: string) {
    const p = new URLSearchParams(params);
    p.set("b_category", key);
    p.set("b_bucket", bucket);
    p.delete("b_page");
    setParams(p, { replace: true });
  }
  function page(next: [number, number] | null) {
    const p = new URLSearchParams(params);
    if (next) p.set("b_page", next.join(","));
    else p.delete("b_page");
    setParams(p, { replace: true });
  }
  const rawCursor = read("page").split(",").map(Number);
  const before: [number, number] | undefined =
    rawCursor.length === 2 &&
    rawCursor.every((n) => Number.isSafeInteger(n) && n >= 0)
      ? [rawCursor[0], rawCursor[1]]
      : undefined;
  const query: BluffQuery = {
    view,
    breakdown,
    filter: {
      currency: currency === "all" ? undefined : currency,
      date_from: read("from", params.get("date_from") ?? "") || undefined,
      date_to: read("to", params.get("date_to") ?? "") || undefined,
      stakes: read("stakes", params.get("stakes") ?? "") || undefined,
      profile: read("profile", params.get("profile") ?? "") || undefined,
      position: read("position", params.get("position") ?? "") || undefined,
      pot_type: read("pot", params.get("pot_type") ?? "") || undefined,
    },
    brand: read("brand") || undefined,
    players: read("players") || undefined,
    action: read("action") || undefined,
    sizing: read("size") || undefined,
    draw: read("draw") || undefined,
    opponent: read("opponent") || undefined,
    depth: read("depth") || undefined,
    texture: read("texture") || undefined,
  };
  const report = useRpc<BluffSummary>({ op: "bluff_report", query }, revision);
  const details = useRpc<BluffPage>(
    category
      ? {
          op: "bluff_report",
          query: { ...query, cell: { category, bucket: cellBucket }, before },
        }
      : null,
    revision,
  );
  const data = report.data;
  const names = (key: string) => t("bluff." + key, { defaultValue: key });
  const word = (value: string) =>
    /^(?:[2-9]|10)$/.test(value)
      ? t("bluff.playerCount", { n: value })
      : t("bluff.v." + value, { defaultValue: t(value) });
  const kind = (key: string) => {
    if (view === "single" || view === "pair_draw" || view === "manual") {
      const i = key.indexOf("_");
      return word(key.slice(0, i)) + " · " + word(key.slice(i + 1));
    }
    const streets = key
      .split("_")
      .filter((s) => ["flop", "turn", "river"].includes(s))
      .map(word)
      .join(" → ");
    const suffix = [
      "interrupted_xr_lead",
      "xr_lead",
      "interrupted_raise",
      "interrupted",
      "raise",
      "continue",
      "stop",
      "made",
    ].find((s) => key.endsWith("_" + s));
    return suffix ? streets + " · " + word(suffix) : streets;
  };
  const amount = (row: BluffMoney, k: "segment" | "net") => {
    const n =
      tableUnit === "BB"
        ? row[k === "segment" ? "segment_bb" : "net_bb"]
        : row[k];
    if (n === null) return "—";
    return new Intl.NumberFormat(i18n.language, {
      maximumFractionDigits: tableUnit === "BB" ? 2 : 6,
      signDisplay: "exceptZero",
    }).format(Number(n));
  };
  const options = (
    key: string,
    values: string[],
    selected = read(key),
    label = names(key),
    all = true,
    display = word,
  ) => (
    <Field label={label}>
      <select value={selected} onChange={(e) => change(key, e.target.value)}>
        {all && <option value="">{t("全部")}</option>}
        {values.map((v) => (
          <option key={v} value={v}>
            {display(v)}
          </option>
        ))}
      </select>
    </Field>
  );
  const groups = data?.groups ?? [];
  const rows =
    view === "single" && breakdown === "none"
      ? singles.map((key) => ({
          key,
          bucket: "",
          g: groups.find((g) => g.category === key),
        }))
      : groups.map((g) => ({ key: g.category, bucket: g.bucket, g }));
  const pct = (n: number, d: number) =>
    d
      ? new Intl.NumberFormat(i18n.language, {
          style: "percent",
          maximumFractionDigits: 1,
        }).format(n / d)
      : "—";
  function handRow(h: BluffHand) {
    return (
      <tr key={h.id + "-" + h.seq}>
        <td>
          <button
            className="text-button mono"
            onClick={() => openHand(h.id, h.seq)}
          >
            {h.hand_id}
          </button>
          <small>{h.date}</small>
        </td>
        <td>
          {word(h.street)} · {word(h.holding)}
          <small>
            {h.outs > 0 ? h.outs + " outs" : ""}
            {h.gutshot ? " · " + word("gutshot") : ""}
            {h.missed ? word("missed") : ""}
          </small>
        </td>
        <td>
          {word(h.action)}
          {h.size !== null && <small>{h.size.toFixed(1)}%</small>}
        </td>
        <td>{word(h.response)}</td>
        <td>
          {amount(h, "segment")}
          {h.exclusion && <small>{word(h.exclusion)}</small>}
        </td>
        <td>{amount(h, "net")}</td>
        <td>
          {h.followups?.map((f) => (
            <button
              className="text-button"
              key={f.seq}
              onClick={() => openHand(h.id, f.seq)}
            >
              {names("reraise")}
              {f.size !== null && " · " + f.size.toFixed(1) + "%"}
            </button>
          ))}
          {h.start !== h.seq && (
            <button
              className="text-button"
              onClick={() => openHand(h.id, h.start)}
            >
              {names("firstShot")}
            </button>
          )}
        </td>
      </tr>
    );
  }

  return (
    <div className="bluff-report">
      <section className="bluff-filters" aria-label={names("filters")}>
        <Field label={t("開始日期 · HKT")}>
          <input
            type="date"
            value={query.filter.date_from ?? ""}
            onChange={(e) => change("from", e.target.value)}
          />
        </Field>
        <Field label={t("結束日期 · HKT")}>
          <input
            type="date"
            value={query.filter.date_to ?? ""}
            onChange={(e) => change("to", e.target.value)}
          />
        </Field>
        {options(
          "brand",
          [
            ...new Set([
              ...profiles.map((p) => p.brand),
              ...(query.brand ? [query.brand] : []),
            ]),
          ].sort(),
        )}
        {options(
          "currency",
          [...new Set(["all", ...(data?.currencies ?? []), currency])],
          currency,
          names("currency"),
          false,
          (v) => (v === "all" ? t("全部") : v),
        )}
        {options(
          "stakes",
          [
            ...new Set([
              ...(data?.stakes ?? []),
              ...(query.filter.stakes ? [query.filter.stakes] : []),
            ]),
          ],
          query.filter.stakes ?? "",
        )}
        {options(
          "pot",
          ["limped", "SRP", "3-bet", "4-bet+", "special"],
          query.filter.pot_type ?? "",
        )}
        {options("players", [
          "hu",
          "multi",
          "3",
          "4",
          "5",
          "6",
          "7",
          "8",
          "9",
          "10",
        ])}
      </section>
      <details className="bluff-advanced">
        <summary>{t("進階篩選")}</summary>
        <div className="bluff-filters">
          {options(
            "profile",
            profiles.map((p) => p.id),
            query.filter.profile ?? "",
            t("來源"),
            true,
            (id) => profiles.find((p) => p.id === id)?.name ?? id,
          )}
          {options("position", positions, query.filter.position ?? "")}
          {options("opponent", positions)}
          {options("size", ["small", "medium", "large", "over"])}
          {options("action", ["bet", "raise", "check_raise"])}
          {options("depth", ["short", "mid", "deep"])}
          {options("texture", textures)}
          {options("draw", ["gutshot", "missed", "other_air"])}
        </div>
      </details>
      <div className="bluff-controls">
        {options("view", views, view, names("view"), false, (v) =>
          names("view." + v),
        )}
        {options(
          "breakdown",
          dimensions,
          breakdown,
          names("breakdown"),
          false,
          (v) => names(v),
        )}
        {currency !== "all" &&
          options(
            "unit",
            ["bb", "cash"],
            tableUnit === "BB" ? "bb" : "cash",
            names("unit"),
            false,
            (v) => (v === "bb" ? "BB" : currency),
          )}
      </div>
      <p className="bluff-caption">{names("hint." + view)}</p>
      {health?.study_error && <ErrorBanner>{health.study_error}</ErrorBanner>}
      {data && !data.coverage.complete && (
        <p role="status">
          {t("bluff.coverage", data.coverage)}{" "}
          {health?.study_paused ? t("bluff.paused") : ""}
        </p>
      )}
      {report.error && <ErrorBanner>{report.error}</ErrorBanner>}
      {!data && report.loading && <Skeleton rows={5} />}
      {data && (
        <section className="bluff-panel">
          <h2>{names("view." + view)}</h2>
          <div className="bluff-scroll">
            <table>
              <thead>
                <tr>
                  <th>{names("category")}</th>
                  {breakdown !== "none" && <th>{names(breakdown)}</th>}
                  <th>{names("attempts")}</th>
                  <th>
                    {names(view === "next" ? "opportunity" : "immediate")}
                  </th>
                  <th>
                    {names("segment")} · {tableUnit}
                  </th>
                  <th>
                    {names("whole")} · {tableUnit}
                  </th>
                </tr>
              </thead>
              <tbody>
                {rows.map(({ key, bucket, g }) => {
                  const denom =
                    view === "next"
                      ? groups
                          .filter(
                            (x) =>
                              x.bucket === bucket &&
                              x.category.slice(
                                0,
                                x.category.lastIndexOf("_"),
                              ) === key.slice(0, key.lastIndexOf("_")),
                          )
                          .reduce((n, x) => n + x.attempts, 0)
                      : 0;
                  return (
                    <tr
                      key={key + ":" + bucket}
                      className={
                        category === key && cellBucket === bucket
                          ? "bluff-selected"
                          : ""
                      }
                    >
                      <th scope="row">
                        {g ? (
                          <button
                            className="text-button"
                            onClick={() => chooseCell(key, bucket)}
                          >
                            {kind(key)}
                          </button>
                        ) : (
                          kind(key)
                        )}
                      </th>
                      {breakdown !== "none" && <td>{word(bucket)}</td>}
                      <td>
                        {g?.attempts ?? 0}
                        {g && g.hands !== g.attempts && (
                          <small>{t("bluff.hands", { n: g.hands })}</small>
                        )}
                      </td>
                      <td>
                        {g
                          ? view === "next"
                            ? pct(g.attempts, denom)
                            : pct(g.folded, g.attempts)
                          : "—"}
                        {g && (
                          <small>
                            {view === "next" ? g.attempts : g.folded} /{" "}
                            {view === "next" ? denom : g.attempts}
                          </small>
                        )}
                      </td>
                      {(["segment", "net"] as const).map((k) => (
                        <td
                          key={k}
                          className={
                            !g
                              ? ""
                              : (g[k === "net" ? "net_bb" : "segment_bb"] ??
                                    0) < 0
                                ? "negative"
                                : "positive"
                          }
                        >
                          {g ? amount(g, k) : "—"}
                          {k === "segment" && g && g.settled < g.attempts && (
                            <small>
                              {g.settled} / {g.attempts}
                            </small>
                          )}
                        </td>
                      ))}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          {groups.length === 0 && <p>{names("empty")}</p>}
          {(data.unknown > 0 || Object.keys(data.exclusions).length > 0) && (
            <p className="bluff-caption">
              {[
                ...(data.unknown
                  ? [names("unknown") + " × " + data.unknown]
                  : []),
                ...Object.entries(data.exclusions).map(
                  ([k, n]) => word(k) + " × " + n,
                ),
              ].join(" · ")}
            </p>
          )}
        </section>
      )}
      {category && (
        <section className="bluff-panel">
          <h2>
            {kind(category)}
            {cellBucket ? " · " + word(cellBucket) : ""}
          </h2>
          {details.error && <ErrorBanner>{details.error}</ErrorBanner>}
          {details.loading && !details.data && <Skeleton rows={3} />}
          {details.data && (
            <>
              <div className="bluff-scroll">
                <table>
                  <thead>
                    <tr>
                      {[
                        "hand",
                        "holding",
                        "action",
                        "response",
                        "segment",
                        "whole",
                        "replay",
                      ].map((k) => (
                        <th key={k}>
                          {names(k)}
                          {["segment", "whole"].includes(k)
                            ? " · " + tableUnit
                            : ""}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>{details.data.rows.map(handRow)}</tbody>
                </table>
              </div>
              {!details.data.rows.length && <p>{names("empty")}</p>}
              <div className="bluff-controls">
                {before && (
                  <button className="button subtle" onClick={() => page(null)}>
                    {names("firstPage")}
                  </button>
                )}
                {details.data.next && (
                  <button
                    className="button subtle"
                    onClick={() => page(details.data!.next)}
                  >
                    {names("nextPage")}
                  </button>
                )}
              </div>
            </>
          )}
        </section>
      )}
      <details className="bluff-advanced">
        <summary>{names("definitions")}</summary>
        <p>{names("cardRule")}</p>
        <p>{names("moneyRule")}</p>
        <p>{names("sizeRule")}</p>
      </details>
    </div>
  );
}
