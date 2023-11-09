use crate::credentials::Credentials;

use anyhow::Result;
use reqwest::Url;
use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TestReportSummary {
    pub total: TestSummaryDetail,
    pub test_suites: Vec<TestSuiteSummary>,
}

#[derive(Deserialize, Debug)]
pub struct TestSummaryDetail {
    pub time: f64,
    pub count: u32,
    pub success: u32,
    pub failed: u32,
    pub skipped: u32,
    pub error: u32,
    pub suite_error: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct TestSuiteSummary {
    pub name: String,
    pub total_time: f64,
    pub total_count: u32,
    pub success_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
    pub error_count: u32,
    pub build_ids: Vec<u64>,
    pub suite_error: Option<String>,
}

#[allow(dead_code)]
pub async fn get_test_report_summary(
    credentials: &Credentials,
    project_id: &str,
    pipeline_id: u32,
) -> Result<TestReportSummary, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/api/v4/projects/{}/pipelines/{}/test_report_summary",
        credentials.url, project_id, pipeline_id
    );
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .bearer_auth(&credentials.token)
        .send()
        .await?;

    // Always capture the response text
    let response_text = response.text().await?;

    // First, parse the raw response text as a serde_json::Value for potential pretty-printing
    let json_value: serde_json::Value = serde_json::from_str(&response_text)?;

    // Now attempt to deserialize the serde_json::Value into TestReportSummary
    serde_json::from_value(json_value.clone()).map_err(|e| {
        // If deserialization into TestReportSummary fails, pretty-print the original JSON
        println!("Failed to deserialize TestReportSummary: {}", e);
        println!(
            "Original response text as pretty-printed JSON: {}",
            serde_json::to_string_pretty(&json_value)
                .unwrap_or_else(|_| "Failed to pretty-print JSON".to_string())
        );
        e.into() // Convert serde_json::Error into a Box<dyn std::error::Error>
    })
}
