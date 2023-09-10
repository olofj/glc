use std::sync::Arc;

use anyhow::Result;
use prettytable::{format, row, table, Table};
use reqwest::Url;
use tokio::task; // for task::spawn

use crate::commands::credentials::Credentials;
use crate::commands::job::get_job_details;
use crate::format::{format_bytes, format_seconds};

pub async fn show_job(
    credentials: &Credentials,
    project: &str,
    job_id: usize,
    status: bool,
    _follow: Option<bool>,
    tail: Option<isize>,
) -> Result<()> {
    let credentials = Arc::new(credentials.clone());
    let project = project.to_string();
    let job_details_future = task::spawn(get_job_details(
        Arc::clone(&credentials),
        project.clone(),
        job_id,
    ));
    let job_logs_future = task::spawn(get_job_logs(Arc::clone(&credentials), project, job_id));

    let job_details = job_details_future.await??;
    let job_logs = job_logs_future.await??;

    if let Some(tail) = tail {
        let tail = -tail as usize;
        let lines: Vec<&str> = job_logs.lines().rev().take(tail).collect();
        println!("\nJob Logs (last {} lines):", tail);
        println!("{}", lines.join("\n"));
    } else {
        println!("\nJob Logs:");
        println!("{}", job_logs);
    }

    // Now job_details and job_logs are available, you can print them or process further

    if status {
        let artifact_size = job_details.artifacts.iter().map(|a| a.size).sum();
        let mut artifact_table = job_details
            .artifacts
            .into_iter()
            .map(|a| row!(a.filename, format_bytes(a.size)))
            .collect::<Table>();
        artifact_table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        artifact_table.set_titles(row!("filename", "size"));

        let mut table = table!(
            ["ID", job_details.id],
            ["Status", job_details.status],
            ["Stage", job_details.stage],
            ["Name", job_details.name],
            ["Artifact size", format_bytes(artifact_size)],
            ["Artifacts", artifact_table],
            ["Started at", job_details.started_at],
            ["Finished at", job_details.finished_at],
            [
                "Duration",
                &format_seconds(job_details.duration.unwrap_or_default())
            ],
            ["Runner", job_details.runner.unwrap().description],
            ["", ""],
            ["Ref", job_details.pipeline.rref],
            ["Source", job_details.pipeline.source],
            ["Pipeline URL", job_details.pipeline.web_url]
        );

        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        table.printstd();
    }

    Ok(())
}

async fn get_job_logs(
    credentials: Arc<Credentials>,
    project: String,
    job_id: usize,
) -> Result<String> {
    let url = format!(
        "{}/api/v4/projects/{}/jobs/{}/trace",
        credentials.url, project, job_id
    );
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .bearer_auth(&credentials.token)
        .send()
        .await?;

    let logs = response.text().await?;

    Ok(logs)
}
