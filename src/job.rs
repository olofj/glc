use crate::credentials::Credentials;
use crate::pipeline::{get_pipelines, Pipeline};
use crate::runner::Runner;

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::Client;
use reqwest::Url;
use serde_derive::Deserialize;
use std::io::Write;

use futures::future::try_join_all;
use tokio::sync::Semaphore;

use std::sync::atomic::{AtomicUsize, Ordering};
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
    job_names: Option<Vec<&str>>,
    max_age: isize,
) -> Result<Vec<Job>, anyhow::Error> {
    let client = Client::new();
    // Each return has a `x-total-pages` attribute coming back, so
    // fetch the first page to know how many more to get
    let first_page_url = format!("{}&page=1", base_url);

    //println!("Fetching {}", base_url);

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
        .map(|v| v.to_str().unwrap_or("1")) // Convert the header value to a string, default to "1" if any error occurs
        .unwrap_or("1") // Provide "1" as default if the header is not present
        .parse::<usize>()
        .unwrap_or(1);

    let oldest_valid_page = Arc::new(AtomicUsize::new(total_pages)); // Initialize with 1, assuming the first page is always valid
    let fetched_pages = Arc::new(AtomicUsize::new(1));

    let mut all_jobs_for_pipeline: Vec<Job> = response
        .json::<Vec<Job>>()
        .await?
        .into_iter()
        .filter(|j| {
            let job_age = seconds_ago(&j.created_at.naive_utc());
            if job_age > max_age {
                oldest_valid_page.store(1, Ordering::Relaxed);
            }
            job_names
                .as_ref()
                .map_or(true, |names| names.contains(&j.name.as_str()))
                && job_age <= max_age
        })
        .collect();

    drop(_permit);

    //println!("url {} total_pages {} jobs from first page {}", base_url, total_pages, all_jobs_for_pipeline.len());

    let page_futures: Vec<_> = (2..=total_pages)
        .map(|page| {
            let client = client.clone();
            let page_url = format!("{}&page={}", base_url, page);
            let credentials = credentials.clone();
            let sem = sem.clone();
            let oldest_valid_page = oldest_valid_page.clone();
            let fetched_pages = fetched_pages.clone();
            let job_names = job_names.clone();

            async move {
                let _permit = sem.acquire().await.unwrap();
                // Skip if the page is already known to be too old
                if page > oldest_valid_page.load(Ordering::Relaxed) {
                    //println!("Skipping page {}", page);
                    return Ok(Vec::new()); // Return an empty vector if the page is too old
                }
                let response = client
                    .get(&page_url)
                    .bearer_auth(&credentials.token)
                    .send()
                    .await;
                if response.is_err() {
                    return Ok(Vec::new()); // Return empty vector if the call failed
                }

                let text = response
                    .unwrap()
                    .text()
                    .await
                    .map_err(Into::<anyhow::Error>::into)
                    .unwrap();
                drop(_permit);
                fetched_pages.fetch_add(1, Ordering::SeqCst);

                let jobs = serde_json::from_str::<Vec<Job>>(&text);
                let ret: Result<Vec<Job>> = match jobs {
                    Ok(jobs) => {
                        let mut valid_jobs = Vec::new();
                        for job in jobs {
                            let job_age = seconds_ago(&job.created_at.naive_utc());
                            if job_age <= max_age {
                                if job_names
                                    .as_ref()
                                    .map_or(true, |names| names.contains(&job.name.as_str()))
                                {
                                    valid_jobs.push(job);
                                }
                            } else {
                                oldest_valid_page.store(page - 1, Ordering::Relaxed);
                                break; // No need to check further jobs on this page
                            }
                        }

                        for j in &mut valid_jobs {
                            j.artifacts_size = match &j.artifacts {
                                Some(a) => a.iter().map(|a| a.size).sum(),
                                _ => 0,
                            };
                        }

                        Ok(valid_jobs)
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

    //println!("Done with {} after {}/{} pages", base_url, fetched_pages.load(Ordering::Relaxed), total_pages);
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
    job_names: Option<Vec<&str>>,
    max_age: Option<isize>,
    status: Option<String>,
) -> Result<Vec<Job>, anyhow::Error> {
    let max_age = max_age.unwrap_or(std::isize::MAX);
    let semaphore = Arc::new(Semaphore::new(30));

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
                "{}/api/v4/projects/{}/pipelines/{}/jobs?per_page=20{}&include_retried=Yes",
                credentials.url, project, pipeline_id, scope_arg
            )
        })
        .collect();

    let mut job_futures = Vec::new();

    for (_index, base_url) in base_urls.iter().enumerate() {
        let job_names = job_names.clone();
        job_futures.push(multifetch(
            credentials.clone(),
            base_url,
            &semaphore,
            job_names,
            max_age,
        ));
    }
    let jobs_results = try_join_all(job_futures).await?;

    let mut ret = Vec::new();
    for mut jobs in jobs_results {
        jobs.retain(|job| {
            job_names
                .as_ref()
                .map_or(true, |names| names.contains(&job.name.as_str()))
                && seconds_ago(&job.created_at.naive_utc()) <= max_age
        });
        ret.extend(jobs);
    }
    print!("\r{:<3} pipelines {:<4} jobs", pipelines.len(), ret.len());
    std::io::stdout().flush().unwrap();
    println!("");

    Ok(ret)
}
