import i18n from "./i18n";
export const number = (n: number) =>
  new Intl.NumberFormat(i18n.language).format(n);
export const money = (v: string | number | null, sign = false) =>
  v === null
    ? "—"
    : `${sign && Number(v) > 0 ? "+" : ""}${new Intl.NumberFormat("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(Number(v))}`;
export const decimal = (v: number | null, places = 1) =>
  v === null ? "—" : v.toFixed(places);
export const time = (epoch: number) =>
  new Intl.DateTimeFormat(i18n.language === "en" ? "en-GB" : i18n.language, {
    timeZone: "Asia/Hong_Kong",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(epoch * 1000);
export const EV_REASONS: Record<string, string> = {
  invalid_ledger: "帳本未通過核對",
  cashout: "含 Cashout 結算",
  multiple_runouts: "多次發牌",
  special_betting: "特殊下注結構",
  special_settlement: "特殊／未知費用",
  multiway_or_no_lock: "多人或無明確 all-in lock",
  side_pots: "含 Side pots",
  unknown_hole_cards: "底牌未完整公開",
  incomplete_board: "牌面資料不完整",
};
export const CUES = [
  [
    "open_vs_three_bet",
    "Open 後遇 3-bet",
    "睇清楚 Fold、Call、4-bet 嘅實際選擇。",
  ],
  ["call_three_bet", "Call 3-bet pots", "按位置及有效籌碼檢討跟注路線。"],
  ["blind_defence", "盲位防守", "分開檢討 SB／BB 嘅回應。"],
  ["cbet_vs_raise", "C-bet 遇 Raise", "檢查下注後面對加注嘅決策。"],
  ["cbet_turn_check", "C-bet → Turn Check", "回看轉牌放慢之後嘅行動。"],
  ["river_call", "River Calls", "同時檢討贏牌與輸牌，避免結果偏差。"],
] as const;
export const ACTIONS: Record<string, string> = {
  small_blind: "SB",
  big_blind: "BB",
  dead_blind: "Missed blind",
  ante: "Ante",
  straddle: "Straddle",
  fold: "Fold",
  check: "Check",
  call: "Call",
  bet: "Bet",
  raise: "Raise",
  return: "退回未跟注",
  collect: "收取底池",
  show: "Show",
  muck: "Muck",
  board: "發牌",
  cashout_choice: "選擇 EV Cashout",
  cashout_receive: "Cashout 收款",
  cashout_risk: "Cashout Risk",
};
export const STAT_LABELS: Record<string, string> = {
  vpip: "VPIP",
  pfr: "PFR",
  rfi: "RFI",
  three_bet: "3-bet",
  fold_three_bet: "Fold to 3-bet",
  blind_fold: "Blind Fold",
  blind_call: "Blind Call",
  blind_raise: "Blind Raise",
  cbet: "Flop C-bet",
  fold_cbet: "Fold vs C-bet",
  wtsd: "WTSD",
  wsd: "W$SD",
  wwsf: "WWSF",
};
