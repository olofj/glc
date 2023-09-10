use std::time::Duration;

use colored::*;
use prettytable::{format, row, Table};

use chrono::Utc;

use crate::commands::credentials::Credentials;
use crate::commands::job::find_jobs;
use crate::commands::job::Job;
use crate::format::{format_bytes, format_seconds};

pub async fn list_jobs(
    creds: &Credentials,
    project: &str,
    pipelines: Option<Vec<usize>>,
    max_age: Option<Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    let jobs: Vec<Job> = find_jobs(creds, project, pipelines, None, max_age).await?;

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
