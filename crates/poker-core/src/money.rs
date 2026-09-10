use anyhow::{bail, Context, Result};
use serde::{Deserialize, Deserializer, Serializer};

pub const SCALE: i64 = 1_000_000;

/// Exact six-decimal fixed point. Never parse settlement amounts through f64.
pub fn parse(input: &str) -> Result<i64> {
    let s = input.trim().trim_start_matches('$').replace(',', "");
    let (negative, s) = s
        .strip_prefix('-')
        .map_or((false, s.as_str()), |s| (true, s));
    let mut parts = s.split('.');
    let whole: i64 = parts.next().context("missing amount")?.parse()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some() || fraction.len() > 6 || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        bail!("unsupported money precision: {input}");
    }
    let fraction: i64 = format!("{fraction:0<6}").parse()?;
    let v = whole
        .checked_mul(SCALE)
        .and_then(|w| w.checked_add(fraction))
        .context("amount overflow")?;
    if v > 1_000_000_000 * SCALE {
        bail!("amount exceeds supported bound")
    }
    Ok(if negative { -v } else { v })
}

pub fn format(v: i64) -> String {
    let sign = if v < 0 { "-" } else { "" };
    let a = v.unsigned_abs();
    let mut fraction = format!("{:06}", a % SCALE as u64);
    while fraction.len() > 2 && fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{sign}{}.{fraction}", a / SCALE as u64)
}
pub fn serialize<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&format(*v))
}
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let s = String::deserialize(d)?;
    parse(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_decimal_roundtrip() {
        assert_eq!(
            parse("0.1").unwrap() + parse("0.2").unwrap(),
            parse("0.3").unwrap()
        );
        for s in ["0.00", "-13.33", "0.000001", "12000.23"] {
            assert_eq!(format(parse(s).unwrap()), s);
        }
        assert!(parse("0.0000001").is_err());
        assert!(parse("NaN").is_err());
    }
}
