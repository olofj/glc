use colored::*;
use prettytable::{format, row, Table};

use chrono::{DateTime, Utc};

use crate::credentials::Credentials;
use crate::format::format_seconds;
use crate::job::find_jobs;
use crate::pipeline::get_pipelines;

use futures::future::join_all;

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
    max_age: Option<isize>,
    source: Option<String>,
    rref: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_age = max_age.unwrap_or(86400);

    let pipelines = get_pipelines(creds, project, max_age, source, rref).await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP);
    table.set_titles(row![
        "ID",
        "Created",
        "🔄 Status",
        "Jobs",
        "Elapsed",
        "Source",
        "SHA",
        "Ref"
    ]);

    // First, collect all the futures into a Vec
    let jobs: Vec<_> = pipelines
        .iter()
        .map(|pipeline| find_jobs(creds, project, vec![pipeline.id as usize], None, None, None))
        .collect();

    // Now, use join_all to execute them concurrently and await their results
    let jobs = join_all(jobs).await;

    // Add a row per time
    for (pipeline, _jobs) in pipelines.iter().zip(jobs.into_iter()) {
        let status = match pipeline.status.as_str() {
            "success" => "✅ Success".green(),
            "failed" => "❌ Failed".red(),
            "running" => "⏳ Running".yellow(),
            _ => "❓ Unknown".normal(),
        };
        let mut elapsed = match (
            pipeline.status.as_str(),
            pipeline.created_at.clone(),
            pipeline.updated_at.clone(),
        ) {
            ("running", Some(c), _) => format_seconds(seconds_ago(c) as f64),
            (_, Some(c), Some(u)) => format_seconds((seconds_ago(c) - seconds_ago(u)) as f64),
            (_, _, _) => "-".to_string(),
        };
        if pipeline.status == "running" {
            elapsed.push_str("+");
        }
        let created = pipeline
            .created_at
            .clone()
            .map_or("-".to_string(), |created| created);
        table.add_row(row![
            &pipeline.id.to_string(),
            &created,
            &status,
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
