use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Duration;

use chrono::NaiveDateTime;
use chrono::Utc;
use colored::*;
use prettytable::{format, row, Cell, Row, Table};
use regex::Regex;
use reqwest::header::LINK;

use crate::commands::credentials::Credentials;
use crate::commands::job::Job;
use crate::commands::pipeline::get_pipelines;
use crate::format::{format_bytes, format_seconds};

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(ndt: &NaiveDateTime) -> Duration {
    let now = Utc::now().naive_utc();

    (now - *ndt).to_std().unwrap()
}

async fn find_jobs(
    credentials: &Credentials,
    project: &str,
    pipelines: Option<Vec<usize>>,
    job_name: &str,
    max_age: Duration,
) -> Result<Vec<Job>, Box<dyn std::error::Error>> {
    let max_age = if pipelines.is_some() {
        Duration::from_secs(std::u64::MAX)
    } else {
        max_age
    };
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
            .unwrap_or(Duration::new(0, 0));
        let mut jobs_page = jobs_page
            .into_iter()
            .filter(|j| j.name == job_name)
            .collect::<Vec<Job>>();
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

pub async fn job_history(
    creds: &Credentials,
    project: &str,
    job_name: &str,
    max_age: Option<Duration>,
    source: Option<String>,
    rref: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_age = max_age.unwrap_or(Duration::from_secs(86400));
    let pipelines = match (source, rref) {
        (None, None) => None,
        (s, r) => {
            let pipelines = get_pipelines(creds, project, max_age, s, r).await?;
            Some(pipelines.into_iter().map(|p| p.id as usize).collect())
        }
    };
    let jobs: Vec<Job> = find_jobs(creds, project, pipelines, job_name, max_age).await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row![
        "ID",
        "Pipeline",
        "Status",
        "Reason",
        "Artifacts",
        "Ref",
        "SHA",
        "Source",
        "Created",
        "Duration",
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
        table.add_row(Row::new(vec![
            Cell::new(&job.id.to_string()),
            Cell::new(&job.pipeline.id.to_string()),
            Cell::new(&status),
            Cell::new(&job.failure_reason.unwrap_or_default()),
            Cell::new(&format_bytes(artifact_size)),
            Cell::new(&job.rref),
            Cell::new(&job.pipeline.sha[0..14]),
            Cell::new(&job.pipeline.source),
            Cell::new(&format!("{}", job.created_at)),
            Cell::new(&format_seconds(job.duration.unwrap_or_default())),
        ]));
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
