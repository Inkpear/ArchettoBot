use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use scraper::{Html, Selector};

use super::ContestFetcher;
use crate::error::Result;
use crate::models::Competition;

pub struct AtCoder;

#[async_trait]
impl ContestFetcher for AtCoder {
    fn platform_name(&self) -> &str {
        "AtCoder"
    }

    async fn fetch(&self) -> Result<Vec<Competition>> {
        let html = reqwest::get("https://atcoder.jp/contests/")
            .await?
            .text()
            .await?;
        parse_atcoder_html(&html)
    }
}

fn parse_atcoder_html(html: &str) -> Result<Vec<Competition>> {
    let doc = Html::parse_document(html);
    let table_sel = Selector::parse("#contest-table-upcoming tbody tr").unwrap();
    let time_sel = Selector::parse("time").unwrap();
    let contest_link_sel = Selector::parse("a[href*=\"/contests/\"]").unwrap();

    let now = Utc::now().timestamp();
    let mut competitions = Vec::new();

    for row in doc.select(&table_sel) {
        let time_str = row
            .select(&time_sel)
            .next()
            .and_then(|t| t.text().next())
            .unwrap_or("");

        let link_elem = match row.select(&contest_link_sel).next() {
            Some(e) => e,
            None => continue,
        };
        let href = link_elem.value().attr("href").unwrap_or("");
        let name = link_elem.text().collect::<Vec<_>>().join("");
        let link = format!("https://atcoder.jp{}", href);

        let duration_text = row
            .text()
            .filter_map(|s| {
                let t = s.trim();
                if t.len() == 5 && t.chars().nth(2) == Some(':') {
                    Some(t.to_string())
                } else {
                    None
                }
            })
            .next()
            .unwrap_or_default();
        let duration = parse_duration(&duration_text);

        if let Ok(dt) = parse_atcoder_time(time_str) {
            let ts = dt.timestamp();
            if ts + duration as i64 > now {
                competitions.push(Competition {
                    link,
                    name,
                    start_time: ts,
                    duration,
                    platform: "AtCoder".to_owned(),
                    notified: false,
                });
            }
        }
    }

    Ok(competitions)
}

fn parse_atcoder_time(s: &str) -> std::result::Result<DateTime<Utc>, ()> {
    // AtCoder displays times in JST (UTC+9)
    let cleaned = s.trim();
    if let Ok(dt) =
        NaiveDateTime::parse_from_str(&cleaned[..cleaned.len().min(19)], "%Y-%m-%d %H:%M:%S")
    {
        let offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        // Interpret the naive datetime as JST local time
        let jst_dt: DateTime<chrono::FixedOffset> =
            offset.from_local_datetime(&dt).single().ok_or(())?;
        Ok(jst_dt.with_timezone(&Utc))
    } else {
        Err(())
    }
}

fn parse_duration(s: &str) -> i32 {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (
            parts[0].trim().parse::<i32>(),
            parts[1].trim().parse::<i32>(),
        ) {
            return h * 3600 + m * 60;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_hh_mm() {
        assert_eq!(parse_duration("01:30"), 5400);
        assert_eq!(parse_duration("02:00"), 7200);
        assert_eq!(parse_duration("00:05"), 300);
    }

    #[test]
    fn parse_duration_invalid() {
        assert_eq!(parse_duration("abc"), 0);
        assert_eq!(parse_duration(""), 0);
    }

    #[test]
    fn parse_atcoder_time_jst() {
        let dt = parse_atcoder_time("2025-06-15 21:00:00+0900").unwrap();
        // 21:00 JST = 12:00 UTC
        assert_eq!(dt.timestamp() % 86400, 12 * 3600);
    }

    #[test]
    fn parse_html_finds_contests() {
        let html = r#"
        <div id="contest-table-upcoming">
        <table><tbody>
        <tr>
            <td class="text-center"><a href="http://www.timeanddate.com/worldclock/fixedtime.html?iso=20300615T2100&p1=248" target="blank"><time class="fixtime fixtime-full">2030-06-15 21:00:00+0900</time></a></td>
            <td><span>Ⓐ</span> <a href="/contests/abc400">ABC400</a></td>
            <td class="text-center">01:40</td>
            <td class="text-center"> - 1999</td>
        </tr>
        <tr>
            <td class="text-center"><a href="http://www.timeanddate.com/worldclock/fixedtime.html?iso=20300622T2100&p1=248" target="blank"><time class="fixtime fixtime-full">2030-06-22 21:00:00+0900</time></a></td>
            <td><span>Ⓐ</span> <a href="/contests/arc200">ARC200</a></td>
            <td class="text-center">02:00</td>
            <td class="text-center">1200 - </td>
        </tr>
        </tbody></table>
        </div>
        "#;
        let result = parse_atcoder_html(html).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "ABC400");
        assert_eq!(result[0].platform, "AtCoder");
        assert_eq!(result[0].duration, 6000); // 1h40m
        assert!(result[0].link.contains("abc400"));
        assert_eq!(result[1].name, "ARC200");
        assert_eq!(result[1].duration, 7200);
    }
}
