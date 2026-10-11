//! Source grammar adapters. Format facts come from the source links in
//! docs/hand-history-formats.md. All adapters feed the existing fixed-point
//! ledger; they never overwrite source text or bypass reconciliation.
use crate::{
    model::{Profile, Tournament},
    money,
};
use anyhow::{bail, ensure, Context, Result};
use chrono::NaiveDateTime;
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct Normalized {
    pub text: String,
    pub currency: String,
    pub format: &'static str,
    pub timezone: Option<String>,
    pub tournament: Option<Tournament>,
}

fn re(pattern: &str) -> Regex {
    use std::sync::{LazyLock, RwLock};
    static CACHE: LazyLock<RwLock<HashMap<String, Regex>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));
    if let Some(regex) = CACHE.read().unwrap().get(pattern) {
        return regex.clone();
    }
    let regex = Regex::new(pattern).expect("static format regex");
    CACHE.write().unwrap().insert(pattern.into(), regex.clone());
    regex
}

pub fn is_header(line: &str) -> bool {
    let s = line
        .trim()
        .trim_start_matches('\u{feff}')
        .trim_start_matches("ï»¿");
    [
        "Poker Hand #",
        "PokerStars ",
        "CoinPoker Hand #",
        "Hand #",
        "Game Hand #",
        "Hand#",
        "Full Tilt Poker Game #",
        "Game started at:",
        "Winamax Poker -",
        "#Game No :",
        "***** Hand History for Game ",
        "***** History for hand ",
        "Game #",
        "Ignition Hand #",
        "Bovada Hand #",
        "Bodog Hand #",
    ]
    .iter()
    .any(|p| s.starts_with(p))
}

fn currency(s: &str) -> Result<String> {
    for (symbol, code) in [("₮", "USDT"), ("€", "EUR"), ("£", "GBP")] {
        if s.contains(symbol) {
            return Ok(code.into());
        }
    }
    for code in ["USD", "EUR", "GBP", "CAD", "USDT"] {
        if s.contains(code) {
            return Ok(code.into());
        }
    }
    ensure!(s.contains('$'), "missing or unsupported cash currency");
    Ok("USD".into())
}
fn value(s: &str) -> Result<i64> {
    let s = s.trim().replace(',', "");
    let c = re(r"[-+]?\d+(?:\.\d+)?")
        .find(&s)
        .context("missing cash amount")?;
    money::parse(c.as_str().trim_start_matches('+'))
}
fn chip_value(s: &str) -> Result<i64> {
    let s = s.trim();
    ensure!(
        re(r"^(?:\d+|\d{1,3}(?:,\d{3})+)(?:\.\d+)?$").is_match(s),
        "unsupported chip amount/abbreviation"
    );
    money::parse(&s.replace(',', ""))
}
fn date(s: &str) -> Result<String> {
    let c = re(r"(\d{4}[/-]\d{1,2}[/-]\d{1,2})[ T]+(\d{1,2}:\d{1,2}:\d{1,2})")
        .captures(s)
        .context("missing hand timestamp")?;
    let n = NaiveDateTime::parse_from_str(
        &format!("{} {}", c[1].replace('-', "/"), &c[2]),
        "%Y/%m/%d %H:%M:%S",
    )?;
    Ok(n.format("%Y/%m/%d %H:%M:%S").to_string())
}
fn timezone(s: &str) -> Option<String> {
    // Explicit source tags override the user's fallback timezone. PokerStars
    // may append a bracketed ET timestamp; use the tag after the first clock.
    let after = re(r"\d{1,2}:\d{1,2}:\d{1,2}\s+(UTC|GMT|ET|EST|EDT|CT|CST|CDT|PT|PST|PDT|WET|CET|CEST|EET|EEST|MSK)\b")
        .captures(s)
        .map(|c| c[1].to_string());
    match after.as_deref() {
        Some("UTC" | "GMT") => Some("UTC".into()),
        Some("ET") => Some("America/New_York".into()),
        Some("EST") => Some("Etc/GMT+5".into()),
        Some("EDT") => Some("Etc/GMT+4".into()),
        Some("CT") => Some("America/Chicago".into()),
        Some("CST") => Some("Etc/GMT+6".into()),
        Some("CDT") => Some("Etc/GMT+5".into()),
        Some("PT") => Some("America/Los_Angeles".into()),
        Some("PST") => Some("Etc/GMT+8".into()),
        Some("PDT") => Some("Etc/GMT+7".into()),
        Some("WET") => Some("Etc/GMT".into()),
        Some("EET") => Some("Etc/GMT-2".into()),
        Some("EEST") => Some("Etc/GMT-3".into()),
        Some("CET") => Some("Etc/GMT-1".into()),
        Some("CEST") => Some("Etc/GMT-2".into()),
        Some("MSK") => Some("Europe/Moscow".into()),
        _ => None,
    }
}
fn card_text(s: &str) -> String {
    re(r"\[([^\]]*)\]")
        .replace_all(s, |c: &regex::Captures| {
            let cs = re(r"\b10([cdhs])\b")
                .replace_all(&c[1].replace(',', " "), "T$1")
                .into_owned();
            format!("[{}]", cs.split_whitespace().collect::<Vec<_>>().join(" "))
        })
        .into_owned()
}
fn cash_text(s: &str) -> String {
    let prefix = re(r"(?:[$€£₮])\s*(\d+(?:[.,]\d+)*)").replace_all(s, |c: &regex::Captures| {
        format!("${}", c[1].replace(',', ""))
    });
    let suffix = re(r"(\d+(?:[.,]\d+)*)[€£₮]").replace_all(&prefix, |c: &regex::Captures| {
        format!("${}", c[1].replace(',', ""))
    });
    suffix
        .replace(" USD", "")
        .replace(" EUR", "")
        .replace(" GBP", "")
        .replace(" CAD", "")
        .replace(" USDT", "")
}

struct Builder {
    id: String,
    date: String,
    table: String,
    max: u8,
    button: u8,
    sb: i64,
    bb: i64,
    currency: String,
    format: &'static str,
    timezone: Option<String>,
    seats: BTreeMap<u8, (String, i64)>,
    participants: HashSet<String>,
    lines: Vec<String>,
    live: HashMap<String, i64>,
    invested: HashMap<String, i64>,
    returned: HashMap<String, i64>,
    awards: i64,
    summary: Option<(i64, String)>,
}
impl Builder {
    fn new(
        id: &str,
        stamp: String,
        sb: i64,
        bb: i64,
        curr: String,
        format: &'static str,
        tz: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            date: stamp,
            table: format.into(),
            max: 0,
            button: 0,
            sb,
            bb,
            currency: curr,
            format,
            timezone: tz,
            seats: BTreeMap::new(),
            participants: HashSet::new(),
            lines: vec![],
            live: HashMap::new(),
            invested: HashMap::new(),
            returned: HashMap::new(),
            awards: 0,
            summary: None,
        }
    }
    fn seat(&mut self, seat: u8, name: &str, stack: i64) -> Result<()> {
        ensure!(
            seat > 0 && seat <= 10 && stack >= 0,
            "invalid source seat/stack"
        );
        ensure!(
            !self.seats.contains_key(&seat) && !self.seats.values().any(|(n, _)| n == name),
            "duplicate player/seat"
        );
        self.seats.insert(seat, (name.into(), stack));
        Ok(())
    }
    fn board(&mut self, line: String) {
        self.live.clear();
        self.lines.push(line);
    }
    fn action(&mut self, name: &str, kind: &str, v: i64, target: bool, all_in: bool) -> Result<()> {
        let mut all_in = all_in;
        ensure!(
            self.seats.values().any(|(n, _)| n == name),
            "unknown source actor"
        );
        ensure!(v >= 0, "negative source amount");
        self.participants.insert(name.into());
        let target = target || kind == "posts straddle";
        let old = self.live.get(name).copied().unwrap_or(0);
        let delta = if target { v - old } else { v };
        let text = match kind {
            "raise" => format!(
                "raises $0 to ${}",
                money::format(if target { v } else { old + v })
            ),
            "fold" => "folds".into(),
            "check" => "checks".into(),
            "return" => {
                ensure!(
                    v <= self.invested.get(name).copied().unwrap_or(0)
                        - self.returned.get(name).copied().unwrap_or(0),
                    "return exceeds player's contribution"
                );
                *self.returned.entry(name.into()).or_default() += v;
                *self.live.entry(name.into()).or_default() -= v;
                self.lines.push(format!(
                    "Uncalled bet (${}) returned to {name}",
                    money::format(v)
                ));
                return Ok(());
            }
            "collect" => {
                self.awards = self.awards.checked_add(v).context("payout overflow")?;
                self.lines
                    .push(format!("{name} collected ${} from pot", money::format(v)));
                return Ok(());
            }
            _ => format!("{kind} ${}", money::format(v)),
        };
        if matches!(
            kind,
            "posts small blind"
                | "posts big blind"
                | "posts dead blind"
                | "posts ante"
                | "posts straddle"
                | "call"
                | "calls"
                | "bet"
                | "bets"
                | "raise"
        ) {
            let stack = self.seats.values().find(|(n, _)| n == name).unwrap().1;
            ensure!(
                delta >= 0
                    && delta
                        <= stack - self.invested.get(name).copied().unwrap_or(0)
                            + self.returned.get(name).copied().unwrap_or(0),
                "source contribution exceeds remaining stack"
            );
            *self.invested.entry(name.into()).or_default() += delta;
            // Some exports omit an all-in suffix. Exact source stack exhaustion
            // is sufficient evidence; never infer it from rounded chip values.
            all_in |= delta > 0
                && self.invested.get(name).copied().unwrap_or(0)
                    - self.returned.get(name).copied().unwrap_or(0)
                    == stack;
            if !matches!(kind, "posts ante" | "posts dead blind") {
                *self.live.entry(name.into()).or_default() += delta;
            }
        }
        self.lines.push(format!(
            "{name}: {text}{}",
            if all_in { " and is all-in" } else { "" }
        ));
        Ok(())
    }
    fn pot(&self) -> i64 {
        self.invested.values().sum::<i64>() - self.returned.values().sum::<i64>()
    }
    fn uncalled(&mut self) -> Result<()> {
        let mut levels: Vec<_> = self.live.iter().map(|(n, v)| (n.clone(), *v)).collect();
        levels.sort_by_key(|(_, v)| -*v);
        if let Some((name, top)) = levels.first() {
            let other = levels.get(1).map(|(_, v)| *v).unwrap_or(0);
            if *top > other {
                self.action(&name.clone(), "return", top - other, false, false)?;
            }
        }
        Ok(())
    }
    fn finish(mut self, infer_return: bool, complete: bool) -> Result<Normalized> {
        ensure!(complete, "incomplete hand: missing settlement");
        ensure!(self.awards > 0, "incomplete hand: missing payouts");
        if infer_return {
            self.uncalled()?;
        }
        ensure!(self.button > 0, "missing source button");
        ensure!(
            self.seats.contains_key(&self.button),
            "button does not identify a source seat"
        );
        self.max = self
            .max
            .max(self.seats.keys().copied().max().unwrap_or(0))
            .max(2);
        let (total, fees) = self.summary.clone().unwrap_or_else(|| {
            (
                self.pot(),
                format!(
                    "Unspecified fee ${}",
                    money::format(self.pot() - self.awards)
                ),
            )
        });
        ensure!(
            self.pot() >= self.awards,
            "payout {} exceeds action ledger {} ({})",
            money::format(self.awards),
            money::format(self.pot()),
            self.format
        );
        let mut s = format!(
            "Poker Hand #{}: Hold'em No Limit (${} /${}) - {}\n",
            self.id,
            money::format(self.sb),
            money::format(self.bb),
            self.date
        )
        .replace(" /", "/");
        s.push_str(&format!(
            "Table '{}' {}-max Seat #{} is the button\n",
            self.table, self.max, self.button
        ));
        for (seat, (name, stack)) in &self.seats {
            s.push_str(&format!(
                "Seat {seat}: {name} (${} in chips)\n",
                money::format(*stack)
            ));
        }
        // Most rooms only emit Dealt to Hero. Mark every active seat so the GG
        // ledger does not mistakenly discard all opponents.
        for (name, _) in self.seats.values() {
            if self.participants.contains(name)
                || self
                    .lines
                    .iter()
                    .any(|line| line.starts_with(&format!("Dealt to {name} [")))
            {
                s.push_str(&format!("Dealt to {name}\n"));
            }
        }
        s.push_str(&self.lines.join("\n"));
        s.push_str(&format!(
            "\n*** SUMMARY ***\nTotal pot ${total} | {fees}\n",
            total = money::format(total)
        ));
        Ok(Normalized {
            text: s,
            currency: self.currency,
            format: self.format,
            timezone: self.timezone,
            tournament: None,
        })
    }
}

