use crate::commands::credentials::Credentials;
use crate::commands::pipeline::Pipeline;
use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Url;
use serde_derive::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Clone, Debug)]
pub struct Artifact {
    pub file_type: String,
    pub size: usize,
    pub filename: String,
    pub file_format: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Job {
    pub id: usize,
    pub status: String,
    pub stage: String,
    pub name: String,
    #[serde(rename = "ref")]
    pub rref: String,
    pub tag: bool,
    #[serde(deserialize_with = "parse_date")]
    pub created_at: DateTime<Utc>,
    #[serde(deserialize_with = "parse_date")]
    pub started_at: DateTime<Utc>,
    #[serde(deserialize_with = "parse_date")]
    pub finished_at: DateTime<Utc>,
    pub duration: Option<f64>,
    pub queued_duration: Option<f64>,
    pub failure_reason: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub pipeline: Pipeline,
    pub tag_list: Vec<String>,
    // include other fields you are interested in
}

fn parse_date<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(d)
        .map(|x: Option<_>| x.unwrap_or(Utc.timestamp_opt(0, 0).unwrap()))
}

pub async fn get_job_details(
    credentials: Arc<Credentials>,
    project: String,
    job_id: usize,
) -> Result<Job> {
    let url = format!(
        "{}/api/v4/projects/{}/jobs/{}",
        credentials.url, project, job_id
    );
    let _url_save = url.clone();
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .bearer_auth(&credentials.token)
        .send()
        .await?;

    let response_text = response.text().await?;

    let job: Job = serde_json::from_str(&response_text)?;

    Ok(job)
}
