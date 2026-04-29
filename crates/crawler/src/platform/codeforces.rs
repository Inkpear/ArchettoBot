use async_trait::async_trait;
use serde::Deserialize;

use super::ContestFetcher;
use crate::error::{CrawlerError, Result};
use crate::models::Competition;

#[derive(Deserialize)]
struct CfResponse {
    status: String,
    result: Vec<CfContest>,
}

#[derive(Deserialize)]
struct CfContest {
    id: i64,
    name: String,
    #[serde(rename = "startTimeSeconds")]
    start_time: i64,
    #[serde(rename = "durationSeconds")]
    duration: i64,
    phase: Option<String>,
}

pub struct Codeforces;

#[async_trait]
impl ContestFetcher for Codeforces {
    fn platform_name(&self) -> &str {
        "Codeforces"
    }

    async fn fetch(&self) -> Result<Vec<Competition>> {
        let resp = reqwest::get("https://codeforces.com/api/contest.list")
            .await?
            .json::<CfResponse>()
            .await?;

        parse_codeforces_response(&resp)
    }
}

fn parse_codeforces_response(resp: &CfResponse) -> Result<Vec<Competition>> {
    if resp.status != "OK" {
        return Err(CrawlerError::Parse(
            "Codeforces API returned non-OK status".into(),
        ));
    }

    let competitions: Vec<Competition> = resp
        .result
        .iter()
        .filter(|c| c.phase.as_deref() == Some("BEFORE"))
        .map(|c| Competition {
            link: format!("https://codeforces.com/contest/{}", c.id),
            name: c.name.clone(),
            start_time: c.start_time,
            duration: c.duration as i32,
            platform: "Codeforces".to_owned(),
            notified: false,
        })
        .collect();

    Ok(competitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_upcoming_contest() {
        let resp: CfResponse = serde_json::from_value(json!({
            "status": "OK",
            "result": [{
                "id": 2000,
                "name": "Codeforces Round #2000",
                "startTimeSeconds": 2000000000,
                "durationSeconds": 7200,
                "phase": "BEFORE"
            }]
        }))
        .unwrap();
        let result = parse_codeforces_response(&resp).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Codeforces Round #2000");
        assert_eq!(result[0].duration, 7200);
        assert_eq!(result[0].platform, "Codeforces");
        assert!(result[0].link.contains("2000"));
    }

    #[test]
    fn filter_finished_contest() {
        let resp: CfResponse = serde_json::from_value(json!({
            "status": "OK",
            "result": [
                {"id": 1, "name": "Old", "startTimeSeconds": 1000000000, "durationSeconds": 7200, "phase": "FINISHED"},
                {"id": 2, "name": "Upcoming", "startTimeSeconds": 2000000000, "durationSeconds": 7200, "phase": "BEFORE"}
            ]
        }))
        .unwrap();
        let result = parse_codeforces_response(&resp).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Upcoming");
    }

    #[test]
    fn non_ok_status() {
        let resp: CfResponse = serde_json::from_value(json!({
            "status": "FAILED",
            "result": []
        }))
        .unwrap();
        assert!(parse_codeforces_response(&resp).is_err());
    }
}
