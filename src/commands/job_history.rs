use std::time::Duration;

use colored::*;
use prettytable::{format, row, Cell, Row, Table};

use crate::commands::credentials::Credentials;
use crate::commands::job::find_jobs;
use crate::commands::job::Job;
use crate::commands::pipeline::get_pipelines;
use crate::format::{format_bytes, format_seconds};

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
    let jobs: Vec<Job> =
        find_jobs(creds, project, pipelines, Some(job_name), Some(max_age)).await?;

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
        "Runner",
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
        let runner_name = if let Some(r) = job.runner {
            r.description
        } else {
            "unknown".to_string()
        };
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
            Cell::new(&runner_name),
            Cell::new(&format_seconds(job.duration.unwrap_or_default())),
        ]));
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
