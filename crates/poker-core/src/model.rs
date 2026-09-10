use crate::money;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PARSER_VERSION: &str = "gg-cash/1.0.0";
pub const STATS_VERSION: &str = "hero-opportunities/1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub brand: String,
    pub hero: String,
    pub timezone: String,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            id: "natural8-hero".into(),
            name: "我的 Natural8".into(),
            brand: "Natural8".into(),
            hero: "Hero".into(),
            timezone: "Asia/Hong_Kong".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub seat: u8,
    pub name: String,
    pub hero: bool,
    #[serde(with = "money")]
    pub stack: i64,
    pub cards: Vec<String>,
    pub position: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub seq: usize,
    pub street: String,
    pub runout: usize,
    pub actor: Option<u8>,
    pub kind: String,
    /// Incremental contribution, return, payout or external settlement.
    #[serde(with = "money")]
    pub amount: i64,
    #[serde(with = "money")]
    pub to: i64,
    pub all_in: bool,
    pub cards: Vec<String>,
    #[serde(with = "money")]
    pub pot_after: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatValue {
    pub numerator: u64,
    pub opportunities: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pot {
    #[serde(with = "money")]
    pub amount: i64,
    pub eligible: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityInput {
    pub hero: Vec<String>,
    pub villain: Vec<String>,
    pub board: Vec<String>,
    #[serde(with = "money")]
    pub net_pot: i64,
    #[serde(with = "money")]
    pub contribution: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: String,
    pub profile: String,
    pub brand: String,
    pub played_at: i64,
    pub local_time: String,
    pub timezone: String,
    pub table_name: String,
    pub max_seats: u8,
    pub player_count: usize,
    pub currency: String,
    pub game: String,
    #[serde(with = "money")]
    pub sb: i64,
    #[serde(with = "money")]
    pub bb: i64,
    pub button: u8,
    pub hero_seat: u8,
    pub position: String,
    pub hand_class: String,
    pub players: Vec<Player>,
    pub actions: Vec<Action>,
    pub boards: Vec<Vec<String>>,
    pub pots: Vec<Pot>,
    #[serde(with = "money")]
    pub invested: i64,
    #[serde(with = "money")]
    pub returned: i64,
    #[serde(with = "money")]
    pub collected: i64,
    #[serde(with = "money")]
    pub cashout: i64,
    #[serde(with = "money")]
    pub cashout_risk: i64,
    #[serde(with = "money")]
    pub net: i64,
    #[serde(with = "money")]
    pub total_pot: i64,
    pub fees: BTreeMap<String, String>,
    pub showdown: bool,
    pub saw_flop: bool,
    pub pot_type: String,
    pub flop_players: usize,
    pub hu_effective_bb: Option<f64>,
    pub hero_stack_bb: f64,
    pub texture: String,
    pub paired: bool,
    pub high_card: String,
    pub stats: BTreeMap<String, StatValue>,
    pub cues: Vec<String>,
    pub status: String,
    pub issues: Vec<Issue>,
    pub ev_status: String,
    pub ev_reason: Option<String>,
    pub equity_input: Option<EquityInput>,
    pub equity: Option<f64>,
    pub adjusted_net: Option<String>,
    pub raw: String,
    pub parser_version: String,
    pub stats_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Filter {
    pub source_date: Option<String>,
    pub profile: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub timezone: Option<String>,
    pub position: Option<String>,
    pub game: Option<String>,
    pub stakes: Option<String>,
    pub currency: Option<String>,
    pub player_count: Option<u8>,
    pub pot_type: Option<String>,
    pub hand_class: Option<String>,
    pub texture: Option<String>,
    pub paired: Option<bool>,
    pub high_card: Option<String>,
    pub showdown: Option<bool>,
    pub stack_min: Option<f64>,
    pub stack_max: Option<f64>,
    pub effective_min: Option<f64>,
    pub effective_max: Option<f64>,
    pub flop_players: Option<u8>,
    pub stat: Option<String>,
    pub stat_mode: Option<String>,
    pub cue: Option<String>,
    pub street: Option<String>,
    pub action: Option<String>,
    pub bet_min: Option<f64>,
    pub bet_max: Option<f64>,
    pub tag: Option<String>,
    pub reviewed: Option<bool>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub ev_status: Option<String>,
    pub session: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Annotation {
    pub note: String,
    pub tags: Vec<String>,
    pub reviewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub profile: Profile,
    pub paths: Vec<String>,
    pub state: String,
    pub scanned: u64,
    pub inserted: u64,
    pub duplicates: u64,
    pub quarantined: u64,
    pub conflicts: u64,
    pub files_done: u64,
    pub files_total: u64,
    pub current_file: String,
    pub message: Option<String>,
    pub started_at: i64,
    pub elapsed_ms: u64,
}

pub const STAT_DEFINITIONS: &[(&str, &str, &str)] = &[
    (
        "vpip",
        "VPIP",
        "主動 Call／Raise 手數 ÷ 可作自願翻前決策手數；不含 walk、強制盲注及 ante。",
    ),
    ("pfr", "PFR", "曾翻前加注手數 ÷ 曾有合法翻前加注機會手數。"),
    (
        "rfi",
        "RFI",
        "無人 limp／raise 前 Open Raise 次數 ÷ 相同 first-in 機會；不含 BB walk。",
    ),
    (
        "three_bet",
        "3-bet",
        "首次翻前 re-raise 手數 ÷ 面對單一 raise 且能合法 re-raise 手數；包含 squeeze。",
    ),
    (
        "fold_three_bet",
        "Fold to 3-bet",
        "Hero 已加注後直接面對 3-bet 而 Fold 次數 ÷ 同類決策機會。",
    ),
    (
        "blind_fold",
        "Blind Fold",
        "盲位直接面對 first-in open、尚無 caller 時 Fold 次數 ÷ 同類機會。",
    ),
    (
        "blind_call",
        "Blind Call",
        "盲位直接面對 first-in open、尚無 caller 時 Call 次數 ÷ 同類機會。",
    ),
    (
        "blind_raise",
        "Blind Raise",
        "盲位直接面對 first-in open、尚無 caller 時 Raise 次數 ÷ 同類機會。",
    ),
    (
        "cbet",
        "Flop C-bet",
        "最後翻前 aggressor 在 flop 無人先下注時 Bet 次數 ÷ 同類可下注機會。",
    ),
    (
        "fold_cbet",
        "Fold vs C-bet",
        "Hero 直接面對 flop C-bet 而 Fold 次數 ÷ 同類可決策機會；不含中間 Raise。",
    ),
    (
        "wtsd",
        "WTSD",
        "Hero 到 showdown 手數 ÷ Hero saw flop 手數。",
    ),
    (
        "wsd",
        "W$SD",
        "Hero showdown 收到 contested pot 派彩手數 ÷ Hero showdown 手數；split 也計贏。",
    ),
    (
        "wwsf",
        "WWSF",
        "Hero saw flop 並收到 contested pot 派彩手數 ÷ Hero saw flop 手數。",
    ),
];
