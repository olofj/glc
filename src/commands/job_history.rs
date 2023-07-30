use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Duration;

use chrono::{DateTime, Utc};
use colored::*;
use prettytable::{format, row, Table};
use regex::Regex;
use reqwest::header::LINK;

use crate::commands::credentials::Credentials;
use crate::commands::job::Job;
use crate::commands::pipeline::get_pipelines;

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(datetime: &String) -> Duration {
    let timestamp: chrono::DateTime<Utc> = DateTime::parse_from_rfc3339(datetime)
        .expect("Failed to parse timestamp")
        .into();
    let now = Utc::now();

    (now - timestamp).to_std().unwrap()
}

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

async fn find_jobs(
    credentials: &Credentials,
    project: &str,
    pipelines: Option<Vec<usize>>,
    job_name: &str,
    max_age: Duration,
) -> Result<Vec<Job>, Box<dyn std::error::Error>> {
    let max_age = if pipelines != None {
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
            .map(|j| match &j.created_at {
                None => Duration::new(0, 0),
                Some(c) => seconds_ago(&c),
            })
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
        "Status",
        "Reason",
        "Artifacts",
        "Ref",
        //        "Source",
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
        table.add_row(row![
            &job.id.to_string(),
            &status,
            &job.failure_reason.unwrap_or_default(),
            &format_bytes(artifact_size),
            &job.rref,
            //        &job.pipeline.source;
            &job.created_at.unwrap_or_default(),
            &format_seconds(job.duration.unwrap_or_default()).as_str(),
        ]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
