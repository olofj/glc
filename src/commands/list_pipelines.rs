use std::time::Duration;

use colored::*;
use prettytable::{format, row, Table};

use chrono::{DateTime, Utc};

use crate::commands::credentials::Credentials;
use crate::commands::pipeline::get_pipelines;
use crate::format::format_seconds;

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(datetime: String) -> Duration {
    let timestamp: chrono::DateTime<Utc> = DateTime::parse_from_rfc3339(&datetime)
        .expect("Failed to parse timestamp")
        .into();
    let now = Utc::now();

    (now - timestamp).to_std().unwrap()
}

pub async fn list_pipelines(
    creds: &Credentials,
    project: &str,
    max_age: Option<Duration>,
    source: Option<String>,
    rref: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_age = max_age.unwrap_or(Duration::from_secs(86400));
    let pipelines = get_pipelines(creds, project, max_age, source, rref).await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP);
    table.set_titles(row!["ID", "Status", "Elapsed", "Source", "Ref"]);

    // Add a row per time
    for pipeline in pipelines {
        let status = match pipeline.status.as_str() {
            "success" => "✅ Success".green(),
            "failed" => "❌ Failed".red(),
            "running" => "⏳ Running".yellow(),
            _ => "❓ Unknown".normal(),
        };
        let elapsed = match (pipeline.created_at, pipeline.finished_at) {
            (Some(c), Some(f)) => {
                format_seconds((seconds_ago(f) - seconds_ago(c)).as_secs() as f64)
            }
            (_, _) => "-".to_string(),
        };
        table.add_row(row![
            &pipeline.id.to_string(),
            &status,
            &elapsed,
            &pipeline.source,
            &pipeline.rref,
        ]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
