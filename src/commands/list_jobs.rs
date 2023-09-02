use std::collections::HashMap;

use colored::*;
use prettytable::{format, row, Table};
use regex::Regex;
use reqwest::header::LINK;

use chrono::Utc;

use crate::commands::credentials::Credentials;
use crate::commands::job::Job;
use crate::format::{format_bytes, format_seconds};

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

        let response_text = response.text().await?;
        let mut jobs_page: Vec<Job> = serde_json::from_str(&response_text).map_err(|e| {
            format!(
                "Failed to parse JSON: {}\nOriginal JSON: {}",
                e, response_text
            )
        })?;
        //let mut jobs_page: Vec<Job> = response.json().await?;
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
        "Tags",
        "Execution time",
    ]);

    // Normalize jobs based on oldest created_at
    let min = jobs.iter().map(|job| job.created_at).min().unwrap();
    let max = jobs
        .iter()
        .map(|job| job.finished_at)
        .max()
        .unwrap_or(Utc::now());
    let scale = 30.0 / (max - min).num_seconds() as f64;

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
        let start_position = (job.started_at - min).num_seconds() as f64 * scale;
        let start_position = start_position as usize;
        let duration_width = (job.finished_at - job.started_at).num_seconds() as f64 * scale;
        let duration_width = duration_width.max(1.0) as usize;
        table.add_row(row![
            &job.id.to_string(),
            &status,
            &job.failure_reason.unwrap_or_default(),
            &job.stage,
            &format_bytes(artifact_size),
            &job.name,
            &format_seconds(job.duration.unwrap_or_default()).as_str(),
            &job.tag_list.join(" "),
            " ".repeat(start_position) + &"-".repeat(duration_width),
        ]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
