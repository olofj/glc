use crate::credentials::Credentials;
use crate::pipeline::Pipeline;
use crate::runner::Runner;

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::Client;
use reqwest::Url;
use serde_derive::Deserialize;
use std::sync::Arc;

use futures::future::join_all;

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
    pub runner: Option<Runner>,
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

    let client = Client::new();
    let response = client
        .get(url)
        .bearer_auth(&credentials.token)
        .send()
        .await?;

    let response_text = response.text().await?;

    //println!("{:#?}", response_text);
    let job: Job = serde_json::from_str(&response_text)?;

    Ok(job)
}

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(ndt: &NaiveDateTime) -> isize {
    let now = Utc::now().naive_utc();

    (now - *ndt).num_seconds() as isize
}

pub async fn find_jobs(
    credentials: &Credentials,
    project: &str,
    pipelines: Vec<usize>,
    job_name: Option<&str>,
    max_age: Option<isize>,
    status: Option<String>,
) -> Result<Vec<Job>, Box<dyn std::error::Error>> {
    let max_age = max_age.unwrap_or(std::isize::MAX);
    let client = reqwest::Client::new();

    // Collect all the futures into a Vec
    let job_futures: Vec<_> = pipelines
        .iter()
        .map(|&pipeline_id| {
            let client = client.clone();
            let url = format!(
                "{}/api/v4/projects/{}/pipelines/{}/jobs?per_page=100",
                credentials.url, project, pipeline_id
            );
            async move {
                let response = client
                    .get(&url)
                    .bearer_auth(&credentials.token)
                    .send()
                    .await?;

                response
                    .json::<Vec<Job>>()
                    .await
                    .map_err(Into::<Box<dyn std::error::Error>>::into) // Convert the error type
            }
        })
        .collect();

    // Now, use join_all to execute them concurrently and await their results
    let jobs_results = join_all(job_futures).await;

    // Process the results
    let mut all_jobs = Vec::new();
    for jobs_res in jobs_results {
        let mut jobs_page = jobs_res?;
        // Filter jobs by job_name, status, and max_age
        jobs_page.retain(|job| {
            job_name.map_or(true, |name| job.name == name)
                && status.as_ref().map_or(true, |s| &job.status == s)
                && seconds_ago(&job.created_at.naive_utc()) <= max_age
        });
        all_jobs.extend(jobs_page);
    }

    println!("{} jobs found", all_jobs.len());

    Ok(all_jobs)
}
