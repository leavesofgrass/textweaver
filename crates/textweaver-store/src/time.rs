//! UTC timestamps without a date-time dependency.
//!
//! textweaver stores Unix seconds (UTC) in its own files and RFC 3339 UTC
//! strings (`2026-09-25T14:03:07Z`) where a timestamp must compare as text,
//! as in the sidecar, whose merge rules compare `ts` lexicographically like
//! star did. star wrote zone-less local time (`2026-09-25T10:03:07`), which
//! compares wrongly across time zones (the star parity reference Part 3 §7 item 19).

/// Days since 1970-01-01 to a proleptic Gregorian `(year, month, day)`.
/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    // `d` is 1..=31 and `m` is 1..=12 by construction.
    (y, m as u32, d as u32)
}

/// `(year, month, day)` to days since 1970-01-01. Hinnant's `days_from_civil`.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(m);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Broken-down UTC time for Unix seconds.
fn parts(ts: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    // `secs` is 0..86400, so these fit in u32.
    let h = (secs / 3600) as u32;
    let mi = ((secs % 3600) / 60) as u32;
    let s = (secs % 60) as u32;
    (y, mo, d, h, mi, s)
}

/// Unix seconds as RFC 3339 UTC, `2026-09-25T14:03:07Z`.
pub fn rfc3339(ts: i64) -> String {
    let (y, mo, d, h, mi, s) = parts(ts);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Unix seconds as a compact UTC stamp for file names, `20260925-140307`.
pub fn file_stamp(ts: i64) -> String {
    let (y, mo, d, h, mi, s) = parts(ts);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

/// Unix seconds as a date for people, `2026-09-25 14:03 UTC`.
pub fn human(ts: i64) -> String {
    let (y, mo, d, h, mi, _) = parts(ts);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
}

/// Parses `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM:SS`, optionally followed by
/// fractional seconds and `Z` or a `±HH:MM` offset. A string without a zone
/// (star's format) is read as UTC. Returns Unix seconds.
pub fn parse_timestamp(s: &str) -> Option<i64> {
    let s = s.trim();
    let num = |a: usize, b: usize| -> Option<i64> {
        let part = s.get(a..b)?;
        if part.bytes().all(|c| c.is_ascii_digit()) {
            part.parse().ok()
        } else {
            None
        }
    };
    let y = num(0, 4)?;
    if s.get(4..5)? != "-" || s.get(7..8)? != "-" {
        return None;
    }
    let mo = u32::try_from(num(5, 7)?).ok()?;
    let d = u32::try_from(num(8, 10)?).ok()?;
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let mut ts = days_from_civil(y, mo, d) * 86_400;
    if s.len() == 10 {
        return Some(ts);
    }
    if !matches!(s.get(10..11)?, "T" | "t" | " ") || s.get(13..14)? != ":" {
        return None;
    }
    let h = num(11, 13)?;
    let mi = num(14, 16)?;
    let mut rest = 16;
    let mut sec = 0;
    if s.get(16..17) == Some(":") {
        sec = num(17, 19)?;
        rest = 19;
    }
    if h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    ts += h * 3600 + mi * 60 + sec;
    let mut tail = &s[rest..];
    if let Some(frac) = tail.strip_prefix('.') {
        let digits = frac.bytes().take_while(u8::is_ascii_digit).count();
        tail = &frac[digits..];
    }
    match tail {
        "" | "Z" | "z" => Some(ts),
        _ => {
            let sign = match tail.get(..1)? {
                "+" => 1,
                "-" => -1,
                _ => return None,
            };
            let oh: i64 = tail.get(1..3)?.parse().ok()?;
            let om: i64 = match tail.get(3..4) {
                Some(":") => tail.get(4..6)?.parse().ok()?,
                _ => tail.get(3..5)?.parse().ok()?,
            };
            Some(ts - sign * (oh * 3600 + om * 60))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        // Friday, September 25, 2026, 14:03:07 UTC.
        let ts = 1_790_344_987;
        assert_eq!(rfc3339(ts), "2026-09-25T14:03:07Z");
        assert_eq!(file_stamp(ts), "20260925-140307");
        assert_eq!(human(ts), "2026-09-25 14:03 UTC");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn parse_round_trip_and_forms() {
        for ts in [0, 951_782_400, 1_790_344_987, 4_102_444_800] {
            assert_eq!(parse_timestamp(&rfc3339(ts)), Some(ts));
        }
        assert_eq!(parse_timestamp("2026-09-25"), Some(1_790_294_400));
        assert_eq!(
            parse_timestamp("2026-09-25T14:03:07"),
            Some(1_790_344_987),
            "star's zone-less form reads as UTC"
        );
        assert_eq!(
            parse_timestamp("2026-09-25T16:03:07.123+02:00"),
            Some(1_790_344_987)
        );
        assert_eq!(parse_timestamp("t"), None);
        assert_eq!(parse_timestamp("2026-13-01"), None);
        assert_eq!(parse_timestamp("legacy"), None);
    }

    #[test]
    fn rfc3339_sorts_like_time() {
        let a = rfc3339(1_000_000_000);
        let b = rfc3339(1_790_344_987);
        assert!(a < b);
    }
}
