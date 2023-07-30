use crate::commands::credentials::Credentials;
use crate::commands::pipeline::Pipeline;
use anyhow::Result;
use reqwest::Url;
use serde_derive::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct Artifact {
    pub file_type: String,
    pub size: usize,
    pub filename: String,
    pub file_format: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Job {
    pub id: usize,
    pub status: String,
    pub stage: String,
    pub name: String,
    #[serde(rename = "ref")]
    pub rref: String,
    pub tag: bool,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration: Option<f64>,
    pub queued_duration: Option<f64>,
    pub failure_reason: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub pipeline: Pipeline,
    // include other fields you are interested in
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
