use chrono::{NaiveDateTime, TimeZone, Utc};

use crate::UTC8;

/// Scan `text` for datetime patterns (>= 16 chars like "YYYY-MM-DD HH:MM"),
/// interpret them as UTC+8 local time, and return the resulting UTC unix timestamps.
pub fn parse_utc8_timestamps(text: &str) -> Vec<i64> {
    let mut timestamps = Vec::new();
    let mut buf = String::new();

    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '-' || ch == ':' || ch == ' ' {
            buf.push(ch);
        } else {
            let s = buf.trim().to_owned();
            if s.len() >= 16 {
                timestamps.push(s);
            }
            buf.clear();
        }
    }
    let s = buf.trim().to_owned();
    if s.len() >= 16 {
        timestamps.push(s);
    }

    timestamps
        .iter()
        .filter_map(|ts| {
            NaiveDateTime::parse_from_str(&ts[..16], "%Y-%m-%d %H:%M")
                .ok()
                .and_then(|dt| UTC8.from_local_datetime(&dt).single())
                .map(|dt| dt.with_timezone(&Utc).timestamp())
        })
        .collect()
}
