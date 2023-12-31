use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use colored::*;
use prettytable::{format, row, Cell, Row, Table};

use crate::credentials::Credentials;
use crate::format::{format_bytes, format_seconds};
use crate::job::find_jobs;
use crate::job::Job;

fn compare_dates_with_tolerance(a: &DateTime<Utc>, b: &DateTime<Utc>, tolerance: i64) -> Ordering {
    let difference = a.signed_duration_since(*b).num_seconds().abs();

    if difference <= tolerance {
        Ordering::Equal
    } else {
        a.cmp(b)
    }
}

pub async fn list_jobs(
    creds: &Credentials,
    project: &str,
    pipelines: Vec<usize>,
    max_age: isize,
    status: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let list_pipeline = !pipelines.is_empty();
    // With specific pipelines, don't use max_age
    let max_age = if pipelines.is_empty() {
        Some(max_age)
    } else {
        None
    };
    let jobs: Vec<Job> = find_jobs(creds, project, pipelines, None, max_age, status).await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    let mut titles = vec![
        "ID",
        "Pipeline",
        "Ref",
        "Status",
        "Reason",
        "Step",
        "Artifacts",
        "Name",
        "Tags",
        "Runner",
        "Elapsed",
        "Queued",
    ];
    if list_pipeline {
        titles.push("Hist");
    }

    table.set_titles(titles.into_iter().map(|t| Cell::new(t)).collect());

    if jobs.is_empty() {
        table.add_row(Row::new(vec![Cell::new("No jobs found").with_hspan(12)]));
        table.printstd();
        return Ok(());
    }
    // Normalize jobs based on oldest created_at
    let min = jobs.iter().map(|job| job.created_at).min().unwrap();
    let max = jobs
        .iter()
        .map(|job| job.finished_at)
        .max()
        .unwrap_or(Utc::now());
    let scale = 30.0 / (max - min).num_seconds() as f64;

    let mut jobs = jobs;

    // Only sort for the histogram if we're listing for a pipeline
    if list_pipeline {
        jobs.sort_by(|a, b| {
            compare_dates_with_tolerance(&a.started_at, &b.started_at, 30)
                .then_with(|| compare_dates_with_tolerance(&a.finished_at, &b.finished_at, 30))
        });
    } else {
        jobs = jobs.into_iter().rev().collect();
    }

    let total_artifacts: usize = jobs.iter().map(|j| j.artifacts_size).sum();

    let nr_jobs = jobs.len();

    // Add a row per time
    for job in jobs.into_iter() {
        let status = match job.status.as_str() {
            "success" => "✅\u{00a0} Success".green(),
            "failed" => "❌\u{00a0} Failed".red(),
            "running" => "⏳\u{00a0} Running".yellow(),
            "created" => "🌱\u{00a0} Created".normal(),
            stat => format!("❓\u{00a0} {stat}").normal(),
        };
        let duration = if job.status == "running" {
            Utc::now() - job.started_at
        } else {
            job.finished_at - job.started_at
        };
        let start_position = (job.started_at - min).num_seconds() as f64 * scale;
        let duration_width = duration.num_seconds() as f64 * scale;
        let duration_width = duration_width.max(1.0).min(30.0);
        let start_position = start_position as usize;
        let duration_width = duration_width as usize;
        let runner = if let Some(runner) = job.runner {
            runner.description
        } else {
            "<unknown>".to_string()
        };
        //println!("job {:?} status {:?} started {:?} duration {:?}", job.id, job.status, start_position, duration_width);
        let mut row = row![
            &job.id.to_string(),
            &job.pipeline.id,
            &job.pipeline.rref,
            &status.to_string(),
            &job.failure_reason.unwrap_or_default(),
            &job.stage,
            &format_bytes(job.artifacts_size),
            &job.name,
            &job.tag_list.unwrap_or_default().join(" "),
            &runner,
            &format_seconds(job.duration.unwrap_or_default()).to_string(),
            &format_seconds(job.queued_duration.unwrap_or_default()).to_string(),
        ];
        if list_pipeline {
            row.add_cell(Cell::new(
                &(" ".repeat(start_position) + &"-".repeat(duration_width)),
            ));
        }
        table.add_row(row);
    }

    // Print the table to stdout
    table.printstd();

    println!("Jobs: {}", nr_jobs);
    println!(
        "Total artifacts produced: {}",
        format_bytes(total_artifacts)
    );

    Ok(())
}