pub fn normalize(raw: &str, profile: &Profile) -> Result<Normalized> {
    let raw = raw
        .trim()
        .trim_start_matches('\u{feff}')
        .trim_start_matches("ï»¿");
    if let Some(hand) = tournament_text(raw)? {
        return Ok(hand);
    }
    if raw.starts_with("Poker Hand #") {
        return Ok(Normalized {
            text: raw.into(),
            currency: "USD".into(),
            format: "gg",
            timezone: None,
            tournament: None,
        });
    }
    if raw.starts_with('<') {
        ensure!(
            !raw.contains("<!DOCTYPE") && !raw.contains("<!ENTITY"),
            "XML DTD/entities are unsupported"
        );
        if raw.contains("<session") {
            return ipoker(raw, profile);
        }
        if raw.contains("<Game ") {
            return microgaming(raw, profile);
        }
        if raw.contains("<description ") {
            return merge(raw, profile);
        }
        bail!("Unsupported XML hand-history format");
    }
    let first = raw.lines().next().context("empty hand")?;
    let lower = first.to_lowercase();
    ensure!(
        !lower.contains("tournament")
            && !lower.contains("omaha")
            && !lower.contains("short deck")
            && !lower.contains("6+")
            && !lower.contains("bomb"),
        "unsupported game: NLHE cash required"
    );
    if first.starts_with("PokerStars ")
        || first.starts_with("CoinPoker Hand #")
        || first.starts_with("Hand #")
        || first.starts_with("Winamax Poker -")
    {
        return stars_style(raw, false);
    }
    if first.starts_with("Full Tilt Poker Game #") {
        return full_tilt(raw);
    }
    if first.starts_with("Game started at:") {
        return winning_legacy(raw);
    }
    if first.starts_with("#Game No :") || first.starts_with("***** Hand History for Game ") {
        return party(raw);
    }
    if first.starts_with("Hand#") {
        return cake(raw);
    }
    if first.starts_with("***** History for hand ") {
        return ongame(raw, profile);
    }
    if first.starts_with("Game #") {
        return unibet(raw);
    }
    if ["Ignition Hand #", "Bovada Hand #", "Bodog Hand #"]
        .iter()
        .any(|p| first.starts_with(p))
    {
        return ignition(raw, profile);
    }
    bail!("Unsupported hand-history format; see supported rooms and export instructions")
}

