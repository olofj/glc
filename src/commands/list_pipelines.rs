use colored::*;
use prettytable::{format, row, Table};

use chrono::{DateTime, Utc};

use crate::credentials::Credentials;
use crate::format::{format_bytes, format_seconds};
use crate::job::{find_jobs, Job};
use crate::pipeline::get_pipelines;

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(datetime: String) -> isize {
    let timestamp: chrono::DateTime<Utc> = DateTime::parse_from_rfc3339(&datetime)
        .expect("Failed to parse timestamp")
        .into();
    let now = Utc::now();

    (now - timestamp).num_seconds() as isize
}

pub async fn list_pipelines(
    creds: &Credentials,
    project: &str,
    max_age: isize,
    source: Option<String>,
    rref: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pipelines = get_pipelines(creds, project, max_age, source, rref).await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP);
    table.set_titles(row![
        "ID",
        "Created",
        "🔄 Status",
        "PASS / FAIL /  RUN / PEND",
        "Jobs",
        "Artifacts",
        "Elapsed",
        "Source",
        "SHA",
        "Ref"
    ]);

    let pids = pipelines.iter().map(|p| p.id as usize).collect();
    let all_jobs = find_jobs(creds, project, pids, None, None, None).await?;

    let jobs: Vec<Vec<&Job>> = pipelines
        .iter()
        .map(|p| all_jobs.iter().filter(|j| j.pipeline.id == p.id).collect())
        .collect();

    // Add a row per time
    for (pipeline, jobs) in pipelines.iter().zip(jobs.into_iter()) {
        let status = match pipeline.status.as_str() {
            "success" => "✅ Success".green(),
            "failed" => "❌ Failed".red(),
            "running" => "⏳ Running".yellow(),
            _ => "❓ Unknown".normal(),
        };
        let success = jobs.iter().filter(|j| j.status == "success").count();
        let failed = jobs.iter().filter(|j| j.status == "failed").count();
        let running = jobs.iter().filter(|j| j.status == "running").count();
        let elapsed = match (
            pipeline.status.as_str(),
            pipeline.created_at.clone(),
            pipeline.updated_at.clone(),
        ) {
            ("running", Some(c), _) => format_seconds(seconds_ago(c) as f64) + "+",
            (_, Some(c), Some(u)) => format_seconds((seconds_ago(c) - seconds_ago(u)) as f64),
            (_, _, _) => "-".to_string(),
        };
        let af_size: usize = jobs.iter().map(|j| j.artifacts_size).sum();
        let status_str = format!(
            "{:>4} / {:>4} / {:>4} / {:>4}",
            success,
            failed,
            running,
            jobs.len() - success - failed - running
        );

        let created = pipeline
            .created_at
            .clone()
            .map_or("-".to_string(), |created| created);
        table.add_row(row![
            &pipeline.id,
            &created,
            &status,
            &status_str,
            &jobs.len(),
            &format_bytes(af_size),
            &elapsed,
            &pipeline.source,
            &pipeline.sha[..14].to_string(),
            &pipeline.rref,
        ]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
