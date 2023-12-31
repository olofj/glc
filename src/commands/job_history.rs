use colored::*;
use prettytable::{format, row, Cell, Row, Table};

use crate::credentials::Credentials;
use crate::format::{format_bytes, format_seconds};
use crate::job::find_jobs;
use crate::job::Job;
use crate::pipeline::get_pipelines;

pub async fn job_history(
    creds: &Credentials,
    project: &str,
    job_name: &str,
    max_age: isize,
    source: Option<String>,
    rref: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pipelines = match (source, rref) {
        (None, None) => Vec::new(),
        (s, r) => {
            let pipelines = get_pipelines(creds, project, max_age, s, r).await?;
            pipelines.into_iter().map(|p| p.id as usize).collect()
        }
    };
    let jobs: Vec<Job> = find_jobs(
        creds,
        project,
        pipelines,
        Some(vec![job_name]),
        Some(max_age),
        None,
    )
    .await?;

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
        "Elapsed",
        "Queued",
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
            Cell::new(&format_bytes(job.artifacts_size)),
            Cell::new(&job.rref),
            Cell::new(&job.pipeline.sha[0..14]),
            Cell::new(&job.pipeline.source),
            Cell::new(&format!("{}", job.created_at)),
            Cell::new(&runner_name),
            Cell::new(&format_seconds(job.duration.unwrap_or_default())),
            Cell::new(&format_seconds(job.queued_duration.unwrap_or_default())),
        ]));
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