/// Tournament betting uses chips even when the header advertises a cash buy-in.
/// Only amount fields are rewritten; player names, cards and source IDs survive.
fn tournament_text(raw: &str) -> Result<Option<Normalized>> {
    let first = raw.lines().next().unwrap_or("");
    let is_tournament = first.contains("Tournament")
        || (first.starts_with("Full Tilt Poker Game #") && !first.contains(": Table "));
    if !is_tournament {
        return Ok(None);
    }
    ensure!(
        !first.to_lowercase().contains("cashout")
            && !first.to_lowercase().contains("omaha")
            && !first.contains("6+")
            && !first.to_lowercase().contains("short deck"),
        "unsupported tournament variant"
    );
    let (format, hand_id, mut meta, sb, bb, stamp, tz, header) = if first.starts_with("PokerStars ")
        || first.starts_with("Poker Hand #")
    {
        let c = re(r"^(?:PokerStars (?:Hand|Game)|Poker Hand) #([^:]+): Tournament #([^,]+), (.*?)Hold'em No Limit\s*-\s*Level\s*([^ (]+)\s*\(([^/]+)/([^)]*)\)")
                .captures(first).context("unsupported Stars/GG tournament header")?;
        let format = if first.starts_with("PokerStars") {
            "pokerstars"
        } else {
            "gg"
        };
        let meta = Tournament {
            id: Some(c[2].into()),
            name: c[3].trim().into(),
            level: Some(c[4].into()),
            buy_in: if format == "pokerstars" {
                Some(c[3].trim().into())
            } else {
                None
            },
        };
        (
            format,
            c[1].to_string(),
            meta,
            chip_value(&c[5])?,
            chip_value(&c[6])?,
            date(first)?,
            timezone(first),
            String::new(),
        )
    } else if first.starts_with("Game Hand #") || first.starts_with("Hand #") {
        let c = re(r"^(?:Game )?Hand #([^ ]+) - (.*?)Tournament #([^ ]+) - Holdem\(No Limit\) - Level ([^ ]+) \(([^/]+)/([^)]*)\)")
                .captures(first).context("unsupported WPN tournament header")?;
        let sb = chip_value(&c[5])?;
        let bb = chip_value(&c[6])?;
        let stamp = date(first)?;
        let header = format!(
            "Hand #{} - Holdem(No Limit) - ${}/${} - {stamp}",
            &c[1],
            money::format(sb),
            money::format(bb)
        );
        (
            "winning",
            c[1].to_string(),
            Tournament {
                id: Some(c[3].into()),
                name: c[2].trim().into(),
                level: Some(c[4].into()),
                buy_in: None,
            },
            sb,
            bb,
            stamp,
            timezone(first),
            header,
        )
    } else if first.starts_with("Winamax Poker - Tournament ") {
        let c = re(r#"Tournament "(.*?)" buyIn: (.*?) level: ([^ ]+) - HandId: #([^ ]+) - Holdem no limit \(([^)]+)\)"#)
                .captures(first).context("unsupported Winamax tournament header")?;
        let blinds: Vec<_> = c[5].split('/').collect();
        ensure!(matches!(blinds.len(), 2 | 3), "invalid Winamax blinds");
        let sb = chip_value(blinds[blinds.len() - 2])?;
        let bb = chip_value(blinds[blinds.len() - 1])?;
        let stamp = date(first)?;
        let header = format!(
            "Winamax Poker - CashGame - HandId: #{} - Holdem no limit (${}/${}) - {stamp}",
            &c[4],
            money::format(sb),
            money::format(bb)
        );
        let id = re(r"Table:.*\((\d+)\)#")
            .captures(raw)
            .map(|c| c[1].to_string());
        (
            "winamax",
            c[4].to_string(),
            Tournament {
                id,
                name: c[1].into(),
                level: Some(c[3].into()),
                buy_in: Some(c[2].into()),
            },
            sb,
            bb,
            stamp,
            timezone(first),
            header,
        )
    } else if first.starts_with("Full Tilt Poker Game #") {
        let c = re(r"^Full Tilt Poker Game #([^:]+): (.+) \((\d+)\),? Table (\d+) - ([\d,.]+)/([\d,.]+)(?: Ante [\d,.]+)? - No Limit Hold'em - (\d+:\d+:\d+) ([A-Z]+) - (\d{4}/\d{2}/\d{2})")
                .captures(first).context("unsupported Full Tilt tournament header")?;
        let stamp = format!("{} {}", &c[9], &c[7]);
        let tz = timezone(&format!("{} {}", &c[7], &c[8]));
        (
            "fulltilt",
            c[1].to_string(),
            Tournament {
                id: Some(c[3].into()),
                name: c[2].into(),
                level: None,
                buy_in: None,
            },
            chip_value(&c[5])?,
            chip_value(&c[6])?,
            stamp,
            tz,
            String::new(),
        )
    } else {
        bail!("unsupported tournament text format")
    };
    ensure!(sb > 0 && bb >= sb, "invalid tournament blinds");
    // A tournament name is metadata; never deduce buy-in from a guarantee/title.
    meta.name = meta.name.trim().to_string();
    let header = if header.is_empty() {
        format!(
            "PokerStars Hand #{hand_id}: Hold'em No Limit (${}/${}) - {stamp}",
            money::format(sb),
            money::format(bb)
        )
    } else {
        header
    };
    let seat = re(r"^Seat (\d+): (.+?) \(\s*\$?([\d,.]+)(?: in chips)?\s*\)(.*)$");
    let names: Vec<_> = raw
        .lines()
        .filter_map(|line| seat.captures(line.trim()).map(|c| c[2].to_string()))
        .collect();
    let number = re(r"\$?(\d[\d,]*(?:\.\d+)?)");
    let amounts = |text: &str| -> Result<String> {
        ensure!(
            !re(r"(?:^|[\s(])-\$?\d").is_match(text),
            "negative tournament amount"
        );
        for token in re(r"\d[\d,.]*[A-Za-z]*").find_iter(text) {
            chip_value(token.as_str())?;
        }
        Ok(number
            .replace_all(text, |c: &regex::Captures| {
                format!("${}", c[1].replace(',', ""))
            })
            .into_owned())
    };
    let mut out = format!("{header}\n");
    if format == "fulltilt" {
        let button = re(r"The button is in seat #(\d+)")
            .captures(raw)
            .context("missing Full Tilt button")?[1]
            .to_string();
        out.push_str(&format!(
            "Table '{}' 9-max Seat #{button} is the button\n",
            meta.id.as_deref().unwrap()
        ));
    }
    for source in raw.lines().skip(1).map(str::trim) {
        let mut line = if let Some(c) = seat.captures(source) {
            format!(
                "Seat {}: {} (${} in chips){}",
                &c[1],
                &c[2],
                money::format(chip_value(&c[3])?),
                &c[4]
            )
        } else if source.starts_with("Total pot ") {
            amounts(source)?
        } else if let Some(c) =
            re(r"^Uncalled bet \((\$?[\d,.]+)\) returned to (.+)$").captures(source)
        {
            format!("Uncalled bet ({}) returned to {}", amounts(&c[1])?, &c[2])
        } else if format == "fulltilt"
            && re(r"^Uncalled bet of ([\d,.]+) returned to (.+)$").is_match(source)
        {
            let c = re(r"^Uncalled bet of ([\d,.]+) returned to (.+)$")
                .captures(source)
                .unwrap();
            format!("Uncalled bet ({}) returned to {}", amounts(&c[1])?, &c[2])
        } else if source.starts_with("Uncalled bet ") {
            bail!("invalid tournament return amount")
        } else {
            source.to_string()
        };
        if format == "fulltilt"
            && (source.starts_with("The button is in seat")
                || source.contains(" seconds left to act")
                || source.ends_with("has timed out")
                || source.ends_with("is sitting out")
                || source.ends_with("has returned"))
        {
            continue;
        }
        for name in &names {
            let Some(text) = source
                .strip_prefix(&format!("{name}: "))
                .or_else(|| source.strip_prefix(&format!("{name} ")))
            else {
                continue;
            };
            // Explicit cash rewards / finish notifications are not chip payouts.
            if text.starts_with("finished the tournament ")
                || (text.starts_with("wins ")
                    && text.contains(" for eliminating ")
                    && text.contains("bounty"))
            {
                line.clear();
                break;
            }
            if text.starts_with("collected ") {
                let c = re(r"^collected (\$?[\d,.]+)(.*)$")
                    .captures(text)
                    .context("invalid chip payout")?;
                ensure!(
                    c[2].starts_with(" from "),
                    "unsupported tournament payout amount/suffix"
                );
                line = format!("{name} collected {}{}", amounts(&c[1])?, &c[2]);
            } else if format == "fulltilt" && text.starts_with("wins the ") {
                let c = re(r"^wins the (?:pot|main pot|side pot(?: #\d+)?) \((\$?[\d,.]+)\)")
                    .captures(text)
                    .context("invalid Full Tilt chip payout")?;
                line = format!("{name} collected {} from pot", amounts(&c[1])?);
            } else if ["posts ", "calls ", "bets ", "raises "]
                .iter()
                .any(|p| text.starts_with(p))
            {
                ensure!(
                    !re(r"(?:^|[\s(])-\$?\d").is_match(text),
                    "negative tournament contribution"
                );
                let text = if format == "fulltilt" {
                    text.replace("raises to ", "raises 0 to ")
                } else {
                    text.into()
                };
                let text = text.replace("posts big blind ante ", "posts ante ");
                line = format!("{name}: {}", amounts(&text)?);
            }
            break;
        }
        // WPN payouts can appear only in the summary. Seat identifiers stay intact.
        if format == "winning" {
            if let Some(c) = re(r"^(Seat \d+: .* won) (\$?[\d,.]+)(.*)$").captures(source) {
                ensure!(
                    c[3].is_empty() || c[3].starts_with(char::is_whitespace),
                    "unsupported WPN payout suffix"
                );
                line = format!("{} {}{}", &c[1], amounts(&c[2])?, &c[3]);
            }
        }
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }
    let mut n = stars_style(&out, false)?;
    n.currency = "CHIPS".into();
    n.format = format;
    n.timezone = tz;
    n.tournament = Some(meta);
    Ok(Some(n))
}

fn stars_style(raw: &str, derived_summary: bool) -> Result<Normalized> {
    let first = raw.lines().next().unwrap();
    let (format, id, sb, bb, curr, stamp) = if first.starts_with("Winamax") {
        let c = re(r"HandId: #([^ ]+).*Holdem no limit \(([^/]+)/([^\)]+)\)")
            .captures(first)
            .context("unsupported Winamax cash header")?;
        (
            "winamax",
            c[1].to_string(),
            value(&c[2])?,
            value(&c[3])?,
            currency(&c[2])?,
            date(first)?,
        )
    } else if first.starts_with("CoinPoker") {
        let c = re(r"^CoinPoker Hand #([^:]+): NLH \(([^/]+)/([^/)]+)(?:/[^)]+)?\)")
            .captures(first)
            .context("unsupported CoinPoker NLH header")?;
        (
            "coinpoker",
            c[1].to_string(),
            value(&c[2])?,
            value(&c[3])?,
            currency(&c[2])?,
            date(first)?,
        )
    } else if first.starts_with("Hand #") {
        let c = re(r"^Hand #([^ ]+) - Holdem\(No Limit\) - ([^/]+)/([^ ]+) -")
            .captures(first)
            .context("unsupported WPN cash header")?;
        (
            "winning",
            c[1].to_string(),
            value(&c[2])?,
            value(&c[3])?,
            currency(first)?,
            date(first)?,
        )
    } else {
        let c = re(r"(?:Hand|Game) #([^:]+):\s*(?:Zoom )?Hold'em No Limit \(([^/]+)/([^\)]+)\)")
            .captures(first)
            .context("unsupported PokerStars NLHE cash header")?;
        (
            "pokerstars",
            c[1].to_string(),
            value(&c[2])?,
            value(&c[3])?,
            currency(&c[0])?,
            date(first)?,
        )
    };
    let mut b = Builder::new(&id, stamp, sb, bb, curr, format, timezone(first));
    let table = re(r"^(?:Table:? )?'(.+)' (\d+)-max(?: \(real money\))? Seat #(\d+) is the button");
    let wpn_table = re(r"^(.+) (\d+)-max Seat #(\d+) is the button");
    let seat = re(r"^Seat (\d+): (.+?) \(([^)]+?)(?: in chips)?\)(.*)$");
    let mut summary = false;
    let mut summary_payouts = false;
    for source in raw.lines().skip(1) {
        let source = source.trim();
        let line = card_text(&cash_text(source));
        if source.starts_with("*** SUMMARY") {
            summary = true;
            summary_payouts = b.awards == 0;
            continue;
        }
        if summary {
            if source.starts_with("Total pot ") {
                let total = value(line.strip_prefix("Total pot ").unwrap())?;
                let fees = line
                    .split_once('|')
                    .map(|(_, f)| f.trim().to_string())
                    .unwrap_or_else(|| "Rake $0".into());
                let fee_sum = re(r"\$([\d.]+)")
                    .captures_iter(&fees)
                    .map(|c| money::parse(&c[1]))
                    .collect::<Result<Vec<_>>>()?
                    .iter()
                    .sum::<i64>();
                b.summary = Some((
                    total
                        + if matches!(format, "winning" | "winamax") {
                            fee_sum
                        } else {
                            0
                        },
                    fees,
                ));
            }
            // Modern WPN walks may only report the payout in the seat summary.
            if format == "winning" && summary_payouts {
                if let Some(c) = re(r"^Seat (\d+): .* won \$([\d.]+)").captures(&line) {
                    let name = b
                        .seats
                        .get(&c[1].parse()?)
                        .context("unknown WPN summary seat")?
                        .0
                        .clone();
                    b.action(&name, "collect", money::parse(&c[2])?, false, false)?;
                }
            }
            continue;
        }
        if let Some(c) = table.captures(&line).or_else(|| wpn_table.captures(&line)) {
            b.table = c[1].into();
            b.max = c[2].parse()?;
            b.button = c[3].parse()?;
            continue;
        }
        if let Some(c) = seat.captures(&line) {
            if !c[4].contains("sitting out") {
                b.seat(c[1].parse()?, &c[2], value(&c[3])?)?;
            }
            continue;
        }
        if line.starts_with("*** ANTE/BLINDS")
            || line.starts_with("*** PRE-FLOP")
            || line.starts_with("*** HOLE CARDS")
        {
            continue;
        }
        if line.starts_with("Dealt to ") {
            b.lines.push(line);
            continue;
        }
        if line.starts_with("***") {
            if line.contains("FLOP") || line.contains("TURN") || line.contains("RIVER") {
                b.board(line);
            } else {
                b.lines.push(line);
            }
            continue;
        }
        if format == "winning" && (line.starts_with("Main pot ") || line.starts_with("Side pot ")) {
            continue;
        }
        if let Some(c) = re(r"^Uncalled bet \(\$([\d.]+)\) returned to (.+)$").captures(&line) {
            b.action(&c[2], "return", money::parse(&c[1])?, false, false)?;
            continue;
        }
        if let Some(c) = re(r"^(.+?) collected \$([\d.]+) from ").captures(&line) {
            b.action(&c[1], "collect", money::parse(&c[2])?, false, false)?;
            continue;
        }
        let mut found = false;
        for (name, _) in b.seats.values().cloned().collect::<Vec<_>>() {
            if let Some(text) = line
                .strip_prefix(&format!("{name}: "))
                .or_else(|| line.strip_prefix(&format!("{name} ")))
            {
                found = true;
                let all = text.contains("all-in");
                if format == "coinpoker" && text.starts_with("posts auto big blind") {
                    ensure!(value(text)?==b.bb,"CoinPoker auto big blind differs from header; export needs source correction");
                }
                let text = text
                    .replace("posts the ", "posts ")
                    .replace("posts auto big blind", "posts big blind");
                let kind = [
                    ("posts small blind", "posts small blind"),
                    ("posts big blind", "posts big blind"),
                    ("posts ante", "posts ante"),
                    ("posts straddle", "posts straddle"),
                    ("posts dead blind", "posts dead blind"),
                    ("folds", "fold"),
                    ("checks", "check"),
                    ("calls", "calls"),
                    ("bets", "bets"),
                ]
                .into_iter()
                .find(|(p, _)| text.starts_with(p))
                .map(|(_, k)| k);
                if let Some(kind) = kind {
                    b.action(
                        &name,
                        kind,
                        if matches!(kind, "fold" | "check") {
                            0
                        } else {
                            value(&text)?
                        },
                        false,
                        all,
                    )?;
                } else if text.starts_with("raises") || text.starts_with("caps ") {
                    let total = if text.starts_with("caps ") {
                        value(&text)?
                    } else {
                        value(
                            text.split_once(" to ")
                                .context("raise missing total-to amount")?
                                .1,
                        )?
                    };
                    b.action(
                        &name,
                        "raise",
                        total,
                        true,
                        all || text.starts_with("caps "),
                    )?;
                } else {
                    b.lines.push(format!(
                        "{name}: {}",
                        text.replace("does not show", "doesn't show")
                    ));
                }
                break;
            }
        }
        if !found && !line.is_empty() {
            b.lines.push(line);
        }
    }
    let complete = summary && (b.summary.is_some() || derived_summary);
    // Some archived Winamax exports include uncalled chips in both gross pot
    // and payout. Do not invent a return when that explicit ledger balances.
    let infer_return = format == "winamax" && b.pot() != b.awards;
    b.finish(infer_return, complete)
}

fn full_tilt(raw: &str) -> Result<Normalized> {
    let first = raw.lines().next().unwrap();
    let c=re(r"^Full Tilt Poker Game #([^:]+): Table (.+) - NL Hold'em - ([^/]+)/([^ ]+) - (\d+:\d+:\d+) ([A-Z]+) - (\d{4}/\d{2}/\d{2})").captures(first).context("unsupported Full Tilt NLHE cash header")?;
    let header = format!(
        "PokerStars Hand #{}: Hold'em No Limit ({}/{}) - {} {} {}",
        &c[1], &c[3], &c[4], &c[7], &c[5], &c[6]
    );
    let button = re(r"The button is in seat #(\d+)")
        .captures(raw)
        .context("missing Full Tilt button")?[1]
        .to_string();
    let mut out = format!(
        "{header}\nTable '{}' 9-max Seat #{button} is the button\n",
        &c[2]
    );
    for s in raw.lines().skip(1) {
        let s = s.trim();
        if s.starts_with("The button is") {
            continue;
        }
        if let Some(c) = re(r"^Seat (\d+): (.+?) \((\$[^)]+)\)(.*)$").captures(s) {
            out.push_str(&format!(
                "Seat {}: {} ({} in chips){}\n",
                &c[1], &c[2], &c[3], &c[4]
            ));
            continue;
        }
        if let Some(c) =
            re(r"^(.+?) wins the (?:pot|main pot|side pot(?: #\d+)?) \((\$[^)]+)\)").captures(s)
        {
            out.push_str(&format!("{} collected {} from pot\n", &c[1], &c[2]));
            continue;
        }
        if s.contains(" seconds left to act")
            || s.ends_with("has timed out")
            || re(r"^.+ adds \$[\d.]+$").is_match(s)
        {
            continue;
        }
        let mut line = s
            .replace("raises to ", "raises $0 to ")
            .replace("posts the small blind of ", "posts small blind ")
            .replace("posts the big blind of ", "posts big blind ");
        for marker in [
            " posts ",
            " calls ",
            " bets ",
            " raises ",
            " folds",
            " checks",
            " shows ",
            " mucks",
            " doesn't show",
        ] {
            if let Some(i) = line.find(marker) {
                line.insert(i, ':');
                break;
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
    let mut n = stars_style(&out, false)?;
    n.format = "fulltilt";
    Ok(n)
}

fn winning_legacy(raw: &str) -> Result<Normalized> {
    let header = raw.lines().nth(1).context("missing WPN game header")?;
    ensure!(
        !header.contains(", Table ") && !re(r"(?m)^Player .+ ante \(").is_match(raw),
        "legacy WPN tournament format is unsupported"
    );
    let c = re(r"^Game ID: (\d+) ([\d.]+)/([\d.]+) (.+)\(Hold'em\)")
        .captures(header)
        .context("unsupported legacy WPN cash header")?;
    let mut b = Builder::new(
        &c[1],
        date(raw)?,
        money::parse(&c[2])?,
        money::parse(&c[3])?,
        "USD".into(),
        "winning-legacy",
        None,
    );
    b.table = c[4].trim().into();
    let mut summary = false;
    let mut complete = false;
    let mut hole_cards: HashMap<String, Vec<String>> = HashMap::new();
    for line in raw.lines().skip(2).map(str::trim) {
        let line = card_text(line);
        if let Some(c) = re(r"^Seat (\d+) is the button").captures(&line) {
            b.button = c[1].parse()?;
            continue;
        }
        if let Some(c) = re(r"^Seat (\d+): (.+?) \(([\d.]+)\)").captures(&line) {
            b.seat(c[1].parse()?, &c[2], money::parse(&c[3])?)?;
            continue;
        }
        if line.starts_with("------ Summary") {
            summary = true;
            b.lines.push("*** SHOWDOWN ***".into());
            continue;
        }
        if line.starts_with("Game ended at:") {
            complete = true;
            continue;
        }
        if summary {
            if let Some(c) = re(r"^Pot: ([\d.]+)\. Rake ([\d.]+)").captures(&line) {
                let rake = money::parse(&c[2])?;
                b.summary = Some((
                    money::parse(&c[1])? + rake,
                    format!("Rake ${}", money::format(rake)),
                ));
            }
            if let Some(c) =
                re(r"^\*?Player (.+?) (?:shows:|mucks|does not show).*Collects: ([\d.]+)\.")
                    .captures(&line)
            {
                let name = &c[1];
                let cs = crate::parser::cards(&line);
                if !cs.is_empty() && line.contains("shows:") {
                    b.lines.push(format!("{name}: shows [{}]", cs.join(" ")));
                }
                let v = money::parse(&c[2])?;
                if v > 0 {
                    b.action(name, "collect", v, false, false)?;
                }
            }
            continue;
        }
        if line.starts_with("***") {
            b.board(line.replace("***:", "***"));
            continue;
        }
        if let Some(c) = re(r"^Uncalled bet \(([\d.]+)\) returned to (.+)").captures(&line) {
            b.action(&c[2], "return", money::parse(&c[1])?, false, false)?;
            continue;
        }
        if let Some(text) = line.strip_prefix("Player ") {
            for (name, _) in b.seats.values().cloned().collect::<Vec<_>>() {
                if let Some(text) = text.strip_prefix(&format!("{name} ")) {
                    if text.starts_with("received card:") {
                        let cs = crate::parser::cards(text);
                        if !cs.is_empty() {
                            let cards = hole_cards.entry(name.clone()).or_default();
                            cards.extend(cs);
                            b.lines
                                .push(format!("Dealt to {name} [{}]", cards.join(" ")));
                        }
                        break;
                    }
                    if text.starts_with("received a card") || text.starts_with("mucks") {
                        break;
                    }
                    let kind = if text.starts_with("has small blind") {
                        "posts small blind"
                    } else if text.starts_with("has big blind") {
                        "posts big blind"
                    } else if text.starts_with("raises") {
                        "raise"
                    } else if text.starts_with("calls") {
                        "calls"
                    } else if text.starts_with("bets") {
                        "bets"
                    } else if text.starts_with("folds") {
                        "fold"
                    } else if text.starts_with("checks") {
                        "check"
                    } else if text.starts_with("allin") {
                        let level = b.live.values().copied().max().unwrap_or(0);
                        if value(text)? + b.live.get(&name).copied().unwrap_or(0) > level {
                            "raise"
                        } else {
                            "calls"
                        }
                    } else {
                        b.lines.push(format!("{name}: {text}"));
                        break;
                    };
                    b.action(
                        &name,
                        kind,
                        if matches!(kind, "fold" | "check") {
                            0
                        } else {
                            value(text)?
                        },
                        false,
                        text.starts_with("allin"),
                    )?;
                    break;
                }
            }
        }
    }
    b.finish(false, summary && complete)
}

fn party(raw: &str) -> Result<Normalized> {
    let pacific = raw.starts_with("#Game No");
    let tournament = raw.lines().find_map(|line| {
        re(r"^Tournament #([^ ]+) (.*?) - Table #([^ ]+)")
            .captures(line)
            .map(|c| Tournament {
                id: Some(c[1].into()),
                name: format!("Tournament #{}", &c[1]),
                level: None,
                buy_in: Some(c[2].into()),
            })
    });
    ensure!(
        !raw.contains("Tournament #") || tournament.is_some(),
        "unrecognized tournament metadata"
    );
    ensure!(
        pacific || (!raw.contains("Buy-in") && !raw.contains("Trny:") && !raw.contains("Tourney")),
        "Party tournament format is not supported"
    );
    let id = re(r"(?:#Game No : |Hand History for Game )(\d+)")
        .captures(raw)
        .context("missing Party/888 hand ID")?[1]
        .to_string();
    let header = raw
        .lines()
        .find(|l| l.contains("No Limit Holdem") || l.contains("NL Texas Hold'em"))
        .context("unsupported Party/888 NLHE cash header")?;
    let limit = re(r"([^ /]+)/([^ ]+)")
        .captures(header)
        .context("missing cash blinds")?;
    let stamp = if pacific {
        let d = header
            .split_once("*** ")
            .context("missing 888 timestamp")?
            .1;
        NaiveDateTime::parse_from_str(d, "%d %m %Y %H:%M:%S")?
            .format("%Y/%m/%d %H:%M:%S")
            .to_string()
    } else {
        let d = header
            .split_once(" - ")
            .context("missing Party timestamp")?
            .1;
        let d = re(r"\b(EST|EDT|ET|UTC|CET|CEST) ").replace(d, "");
        NaiveDateTime::parse_from_str(&d, "%A, %B %d, %H:%M:%S %Y")?
            .format("%Y/%m/%d %H:%M:%S")
            .to_string()
    };
    let mut b = Builder::new(
        &id,
        stamp,
        value(&limit[1])?,
        value(&limit[2])?,
        currency(header)?,
        if pacific { "888" } else { "partypoker" },
        timezone(header),
    );
    let mut complete = false;
    for source in raw.lines().map(str::trim) {
        let line = card_text(&cash_text(source));
        if let Some(c) = re(r"^Table (.+?)(?: (\d+) Max)? \(Real Money\)").captures(&line) {
            b.table = c[1].into();
            if let Some(m) = c.get(2) {
                b.max = m.as_str().parse()?;
            }
            continue;
        }
        if let Some(c) = re(r"^Seat (\d+) is the button").captures(&line) {
            b.button = c[1].parse()?;
            continue;
        }
        if let Some(c) = re(r"^Total number of players : \d+/(\d+)").captures(&line) {
            b.max = c[1].parse()?;
            continue;
        }
        if let Some(c) = re(r"^Seat (\d+): (.+?) \( (\$[^ ]+) \)").captures(&line) {
            b.seat(c[1].parse()?, &c[2], value(&c[3])?)?;
            continue;
        }
        if line.starts_with("Dealt to ") {
            b.lines.push(line);
            continue;
        }
        let lower = line.to_lowercase();
        if lower.starts_with("** dealing flop") {
            b.board(format!("*** FLOP *** {}", bracket_part(&line)));
            continue;
        }
        if lower.starts_with("** dealing turn") {
            b.board(format!("*** TURN *** {}", bracket_part(&line)));
            continue;
        }
        if lower.starts_with("** dealing river") {
            b.board(format!("*** RIVER *** {}", bracket_part(&line)));
            continue;
        }
        if lower.starts_with("** summary") {
            complete = true;
            b.lines.push("*** SHOWDOWN ***".into());
            continue;
        }
        for (name, _) in b.seats.values().cloned().collect::<Vec<_>>() {
            if let Some(text) = line.strip_prefix(&format!("{name} ")) {
                let kind = if text.starts_with("posts small blind") {
                    "posts small blind"
                } else if text.starts_with("posts big blind") {
                    "posts big blind"
                } else if text.starts_with("posts ante") {
                    "posts ante"
                } else if text.starts_with("calls") {
                    "calls"
                } else if text.starts_with("bets") {
                    "bets"
                } else if text.starts_with("raises") {
                    "raise"
                } else if text.starts_with("folds") {
                    "fold"
                } else if text.starts_with("checks") {
                    "check"
                } else if text.starts_with("collected") || text.starts_with("wins ") {
                    complete = true;
                    "collect"
                } else if text.starts_with("shows ") {
                    b.lines.push("*** SHOWDOWN ***".into());
                    b.lines.push(format!("{name}: {text}"));
                    break;
                } else if text.starts_with("does not show") || text.starts_with("mucks") {
                    break;
                } else {
                    b.lines.push(format!("{name}: {text}"));
                    break;
                };
                // Party and 888 bracketed raises are incremental contributions.
                b.action(
                    &name,
                    kind,
                    if matches!(kind, "fold" | "check") {
                        0
                    } else {
                        value(text)?
                    },
                    false,
                    text.contains("all-in"),
                )?;
                break;
            }
        }
    }
    if tournament.is_some() {
        if b.pot() != b.awards {
            b.uncalled()?;
        }
        ensure!(
            b.pot() == b.awards,
            "tournament chip payouts do not reconcile"
        );
        b.summary = Some((b.pot(), "Rake $0".into()));
    }
    let mut n = b.finish(tournament.is_none(), complete)?;
    if tournament.is_some() {
        n.currency = "CHIPS".into();
    }
    n.tournament = tournament;
    Ok(n)
}
fn bracket_part(s: &str) -> String {
    s.find('[').map(|i| s[i..].to_string()).unwrap_or_default()
}

fn cake(raw: &str) -> Result<Normalized> {
    let first = raw.lines().next().unwrap();
    let c=re(r"^Hand#([^ ]+) - (.+?) -- ([^/]+)/([^ ]+) NL Hold'em -- (\d{4}/\d{2}/\d{2}) - (\d+:\d+:\d+)").captures(first).context("unsupported Cake NLHE cash header")?;
    let mut out = format!(
        "PokerStars Hand #{}: Hold'em No Limit ({}/{}) - {} {}\n",
        &c[1], &c[3], &c[4], &c[5], &c[6]
    );
    let button = re(r"Seat (\d+): .* DEALER")
        .captures(raw)
        .context("missing Cake dealer")?[1]
        .to_string();
    out.push_str(&format!(
        "Table '{}' 10-max Seat #{button} is the button\n",
        &c[2]
    ));
    for source in raw.lines().skip(1) {
        let line = card_text(source);
        if let Some(c) = re(r"^(.+?): returns uncalled bet (\$[\d.]+)").captures(&line) {
            out.push_str(&format!("Uncalled bet ({}) returned to {}\n", &c[2], &c[1]));
            continue;
        }
        if let Some(c) = re(r"^(.+?) wins (\$[\d.]+)").captures(&line) {
            let name = c[1].strip_suffix(':').unwrap_or(&c[1]);
            out.push_str(&format!("{name} collected {} from pot\n", &c[2]));
            continue;
        }
        out.push_str(
            &line
                .replace("raises to ", "raises $0 to ")
                .replace("***SHOW DOWN***", "*** SHOW DOWN ***"),
        );
        out.push('\n');
    }
    // No total/rake footer: Builder records the settlement difference as an
    // unspecified fee and leaves eligible equity excluded.
    out.push_str("*** SUMMARY ***\n");
    let mut n = stars_style(&out, true)?;
    n.format = "cake";
    Ok(n)
}

fn unibet(raw: &str) -> Result<Normalized> {
    let first = raw.lines().next().unwrap();
    let c=re(r"^Game #([^:]+): Table (.+) - ([\d.]+)/([\d.]+) - No Limit Hold'Em(?: Banzai)? - (\d+:\d+:\d+) (\d{4}/\d{2}/\d{2})").captures(first).context("unsupported Unibet cash header")?;
    let header = format!(
        "PokerStars Hand #{}: Hold'em No Limit ({}{}/{}{}) - {} {}",
        &c[1],
        if first.contains('€') { "€" } else { "$" },
        &c[3],
        if first.contains('€') { "€" } else { "$" },
        &c[4],
        &c[6],
        &c[5]
    );
    let names = re(r"^(.+) has the button");
    let button_name = raw
        .lines()
        .find_map(|s| names.captures(s).map(|c| c[1].to_string()))
        .context("missing Unibet button")?;
    let seat = re(r"^Seat (\d+): (.+?) \(([^)]+)\)");
    let button = raw
        .lines()
        .find_map(|s| {
            seat.captures(s)
                .filter(|c| c[2] == button_name)
                .map(|c| c[1].to_string())
        })
        .context("missing Unibet dealer seat")?;
    let mut out = format!(
        "{header}\nTable '{}' 6-max Seat #{button} is the button\n",
        &c[2]
    );
    for source in raw.lines().skip(1) {
        let line = cash_text(source);
        if let Some(c) = seat.captures(&line) {
            out.push_str(&format!("Seat {}: {} ({} in chips)\n", &c[1], &c[2], &c[3]));
            continue;
        }
        if line.starts_with("*** Summary") {
            out.push_str("*** SUMMARY ***\n");
            continue;
        }
        if line.starts_with("Total pot ") {
            out.push_str(&line.replace(" Rake ", " | Rake "));
            out.push('\n');
            continue;
        }
        if let Some(c) = re(r"^(.+) wins (\$[\d.]+)").captures(&line) {
            out.push_str(&format!("{} collected {} from pot\n", &c[1], &c[2]));
            continue;
        }
        let upper = line.to_uppercase();
        if line.starts_with("***") {
            if upper.starts_with("*** FLOP")
                || upper.starts_with("*** TURN")
                || upper.starts_with("*** RIVER")
            {
                let marker = upper.split('[').next().unwrap_or("").trim();
                out.push_str(&format!("{marker} {}\n", bracket_part(&line)));
            }
            continue;
        }
        if line.starts_with("Dealt in ") || line.contains(" has the button") {
            continue;
        }
        out.push_str(&line);
        out.push('\n');
    }
    let mut n = stars_style(&out, false)?;
    n.format = "unibet";
    Ok(n)
}

fn ignition(raw: &str, profile: &Profile) -> Result<Normalized> {
    let first = raw.lines().next().unwrap();
    let c=re(r"^(?:Ignition|Bovada|Bodog) Hand #([^ ]+)\s+(?:TBL#|Zone Poker ID#)([^ ]+) HOLDEM(?:ZonePoker)? No Limit - (.+)$").captures(first).context("unsupported Ignition cash header")?;
    let blind = |kind: &str| -> Result<i64> {
        let c = re(&format!(r"(?m)^.+ : {kind} \$([\d.]+)"))
            .captures(raw)
            .context("missing Ignition blinds")?;
        money::parse(&c[1])
    };
    let mut b = Builder::new(
        &c[1],
        date(&c[3])?,
        blind("Small Blind")?,
        blind("Big blind")?,
        "USD".into(),
        "ignition",
        None,
    );
    b.table = c[2].into();
    let mut complete = false;
    for source in raw.lines().skip(1) {
        let source = source.replace("  [ME]", " [ME]");
        let line = if let Some(c) = re(r"^(?:Seat \d+: )?(.+? \[ME\])(?: : | \()").captures(&source)
        {
            source.replace(&c[1], &profile.hero)
        } else {
            source
        };
        if line.starts_with("*** SUMMARY") {
            complete = true;
            continue;
        }
        if let Some(c) = re(r"^Seat (\d+): (.+?) \((\$[^ ]+) in chips\)").captures(&line) {
            b.seat(c[1].parse()?, &c[2], value(&c[3])?)?;
            continue;
        }
        if line.contains("Set dealer") {
            b.button = bracket_part(&line)
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse()?;
            continue;
        }
        if line.starts_with("Total Pot(") {
            b.summary = Some((value(&line)?, "Rake $0".into()));
            continue;
        }
        if line.starts_with("***") {
            if line.contains("FLOP") || line.contains("TURN") || line.contains("RIVER") {
                b.board(line);
            } else {
                b.lines.push(line);
            }
            continue;
        }
        if let Some((name, text)) = line.split_once(" : ") {
            let text = text.trim();
            if text.starts_with("Card dealt to a spot") {
                b.lines
                    .push(format!("Dealt to {} {}", name.trim(), bracket_part(text)));
                continue;
            }
            if text.starts_with("Return uncalled portion of bet") {
                b.action(name.trim(), "return", value(text)?, false, false)?;
                continue;
            }
            if text.starts_with("Showdown") {
                b.lines.push("*** SHOWDOWN ***".into());
                continue;
            }
            if text.starts_with("Does not show")
                || text.starts_with("Mucks")
                || text.starts_with("Leave(")
                || text.starts_with("Enter(")
                || text.starts_with("Seat sit out")
            {
                continue;
            }
            let kind = if text.starts_with("Small Blind") {
                "posts small blind"
            } else if text.starts_with("Big blind") {
                "posts big blind"
            } else if text.starts_with("Folds") {
                "fold"
            } else if text.starts_with("Checks") {
                "check"
            } else if text.starts_with("Calls") {
                "calls"
            } else if text.starts_with("Bets") {
                "bets"
            } else if text.starts_with("Raises") {
                "raise"
            } else if text.starts_with("Hand result") {
                "collect"
            } else if text.starts_with("Shows ") {
                b.lines
                    .push(format!("{}: shows {}", name.trim(), bracket_part(text)));
                continue;
            } else if text.starts_with("Does not show") {
                continue;
            } else {
                b.lines.push(format!("{}: {text}", name.trim()));
                continue;
            };
            let v = if matches!(kind, "fold" | "check") {
                0
            } else if kind == "raise" {
                value(
                    text.split_once(" to ")
                        .context("Ignition raise needs total")?
                        .1,
                )?
            } else {
                value(text)?
            };
            b.action(
                name.trim(),
                kind,
                v,
                kind == "raise",
                text.contains("All-in"),
            )?;
        }
    }
    b.finish(false, complete)
}

fn xml_text<'a, 'i>(n: roxmltree::Node<'a, 'i>, tag: &str) -> Option<&'a str> {
    n.children()
        .find(|n| n.has_tag_name(tag))
        .and_then(|n| n.text())
}
fn xml_money(s: &str) -> Result<i64> {
    value(s)
}
fn ipoker(raw: &str, profile: &Profile) -> Result<Normalized> {
    ensure!(
        !raw.contains("<!DOCTYPE") && !raw.contains("<!ENTITY"),
        "XML DTD/entities are unsupported"
    );
    let doc = roxmltree::Document::parse(raw).context("invalid iPoker XML")?;
    let session = doc
        .descendants()
        .find(|n| n.has_tag_name("session"))
        .context("unsupported XML: expected iPoker session")?;
    let general = session
        .children()
        .find(|n| n.has_tag_name("general"))
        .context("missing iPoker session metadata")?;
    ensure!(
        xml_text(general, "mode") == Some("real"),
        "iPoker real-money game required"
    );
    let gt = xml_text(general, "gametype").context("missing iPoker game type")?;
    let tournament_name = xml_text(general, "tournamentname").filter(|s| !s.is_empty());
    let tournament_code =
        xml_text(general, "tournamentcode").filter(|s| !s.is_empty() && *s != "0");
    let tournament = if tournament_name.is_some() || tournament_code.is_some() {
        ensure!(gt == "Holdem NL", "unsupported iPoker tournament variant");
        let table = xml_text(general, "tablename").unwrap_or("");
        let id = tournament_code.map(str::to_string).or_else(|| {
            re(r"(?:\(#?|,\s*)(\d+)\)?$")
                .captures(table)
                .map(|c| c[1].to_string())
        });
        Some(Tournament {
            id,
            name: tournament_name.unwrap_or(table).into(),
            level: xml_text(general, "level").map(str::to_string),
            buy_in: xml_text(general, "buyin").map(str::to_string),
        })
    } else {
        None
    };
    let games: Vec<_> = session
        .children()
        .filter(|n| n.has_tag_name("game"))
        .collect();
    ensure!(games.len() == 1, "expected one iPoker game per parser call");
    let game = games[0];
    let g = game
        .children()
        .find(|n| n.has_tag_name("general"))
        .context("missing game metadata")?;
    let (sb, bb, curr) = if tournament.is_some() {
        // These XML exports omit the scheduled blind level. Require two full
        // blind posts; a short all-in blind cannot establish the level.
        let blind = |kind: &str| -> Result<i64> {
            let posts: Vec<_> = game
                .descendants()
                .filter(|n| n.has_tag_name("action") && n.attribute("type") == Some(kind))
                .collect();
            ensure!(posts.len() == 1, "missing/ambiguous tournament blinds");
            let a = posts[0];
            let name = a.attribute("player").context("missing blind player")?;
            let v = xml_money(a.attribute("sum").context("missing blind amount")?)?;
            let p = g
                .descendants()
                .find(|p| p.has_tag_name("player") && p.attribute("name") == Some(name))
                .context("unknown blind player")?;
            let ante = game
                .descendants()
                .filter(|n| {
                    n.has_tag_name("action")
                        && n.attribute("type") == Some("15")
                        && n.attribute("player") == Some(name)
                })
                .map(|a| xml_money(a.attribute("sum").unwrap_or("")))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum::<i64>();
            ensure!(
                v > 0
                    && xml_money(p.attribute("chips").context("missing blind stack")?)? > v + ante,
                "short/all-in XML blind cannot establish tournament level"
            );
            Ok(v)
        };
        (blind("1")?, blind("2")?, "CHIPS".into())
    } else {
        let limit = re(r"^Holdem NL (.+)/(.+)$")
            .captures(gt)
            .context("unsupported iPoker cash game/blinds")?;
        let curr = match xml_text(general, "tablecurrency").filter(|s| !s.is_empty()) {
            Some(s) => s.to_string(),
            None => currency(gt)?,
        };
        ensure!(
            ["USD", "EUR", "GBP", "CAD", "USDT"].contains(&curr.as_str()),
            "unsupported iPoker currency"
        );
        (xml_money(&limit[1])?, xml_money(&limit[2])?, curr)
    };
    let mut b = Builder::new(
        game.attribute("gamecode").context("missing gamecode")?,
        date(xml_text(g, "startdate").context("missing game startdate")?)?,
        sb,
        bb,
        curr,
        "ipoker",
        None,
    );
    b.table = xml_text(general, "tablename").unwrap_or("iPoker").into();
    let mut wins = vec![];
    let mut source_bets = HashMap::new();
    let mut reveals = vec![];
    for p in g.descendants().filter(|n| n.has_tag_name("player")) {
        let seat: u8 = p
            .attribute("seat")
            .context("missing player seat")?
            .parse()?;
        let name = p.attribute("name").context("missing player name")?;
        b.seat(
            seat,
            name,
            xml_money(p.attribute("chips").context("missing stack")?)?,
        )?;
        if p.attribute("dealer") == Some("1") {
            b.button = seat;
        }
        source_bets.insert(
            name.to_string(),
            xml_money(p.attribute("bet").context("missing player bet")?)?,
        );
        let win = xml_money(p.attribute("win").context("missing player win")?)?;
        if win > 0 {
            wins.push((name.to_string(), win));
        }
    }
    for round in game.children().filter(|n| n.has_tag_name("round")) {
        let no: u8 = round
            .attribute("no")
            .context("missing round number")?
            .parse()?;
        if no >= 2 {
            b.live.clear();
        }
        for c in round.children().filter(|n| n.has_tag_name("cards")) {
            let cs = c
                .text()
                .unwrap_or("")
                .split_whitespace()
                .filter(|c| *c != "X")
                .map(|c| {
                    let suit = c.chars().next().unwrap_or('?').to_ascii_lowercase();
                    let rank = c.get(1..).unwrap_or("").replace("10", "T");
                    format!("{rank}{suit}")
                })
                .collect::<Vec<_>>();
            if c.attribute("type") == Some("Pocket") {
                if !cs.is_empty() {
                    let name = c.attribute("player").context("missing cards player")?;
                    if name == profile.hero {
                        b.lines.push(format!("Dealt to {name} [{}]", cs.join(" ")));
                    } else {
                        // Exported Pocket metadata includes eventual showdown
                        // cards; it is not evidence of a preflop public reveal.
                        reveals.push(format!("{name}: shows [{}]", cs.join(" ")));
                    }
                }
            } else {
                let street = match c.attribute("type") {
                    Some("Flop") => "FLOP",
                    Some("Turn") => "TURN",
                    Some("River") => "RIVER",
                    _ => bail!("unknown iPoker board type"),
                };
                b.board(format!("*** {street} *** [{}]", cs.join(" ")));
            }
        }
        let mut actions = round
            .children()
            .filter(|n| n.has_tag_name("action"))
            .map(|a| {
                Ok((
                    a.attribute("no")
                        .context("missing action sequence")?
                        .parse::<u32>()?,
                    a,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        actions.sort_by_key(|(no, _)| *no);
        ensure!(
            !actions.windows(2).any(|w| w[0].0 == w[1].0),
            "duplicate iPoker action sequence"
        );
        for (_, a) in actions {
            let name = a.attribute("player").context("missing action player")?;
            let v = xml_money(a.attribute("sum").context("missing action amount")?)?;
            let ty = a.attribute("type").context("missing action type")?;
            let kind = match ty {
                "0" => "fold",
                "1" => "posts small blind",
                "2" => "posts big blind",
                "3" => "calls",
                "4" => "check",
                "5" => "bets",
                "15" => "posts ante",
                "23" => "raise",
                "6" | "7" => {
                    let old = b.live.get(name).copied().unwrap_or(0);
                    let level = b.live.values().copied().max().unwrap_or(0);
                    if old + v > level {
                        "raise"
                    } else {
                        "calls"
                    }
                }
                "8" | "9" => continue,
                _ => bail!("unknown iPoker action type {ty}"),
            };
            b.action(name, kind, v, ty == "23", matches!(ty, "6" | "7"))?;
        }
    }
    for (name, v) in source_bets {
        ensure!(
            b.invested.get(&name).copied().unwrap_or(0)
                - b.returned.get(&name).copied().unwrap_or(0)
                == v,
            "iPoker source bet disagrees with action ledger"
        );
    }
    b.lines.push("*** SHOWDOWN ***".into());
    b.lines.extend(reveals);
    for (name, win) in wins {
        b.action(&name, "collect", win, false, false)?;
    }
    if tournament.is_some() {
        ensure!(
            b.pot() == b.awards,
            "iPoker tournament chip payouts do not reconcile"
        );
        b.summary = Some((b.pot(), "Rake $0".into()));
    }
    let mut n = b.finish(false, true)?;
    n.tournament = tournament;
    Ok(n)
}

fn ongame(raw: &str, profile: &Profile) -> Result<Normalized> {
    let id = re(r"History for hand ([^ ]+)")
        .captures(raw)
        .context("missing OnGame ID")?[1]
        .to_string();
    let header = raw
        .lines()
        .find(|l| l.starts_with("Table:"))
        .context("missing OnGame table")?;
    let c = re(r"^Table: (.+) \(NO_LIMIT TEXAS_HOLDEM ([^/]+)/([^,]+), Real money\)")
        .captures(header)
        .context("unsupported OnGame cash game")?;
    let start = raw
        .lines()
        .find_map(|l| l.strip_prefix("Start hand: "))
        .context("missing OnGame timestamp")?;
    let stamp = re(r"\b(CET|CEST|UTC|GMT|EST|EDT|ET) ").replace(start, "");
    let stamp = NaiveDateTime::parse_from_str(&stamp, "%a %b %d %H:%M:%S %Y")?
        .format("%Y/%m/%d %H:%M:%S")
        .to_string();
    let mut b = Builder::new(
        &id,
        stamp,
        value(&c[2])?,
        value(&c[3])?,
        currency(header)?,
        "ongame",
        timezone(start),
    );
    b.table = c[1].into();
    b.button = re(r"Button: seat (\d+)")
        .captures(raw)
        .context("missing OnGame button")?[1]
        .parse()?;
    let mut complete = false;
    let mut gross = None;
    let mut rake = 0;
    for source in raw.lines().map(str::trim) {
        let line = card_text(&cash_text(source));
        if line == "Summary:" {
            complete = true;
            continue;
        }
        if complete {
            if let Some(c) =
                re(r"^(?:Main|Side) pot(?: #\d+)?: (\$[\d.]+) won by (.+?) \((\$[\d.]+)\)")
                    .captures(&line)
            {
                *gross.get_or_insert(0) += value(&c[1])?;
                b.action(&c[2], "collect", value(&c[3])?, false, false)?;
                continue;
            }
            if line.starts_with("Rake taken:") {
                rake = value(&line)?;
                continue;
            }
            if let Some(c) = re(r"^Seat \d+: (.+?) \(.*\[([^]]+)\]").captures(&line) {
                if c[1] == profile.hero {
                    b.lines.push(format!("Dealt to {} [{}]", &c[1], &c[2]));
                } else {
                    b.lines.push(format!("{}: shows [{}]", &c[1], &c[2]));
                }
            }
            continue;
        }
        if let Some(c) = re(r"^Seat (\d+): (.+?) \((\$[\d.]+)\)").captures(&line) {
            b.seat(c[1].parse()?, &c[2], value(&c[3])?)?;
            continue;
        }
        if line.starts_with("--- Dealing ") {
            let street = if line.contains("flop") {
                "FLOP"
            } else if line.contains("turn") {
                "TURN"
            } else if line.contains("river") {
                "RIVER"
            } else {
                bail!("unknown OnGame street")
            };
            b.board(format!("*** {street} *** {}", bracket_part(&line)));
            continue;
        }
        if line.starts_with("Dealt to ") {
            b.lines.push(line);
            continue;
        }
        for (name, _) in b.seats.values().cloned().collect::<Vec<_>>() {
            if let Some(text) = line.strip_prefix(&format!("{name} ")) {
                let kind = if text.starts_with("posts small blind") {
                    "posts small blind"
                } else if text.starts_with("posts big blind") {
                    "posts big blind"
                } else if text.starts_with("calls") {
                    "calls"
                } else if text.starts_with("bets") {
                    "bets"
                } else if text.starts_with("raises") {
                    "raise"
                } else if text.starts_with("folds") {
                    "fold"
                } else if text.starts_with("checks") {
                    "check"
                } else if text.starts_with("shows") || text.starts_with("mucks") {
                    b.lines.push(format!("{name}: {text}"));
                    break;
                } else {
                    bail!("unknown OnGame action")
                };
                let v = if matches!(kind, "fold" | "check") {
                    0
                } else if kind == "raise" {
                    value(
                        text.split_once(" to ")
                            .context("missing OnGame raise target")?
                            .1,
                    )?
                } else {
                    value(text)?
                };
                b.action(&name, kind, v, kind == "raise", text.contains("all-in"))?;
                break;
            }
        }
    }
    b.uncalled()?;
    b.lines.push("*** SHOWDOWN ***".into());
    b.summary = Some((
        gross.context("missing OnGame payout summary")?,
        format!("Rake ${}", money::format(rake)),
    ));
    b.finish(false, complete)
}

fn xml_cards(n: roxmltree::Node) -> Vec<String> {
    n.children()
        .filter(|c| c.has_tag_name("Card"))
        .map(|c| {
            format!(
                "{}{}",
                c.attribute("value").unwrap_or("?").replace("10", "T"),
                c.attribute("suit").unwrap_or("?").to_lowercase()
            )
        })
        .collect()
}
fn xml_actor(b: &Builder, n: roxmltree::Node, key: &str, zero_based: bool) -> Result<String> {
    let actor = n.attribute(key).context("missing XML actor")?;
    if let Ok(seat) = actor.parse::<u8>() {
        let seat = seat
            .checked_add(u8::from(zero_based))
            .context("invalid XML seat")?;
        Ok(b.seats.get(&seat).context("unknown XML seat")?.0.clone())
    } else {
        ensure!(
            b.seats.values().any(|(name, _)| name == actor),
            "unknown XML actor"
        );
        Ok(actor.into())
    }
}
fn merge(raw: &str, profile: &Profile) -> Result<Normalized> {
    // Merge exports use sibling description/game elements rather than one root.
    let wrapped = format!(
        "<history>{}</history>",
        re(r"<\?xml[^?]*\?>").replace(raw, "")
    );
    let doc = roxmltree::Document::parse(&wrapped).context("invalid Merge XML")?;
    let desc = doc
        .descendants()
        .find(|n| n.has_tag_name("description"))
        .context("missing Merge description")?;
    let stakes = desc.attribute("stakes").context("missing Merge stakes")?;
    ensure!(
        desc.attribute("type") == Some("Holdem") && stakes.starts_with("No Limit ("),
        "unsupported Merge game"
    );
    let limit = re(r"\(([^/]+)/([^\)]+)\)")
        .captures(stakes)
        .context("missing Merge blinds")?;
    let games = doc
        .descendants()
        .filter(|n| n.has_tag_name("game"))
        .collect::<Vec<_>>();
    ensure!(games.len() == 1, "expected one Merge game");
    let g = games[0];
    ensure!(
        g.attribute("realmoney") == Some("true")
            && !g.attributes().any(|a| a.name().contains("tournament")),
        "Merge cash game required"
    );
    let stamp = NaiveDateTime::parse_from_str(
        g.attribute("starttime")
            .context("missing Merge timestamp")?,
        "%Y%m%d%H%M%S",
    )?
    .format("%Y/%m/%d %H:%M:%S")
    .to_string();
    let mut b = Builder::new(
        g.attribute("id").context("missing Merge ID")?,
        stamp,
        value(&limit[1])?,
        value(&limit[2])?,
        currency(stakes)?,
        "merge",
        None,
    );
    b.max = g
        .attribute("seats")
        .context("missing Merge table size")?
        .parse()?;
    b.table = g
        .attribute("data")
        .unwrap_or("Merge")
        .split('|')
        .nth(1)
        .unwrap_or("Merge")
        .into();
    let players = g
        .children()
        .find(|n| n.has_tag_name("players"))
        .context("missing Merge players")?;
    b.button = players
        .attribute("dealer")
        .context("missing Merge button")?
        .parse::<u8>()?
        .checked_add(1)
        .context("invalid Merge button")?;
    for p in players.children().filter(|n| n.has_tag_name("player")) {
        if p.attribute("dealtin") == Some("false") {
            continue;
        }
        b.seat(
            p.attribute("seat")
                .context("missing Merge seat")?
                .parse::<u8>()?
                .checked_add(1)
                .context("invalid Merge seat")?,
            p.attribute("nickname").context("missing Merge player")?,
            xml_money(p.attribute("balance").context("missing Merge stack")?)?,
        )?;
    }
    let mut complete = false;
    let mut hidden_reveals = vec![];
    for round in g.children().filter(|n| n.has_tag_name("round")) {
        let rid = round.attribute("id").context("missing Merge street")?;
        let mut round_reveals = vec![];
        for c in round.children().filter(|n| n.has_tag_name("cards")) {
            let cs = c
                .attribute("cards")
                .context("missing Merge cards")?
                .replace(',', " ");
            match c.attribute("type") {
                Some("COMMUNITY") => {
                    let street = match rid {
                        "POSTFLOP" => "FLOP",
                        "POSTTURN" => "TURN",
                        "POSTRIVER" => "RIVER",
                        _ => bail!("unknown Merge board"),
                    };
                    b.board(format!("*** {street} *** [{cs}]"));
                }
                Some("SHOWN" | "HOLE") => {
                    let name = xml_actor(&b, c, "player", true)?;
                    if name == profile.hero {
                        b.lines.push(format!("Dealt to {name} [{cs}]"));
                    } else if c.attribute("type") == Some("SHOWN") {
                        round_reveals.push(format!("{name}: shows [{cs}]"));
                    } else {
                        hidden_reveals.push(format!("{name}: shows [{cs}]"));
                    }
                }
                _ => bail!("unknown Merge card type"),
            }
        }
        for e in round.children().filter(|n| n.has_tag_name("event")) {
            let ty = e.attribute("type").context("missing Merge action")?;
            if matches!(ty, "SHOW" | "MUCK") {
                b.lines.push("*** SHOWDOWN ***".into());
                continue;
            }
            let name = xml_actor(&b, e, "player", true)?;
            let v = e
                .attribute("amount")
                .map(xml_money)
                .transpose()?
                .unwrap_or(0);
            let kind = match ty {
                "SMALL_BLIND" => "posts small blind",
                "BIG_BLIND" => "posts big blind",
                "INITIAL_BLIND" => "posts dead blind",
                "RETURN_BLIND" => "return",
                "ANTE" => "posts ante",
                "FOLD" => "fold",
                "CHECK" => "check",
                "CALL" => "calls",
                "BET" => "bets",
                "RAISE" => "raise",
                "ALL_IN" => {
                    if b.live.get(&name).copied().unwrap_or(0) + v
                        > b.live.values().copied().max().unwrap_or(0)
                    {
                        "raise"
                    } else {
                        "calls"
                    }
                }
                _ => bail!("unknown Merge action {ty}"),
            };
            b.action(&name, kind, v, ty == "RAISE", ty == "ALL_IN")?;
        }
        b.lines.extend(round_reveals);
        if rid == "END_OF_GAME" {
            complete = true;
            b.lines.push("*** SHOWDOWN ***".into());
            for w in round.children().filter(|n| n.has_tag_name("winner")) {
                let name = xml_actor(&b, w, "player", true)?;
                let v = xml_money(w.attribute("amount").context("missing Merge win")?)?;
                b.action(
                    &name,
                    if w.attribute("uncalled") == Some("true") {
                        "return"
                    } else {
                        "collect"
                    },
                    v,
                    false,
                    false,
                )?;
            }
        }
    }
    b.lines.extend(hidden_reveals);
    b.finish(false, complete)
}
fn microgaming(raw: &str, profile: &Profile) -> Result<Normalized> {
    let doc = roxmltree::Document::parse(raw).context("invalid MPN XML")?;
    let g = doc.root_element();
    ensure!(
        g.has_tag_name("Game")
            && g.attribute("gametype") == Some("Hold'em")
            && g.attribute("betlimit") == Some("NL")
            && g.attribute("istournament") == Some("0")
            && g.attribute("realmoney") == Some("true"),
        "unsupported MPN game"
    );
    let curr = match g.attribute("currencysymbol") {
        Some("rCA=") => "EUR",
        Some("JAA=") => "USD",
        Some("owA=") => "GBP",
        _ => bail!("unsupported MPN currency"),
    };
    let (sb, bb) = g
        .attribute("stakes")
        .context("missing MPN stakes")?
        .split_once('|')
        .context("missing MPN blinds")?;
    let mut b = Builder::new(
        g.attribute("id").context("missing MPN ID")?,
        date(g.attribute("date").context("missing MPN timestamp")?)?,
        money::parse(sb)?,
        money::parse(bb)?,
        curr.into(),
        "microgaming",
        None,
    );
    b.table = g.attribute("tablename").unwrap_or("MPN").into();
    b.max = g
        .attribute("tablesize")
        .context("missing MPN table size")?
        .parse()?;
    let mut endbalances = vec![];
    let mut payouts: HashMap<String, i64> = HashMap::new();
    let mut complete = false;
    for p in g
        .descendants()
        .filter(|n| n.has_tag_name("Seat") && n.attribute("alias").is_some())
    {
        if p.attribute("sittingout") == Some("true") {
            continue;
        }
        let name = p.attribute("alias").unwrap();
        let stack = xml_money(p.attribute("balance").context("missing MPN balance")?)?;
        b.seat(
            p.attribute("num").context("missing MPN seat")?.parse()?,
            name,
            stack,
        )?;
        if p.attribute("dealer") == Some("true") {
            b.button = p.attribute("num").unwrap().parse()?;
        }
        endbalances.push((
            name.to_string(),
            stack,
            xml_money(
                p.attribute("endbalance")
                    .context("missing MPN endbalance")?,
            )?,
        ));
    }
    for a in g.descendants().filter(|n| n.has_tag_name("Action")) {
        let ty = a.attribute("type").context("missing MPN action")?;
        if matches!(ty, "DealFlop" | "DealTurn" | "DealRiver") {
            let street = match ty {
                "DealFlop" => "FLOP",
                "DealTurn" => "TURN",
                _ => "RIVER",
            };
            b.board(format!("*** {street} *** [{}]", xml_cards(a).join(" ")));
            continue;
        }
        if ty == "Win" {
            complete = true;
            b.lines.push("*** SHOWDOWN ***".into());
            for w in a.children().filter(|n| n.has_tag_name("Seat")) {
                let name = xml_actor(&b, w, "num", false)?;
                let v = xml_money(w.attribute("amount").context("missing MPN payout")?)?;
                *payouts.entry(name.clone()).or_default() += v;
                b.action(&name, "collect", v, false, false)?;
            }
            continue;
        }
        let name = xml_actor(&b, a, "seat", false)?;
        if matches!(ty, "ShowCards" | "DealCards") {
            let cs = xml_cards(a);
            if name == profile.hero {
                b.lines.push(format!("Dealt to {name} [{}]", cs.join(" ")));
            } else {
                b.lines.push(format!("{name}: shows [{}]", cs.join(" ")));
            }
            continue;
        }
        let v = a
            .attribute("value")
            .map(xml_money)
            .transpose()?
            .unwrap_or(0);
        let kind = match ty {
            "SmallBlind" => "posts small blind",
            "BigBlind" => "posts big blind",
            "Ante" => "posts ante",
            "Fold" => "fold",
            "Check" => "check",
            "Call" => "calls",
            "Bet" => "bets",
            "Raise" => "raise",
            "AllIn" => {
                if b.live.get(&name).copied().unwrap_or(0) + v
                    > b.live.values().copied().max().unwrap_or(0)
                {
                    "raise"
                } else {
                    "calls"
                }
            }
            "Return" => "return",
            _ => bail!("unknown MPN action {ty}"),
        };
        b.action(&name, kind, v, ty == "Raise", ty == "AllIn")?;
    }
    for (name, start, end) in endbalances {
        ensure!(
            start - b.invested.get(&name).copied().unwrap_or(0)
                + b.returned.get(&name).copied().unwrap_or(0)
                + payouts.get(&name).copied().unwrap_or(0)
                == end,
            "MPN end balance disagrees with action ledger"
        );
    }
    // MPN's rake attribute is a rate; do not treat it as a cash amount.
    b.finish(false, complete)
}

/// Stream iPoker sessions, Merge exports and MPN game XML into single hands.
/// Memory is bounded by metadata (64 KiB) and one game (1 MiB).
pub fn stream_xml(
    reader: impl std::io::BufRead,
    consume: &mut impl FnMut(&str) -> Result<bool>,
) -> Result<()> {
    use quick_xml::{
        events::{BytesEnd, BytesStart, Event},
        Reader, Writer,
    };
    let mut reader = Reader::from_reader(reader);
    let mut buf = vec![];
    let mut metadata = Writer::new(Vec::new());
    let mut description = Writer::new(Vec::new());
    let mut game: Option<Writer<Vec<u8>>> = None;
    let mut session = false;
    let mut describing = false;
    let mut count = 0;
    let mut metadata_done = false;
    let mut game_kind = "";
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .context("invalid hand-history XML stream")?;
        match &event {
            Event::DocType(_) => bail!("XML DTD/entities are unsupported"),
            Event::Start(e) if e.name().as_ref() == "session" => {
                ensure!(
                    !session && game.is_none(),
                    "nested XML sessions are unsupported"
                );
                session = true;
                metadata_done = false;
                metadata = Writer::new(Vec::new());
                metadata.write_event(event.clone())?;
            }
            Event::Start(e) if e.name().as_ref() == "description" => {
                ensure!(game.is_none(), "unexpected XML description");
                description = Writer::new(Vec::new());
                describing = true;
                description.write_event(event.clone())?;
            }
            Event::Empty(e) if e.name().as_ref() == "description" => {
                description = Writer::new(Vec::new());
                description.write_event(event.clone())?;
            }
            Event::End(e) if e.name().as_ref() == "description" => {
                description.write_event(event.clone())?;
                describing = false;
            }
            Event::Start(e) if matches!(e.name().as_ref(), "game" | "Game") => {
                ensure!(game.is_none(), "nested XML games are unsupported");
                let mut w = if session {
                    metadata_done = true;
                    game_kind = "ipoker";
                    Writer::new(metadata.get_ref().clone())
                } else if e.name().as_ref() == "game" {
                    ensure!(
                        !description.get_ref().is_empty(),
                        "Merge XML needs description"
                    );
                    game_kind = "merge";
                    let mut w = Writer::new(Vec::new());
                    w.write_event(Event::Start(BytesStart::new("history")))?;
                    w.get_mut().extend_from_slice(description.get_ref());
                    w
                } else {
                    game_kind = "microgaming";
                    Writer::new(Vec::new())
                };
                w.write_event(event.clone())?;
                game = Some(w);
            }
            Event::End(e) if matches!(e.name().as_ref(), "game" | "Game") => {
                let mut w = game.take().context("unexpected XML game end")?;
                w.write_event(event.clone())?;
                if game_kind == "ipoker" {
                    w.write_event(Event::End(BytesEnd::new("session")))?;
                }
                if game_kind == "merge" {
                    w.write_event(Event::End(BytesEnd::new("history")))?;
                }
                let raw =
                    String::from_utf8(w.into_inner()).context("hand history must use UTF-8")?;
                count += 1;
                if !consume(&raw)? {
                    return Ok(());
                }
            }
            Event::End(e) if e.name().as_ref() == "session" => {
                ensure!(game.is_none(), "incomplete XML game");
                session = false;
            }
            Event::Eof => {
                ensure!(
                    !session && !describing && game.is_none() && count > 0,
                    "incomplete/empty hand-history XML"
                );
                break;
            }
            _ => {
                if let Some(w) = &mut game {
                    w.write_event(event)?;
                } else if describing {
                    description.write_event(event)?;
                } else if session && !metadata_done {
                    metadata.write_event(event)?;
                }
            }
        }
        ensure!(
            metadata.get_ref().len() + description.get_ref().len() <= 65_536,
            "XML metadata exceeds 64 KiB"
        );
        ensure!(
            game.as_ref().is_none_or(|w| w.get_ref().len() <= 1_048_560),
            "single hand exceeds 1 MiB; source may be invalid"
        );
        buf.clear();
    }
    Ok(())
}
