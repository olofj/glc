use crate::credentials::Credentials;
use crate::pipeline::{get_pipelines, Pipeline};
use crate::runner::Runner;

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::Client;
use reqwest::Url;
use serde_derive::Deserialize;
use std::io::Write;
use std::sync::Arc;

use futures::future::try_join_all;
use std::time::Instant;
use tokio::sync::Semaphore;

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
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(skip)]
    pub artifacts_size: usize,
    pub pipeline: Pipeline,
    pub tag_list: Option<Vec<String>>,
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

async fn multifetch(
    credentials: Credentials,
    base_url: &str,
    sem: &Arc<Semaphore>,
    job_name: Option<&str>,
    max_age: isize,
) -> Result<Vec<Job>, anyhow::Error> {
    let client = Client::new();
    // Each return has a `x-total-pages` attribute coming back, so
    // fetch the first page to know how many more to get
    let first_page_url = format!("{}&page=1", base_url);

    let _permit = sem.acquire().await.unwrap();
    //println!("fetching {}", first_page_url);
    let response = client
        .clone()
        .get(&first_page_url)
        .bearer_auth(&credentials.token)
        .send()
        .await?;

    let total_pages = response
        .headers()
        .get("x-total-pages")
        .map_or("1", |_| &"1")
        .parse::<usize>()?;

    let mut all_jobs_for_pipeline: Vec<Job> = response
        .json::<Vec<Job>>()
        .await?
        .into_iter()
        .filter(|j| {
            job_name.map_or(true, |name| j.name == name)
                && seconds_ago(&j.created_at.naive_utc()) <= max_age
        })
        .collect();

    drop(_permit);

    //println!("url {} total_pages {}", base_url, total_pages);
    let total_pages: usize = 3;

    let page_futures: Vec<_> = (2..=total_pages)
        .map(|page| {
            let client = client.clone();
            let page_url = format!("{}&page={}", base_url, page);
            let credentials = credentials.clone();
            let sem = sem.clone();

            async move {
                let _permit = sem.acquire().await.unwrap();
                let response = client
                    .get(&page_url)
                    .bearer_auth(&credentials.token)
                    .send()
                    .await.unwrap();

                let text = response.text().await.map_err(Into::<anyhow::Error>::into).unwrap();
                drop(_permit);

                let jobs = serde_json::from_str::<Vec<Job>>(&text);
                let ret: Result<Vec<Job>> = match jobs {
                    Ok(jobs) => {
                        // If successful, proceed with the jobs after filtering
                        let mut jobs: Vec<Job> = jobs
                            .into_iter()
                            .filter(|j| {
                                job_name.map_or(true, |name| j.name == name)
                                    && seconds_ago(&j.created_at.naive_utc()) <= max_age
                            })
                            .collect();

                        for j in &mut jobs {
                            j.artifacts_size = match &j.artifacts {
                                Some(a) => a.iter().map(|a| a.size).sum(),
                                _ => 0,
                            };
                        }

                        Ok(jobs)
                    }
                    Err(e) => {
                        // If there's an error, print the error and the response body
                        println!(
                            "Error processing JSON: {}\nURL{}\nResponse was: {}",
                            e, page_url, text
                        );
                        Err(e.into())
                    }
                };
                //println!("done");
                ret
            }
        })
        .collect();

    let pages_results = try_join_all(page_futures).await?;
    for jobs in pages_results {
        all_jobs_for_pipeline.extend(jobs);
    }

    if all_jobs_for_pipeline.is_empty() {
        print!(".");
    } else {
        print!("*");
    }
    std::io::stdout().flush().unwrap();

    Ok::<Vec<Job>, anyhow::Error>(all_jobs_for_pipeline)
}

pub async fn get_runner_jobs(
    credentials: &Credentials,
    runner_id: usize,
    max_age: isize,
) -> Result<Vec<Job>> {
    let base_url = format!(
        "{}/api/v4/runners/{}/jobs?order_by=id&per_page=10",
        credentials.url, runner_id
    );
    let semaphore = Arc::new(Semaphore::new(10));

    Ok(multifetch(credentials.clone(), &base_url, &semaphore, None, max_age).await?)
}

pub async fn find_jobs(
    credentials: &Credentials,
    project: &str,
    pipelines: Vec<usize>,
    job_name: Option<&str>,
    max_age: Option<isize>,
    status: Option<String>,
) -> Result<Vec<Job>, anyhow::Error> {
    let max_age = max_age.unwrap_or(std::isize::MAX);
    let semaphore = Arc::new(Semaphore::new(30));

    let start_time = Instant::now();

    let pipelines = if pipelines.is_empty() {
        get_pipelines(credentials, project, max_age, None, None)
            .await?
            .into_iter()
            .map(|p| p.id as usize)
            .collect()
    } else {
        pipelines
    };

    let scope_arg = status
        .as_ref()
        .map_or(String::new(), |s| format!("&scope={}", s));

    let base_urls: Vec<String> = pipelines
        .iter()
        .map(|&pipeline_id| {
            format!(
                "{}/api/v4/projects/{}/pipelines/{}/jobs?per_page=100{}&include_retried=Yes",
                credentials.url, project, pipeline_id, scope_arg
            )
        })
        .collect();

    let mut job_futures = Vec::new();

    for base_url in base_urls.iter() {
        job_futures.push(multifetch(
            credentials.clone(),
            base_url,
            &semaphore,
            job_name,
            max_age,
        ));
    }
    print!("Fetching jobs from pipelines: ");
    let jobs_results = try_join_all(job_futures).await?;
    println!("");
    println!("Request completed in: {:?}", start_time.elapsed());

    let mut all_jobs = Vec::new();
    for mut jobs in jobs_results {
        jobs.retain(|job| {
            job_name.map_or(true, |name| job.name == name)
                && seconds_ago(&job.created_at.naive_utc()) <= max_age
        });
        all_jobs.extend(jobs);
    }

    println!("Completely completed in: {:?}", start_time.elapsed());
    Ok(all_jobs)
}
