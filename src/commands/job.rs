use crate::commands::credentials::Credentials;
use crate::commands::pipeline::Pipeline;
use crate::commands::runner::Runner;

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use reqwest::header::LINK;
use reqwest::Url;
use serde_derive::Deserialize;
use std::sync::Arc;

use std::collections::HashMap;
use std::io::{self, Write};

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

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(ndt: &NaiveDateTime) -> isize {
    let now = Utc::now().naive_utc();

    (now - *ndt).num_seconds() as isize
}

pub async fn find_jobs(
    credentials: &Credentials,
    project: &str,
    pipelines: Option<Vec<usize>>,
    job_name: Option<&str>,
    max_age: Option<isize>,
    status: Option<String>,
) -> Result<Vec<Job>, Box<dyn std::error::Error>> {
    let max_age = max_age.unwrap_or(std::isize::MAX);
    let mut jobs = Vec::new();
    let mut urls: Vec<String> = match pipelines {
        Some(pipelines) => pipelines
            .into_iter()
            .map(|p| {
                format!(
                    "{}/api/v4/projects/{}/pipelines/{}/jobs?per_page=100",
                    credentials.url, project, p
                )
            })
            .collect(),
        None => vec![format!(
            "{}/api/v4/projects/{}/jobs?per_page=100",
            credentials.url, project
        )],
    };
    let client = reqwest::Client::new();
    let mut next_url: Option<String> = urls.pop();
    let mut stdout = io::stdout();

    print!("Scanning for jobs: ");
    while let Some(url) = next_url {
        let response = client
            .get(url)
            .bearer_auth(&credentials.token)
            .send()
            .await?;

        let link_header = response
            .headers()
            .get(LINK)
            .ok_or("Missing Link header")?
            .to_str()?;

        next_url = match parse_next_page(link_header) {
            None => urls.pop(),
            u => u,
        };

        let jobs_page: Vec<Job> = response.json::<Vec<Job>>().await?;
        let res_max_age = jobs_page
            .iter()
            .map(|j| seconds_ago(&j.created_at.naive_utc()))
            .max()
            .unwrap_or(0);
        let mut jobs_page: Vec<Job> = jobs_page
            .into_iter()
            .filter(|j| job_name.as_ref().map_or(true, |name| &j.name == name))
            .filter(|j| status.as_ref().map_or(true, |status| &j.status == status))
            .collect();
        let new = jobs_page.len();
        jobs.append(&mut jobs_page);

        if res_max_age > max_age {
            next_url = None;
        }
        if new > 0 {
            print!("*");
        } else {
            print!(".");
        }
        stdout.flush().unwrap();
    }
    println!(" {} found", jobs.len());

    Ok(jobs)
}

fn parse_next_page(link_header: &str) -> Option<String> {
    let links: HashMap<String, String> = link_header
        .split(',')
        .map(|line| {
            let re = Regex::new(r#"<([^>]*)>;\s*rel="([^"]*)""#).unwrap();

            re.captures(line)
                .map(|cap| {
                    let url = &cap[1];
                    let rel = &cap[2];
                    (rel.into(), url.into())
                })
                .unwrap()
        })
        .collect();
    //    println!("links: {:?}", links);
    links.get("next").cloned()
}
