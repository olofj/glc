use reqwest::Url;
use std::sync::Arc;

use futures::future::join_all;

use anyhow::Result;

use crate::credentials::Credentials;
use crate::job::{find_jobs, get_job_details, Job};

pub async fn cancel_job(
    creds: &Credentials,
    project: &str,
    jobs: Option<Vec<usize>>,
    pipeline: Option<usize>,
    job_names: Option<Vec<String>>,
) -> Result<(), anyhow::Error> {
    let job_names: Option<Vec<String>> = job_names.clone();
    let job_names: Option<Vec<&str>> = job_names
        .as_ref()
        .map(|vec| vec.iter().map(AsRef::as_ref).collect());
    let jobs: Vec<Job> = if let Some(pipeline) = pipeline {
        find_jobs(creds, project, vec![pipeline], job_names, None, None).await?
    } else {
        let futures = jobs
            .unwrap()
            .into_iter()
            .map(|j| get_job_details(Arc::new(creds.clone()), project.to_string(), j));

        let results = join_all(futures).await;
        results.into_iter().collect::<anyhow::Result<Vec<Job>>>()?
    };
    let jobs: Vec<usize> = jobs.into_iter().map(|j| j.id).collect();

    println!("Cancelling {} jobs...", jobs.len());

    for job in jobs {
        let url = format!(
            "{}/api/v4/projects/{}/jobs/{}/cancel",
            creds.url, project, job
        );
        let url = Url::parse(&url)?;

        let client = reqwest::Client::new();
        let response = client.post(url).bearer_auth(&creds.token).send().await?;

        let ret = response.text().await?;
        println!("Job {} ret: {}", job, ret);
    }

    Ok(())
}
