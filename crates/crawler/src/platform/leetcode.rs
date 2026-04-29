use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use super::ContestFetcher;
use crate::error::Result;
use crate::models::Competition;

#[derive(Deserialize)]
struct GraphQlResponse {
    data: GraphQlData,
}

#[derive(Deserialize)]
struct GraphQlData {
    #[serde(rename = "contestUpcomingContests")]
    contest_upcoming_contests: Vec<LcContest>,
}

#[derive(Deserialize)]
struct LcContest {
    title: String,
    #[serde(rename = "titleSlug")]
    title_slug: String,
    #[serde(rename = "startTime")]
    start_time: i64,
    duration: i64,
}

pub struct LeetCode;

#[async_trait]
impl ContestFetcher for LeetCode {
    fn platform_name(&self) -> &str {
        "LeetCode"
    }

    async fn fetch(&self) -> Result<Vec<Competition>> {
        let client = reqwest::Client::new();
        let query = r#"{
            contestUpcomingContests {
                title
                titleSlug
                startTime
                duration
            }
        }"#;

        let resp = client
            .post("https://leetcode.cn/graphql")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({"query": query}))
            .send()
            .await?;

        let body: GraphQlResponse = resp.json().await?;
        parse_leetcode_response(&body)
    }
}

fn parse_leetcode_response(body: &GraphQlResponse) -> Result<Vec<Competition>> {
    let now = Utc::now().timestamp();
    let competitions: Vec<Competition> = body
        .data
        .contest_upcoming_contests
        .iter()
        .filter(|c| c.start_time + c.duration > now)
        .map(|c| Competition {
            link: format!("https://leetcode.cn/contest/{}", c.title_slug),
            name: c.title.clone(),
            start_time: c.start_time,
            duration: c.duration as i32,
            platform: "LeetCode".to_owned(),
            notified: false,
        })
        .collect();
    Ok(competitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_response(contests: Vec<serde_json::Value>) -> GraphQlResponse {
        let json = json!({
            "data": {
                "contestUpcomingContests": contests
            }
        });
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn parse_upcoming_contest() {
        let body = make_response(vec![json!({
            "title": "Weekly Contest 500",
            "titleSlug": "weekly-contest-500",
            "startTime": 2000000000,
            "duration": 5400
        })]);
        let result = parse_leetcode_response(&body).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Weekly Contest 500");
        assert_eq!(result[0].platform, "LeetCode");
        assert!(!result[0].notified);
        assert!(result[0].link.contains("weekly-contest-500"));
    }

    #[test]
    fn filter_past_contest() {
        let body = make_response(vec![json!({
            "title": "Past Contest",
            "titleSlug": "past-contest",
            "startTime": 1000000000,
            "duration": 5400
        })]);
        let result = parse_leetcode_response(&body).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn multiple_contests() {
        let body = make_response(vec![
            json!({"title": "A", "titleSlug": "a", "startTime": 2000000000, "duration": 3600}),
            json!({"title": "B", "titleSlug": "b", "startTime": 2100000000, "duration": 7200}),
        ]);
        let result = parse_leetcode_response(&body).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "A");
        assert_eq!(result[1].name, "B");
    }
}
