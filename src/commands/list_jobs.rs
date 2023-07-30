use std::collections::HashMap;

use colored::*;
use prettytable::{format, row, Table};
use regex::Regex;
use reqwest::header::LINK;

use crate::commands::credentials::Credentials;
use crate::commands::job::Job;

fn format_bytes(bytes: usize) -> ColoredString {
    let bytes = bytes as f64;
    let kilobytes = bytes / 1024f64;
    let megabytes = kilobytes / 1024f64;
    let gigabytes = megabytes / 1024f64;
    let terabytes = gigabytes / 1024f64;

    if terabytes >= 1f64 {
        format!("{:6.2} TB", terabytes).bright_red()
    } else if gigabytes >= 1f64 {
        format!("{:6.2} GB", gigabytes).bright_red()
    } else if megabytes >= 200f64 {
        format!("{:6.1} MB", megabytes).yellow()
    } else if megabytes >= 1f64 {
        format!("{:6.1} MB", megabytes).normal()
    } else if kilobytes >= 1f64 {
        format!("{:6.1} KB", kilobytes).normal()
    } else if bytes >= 1f64 {
        format!("{:6.1} B", bytes).normal()
    } else {
        format!("{:>6}", "-").normal()
    }
}

fn format_seconds(sec: f64) -> String {
    let sec = sec as usize;
    let minutes = sec / 60_usize;
    let hours = minutes / 60_usize;
    let days = hours / 24_usize;

    if days >= 1 {
        format!(
            "{:.0}d {:.0}h:{:.0}m.{:.0}s",
            days,
            hours % 24,
            minutes % 60,
            sec % 60
        )
    } else if hours >= 1 {
        format!("{:.0}h:{:.0}m.{:.0}s", hours, minutes % 60, sec % 60)
    } else if minutes >= 1 {
        format!("{:.0}m.{:.1}s", minutes, sec % 60)
    } else {
        format!("{:.2}s", sec)
    }
}

async fn get_jobs(
    credentials: &Credentials,
    project: &str,
    pipeline: Option<usize>,
) -> Result<Vec<Job>, Box<dyn std::error::Error>> {
    let mut jobs = Vec::new();
    let url = match pipeline {
        Some(pipeline) => format!(
            "{}/api/v4/projects/{}/pipelines/{}/jobs",
            credentials.url, project, pipeline
        ),
        None => format!("{}/api/v4/projects/{}/jobs", credentials.url, project),
    };

    let client = reqwest::Client::new();
    let mut next_url: Option<String> = Some(url);
    let mut count = 10;

    while let Some(url) = next_url {
        count -= 1;
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
        if count > 0 {
            next_url = parse_next_page(link_header);
        } else {
            next_url = None;
        }

        let mut jobs_page: Vec<Job> = response.json().await?;
        jobs.append(&mut jobs_page);
    }

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

pub async fn list_jobs(
    creds: &Credentials,
    project: &str,
    pipeline: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let jobs: Vec<Job> = get_jobs(creds, project, pipeline).await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row![
        "ID",
        "Status",
        "Reason",
        "Step",
        "Artifacts",
        "Name",
        "Time",
    ]);

    // Add a row per time
    for job in jobs.into_iter().rev() {
        let status = match job.status.as_str() {
            "success" => "✅\u{00a0} Success".green(),
            "failed" => "❌\u{00a0} Failed".red(),
            "running" => "⏳\u{00a0} Running".yellow(),
            "created" => "🌱\u{00a0} Created".normal(),
            stat => format!("❓\u{00a0} {stat}").normal(),
        };
        let artifact_size = job.artifacts.into_iter().map(|a| a.size).sum();
        table.add_row(row![
            &job.id.to_string(),
            &status,
            &job.failure_reason.unwrap_or_default(),
            &job.stage,
            &format_bytes(artifact_size),
            &job.name,
            &format_seconds(job.duration.unwrap_or_default()).as_str(),
        ]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
