use crate::commands::credentials::Credentials;
use anyhow::Result;
use reqwest::Url;
use std::sync::Arc;
use tokio::task; // for task::spawn

use crate::commands::job::Job;
use prettytable::{format, table};

fn format_bytes(bytes: usize) -> String {
    let bytes = bytes as f64;
    let kilobytes = bytes / 1024f64;
    let megabytes = kilobytes / 1024f64;
    let gigabytes = megabytes / 1024f64;
    let terabytes = gigabytes / 1024f64;

    if terabytes >= 1f64 {
        format!("{:.2} TB", terabytes)
    } else if gigabytes >= 1f64 {
        format!("{:.2} GB", gigabytes)
    } else if megabytes >= 1f64 {
        format!("{:.2} MB", megabytes)
    } else if kilobytes >= 1f64 {
        format!("{:.2} KB", kilobytes)
    } else {
        format!("{:.2} B", bytes)
    }
}

fn format_seconds(sec: f64) -> String {
    let sec = sec as usize;
    let minutes = sec / 60_usize;
    let hours = minutes / 60_usize;
    let days = hours / 24_usize;

    if days >= 1 {
        format!(
            "{:.0}d{:.0}h{:.0}m.{:.0}s",
            days,
            hours % 24,
            minutes % 60,
            sec % 60
        )
    } else if hours >= 1 {
        format!("{:.0}h{:.0}m.{:.0}s", hours, minutes % 60, sec % 60)
    } else if minutes >= 1 {
        format!("{:.0}m.{:.1}s", minutes, sec % 60)
    } else {
        format!("{:2}s", sec)
    }
}

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
        let artifact_size = job_details.artifacts.into_iter().map(|a| a.size).sum();

        let mut table = table!(
            ["ID", job_details.id],
            ["Status", job_details.status],
            ["Stage", job_details.stage],
            ["Name", job_details.name],
            ["Artifacts", format_bytes(artifact_size)],
            ["Started at", job_details.started_at.unwrap_or_default()],
            ["Finished at", job_details.finished_at.unwrap_or_default()],
            [
                "Duration",
                &format_seconds(job_details.duration.unwrap_or_default())
            ]
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

async fn get_job_details(
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
    //    println!("URL was: {:?}", _url_save);
    //    println!("Server response: {}", response_text);

    let job_details: Job = serde_json::from_str(&response_text)?;

    Ok(job_details)
}
