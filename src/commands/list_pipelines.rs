use std::time::Duration;

use colored::*;
use prettytable::{format, row, Table};

use crate::commands::credentials::Credentials;
use crate::commands::pipeline::get_pipelines;

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
    table.set_titles(row!["ID", "Status", "Source", "Ref"]);

    // Add a row per time
    for pipeline in pipelines {
        let status = match pipeline.status.as_str() {
            "success" => "✅ Success".green(),
            "failed" => "❌ Failed".red(),
            "running" => "⏳ Running".yellow(),
            _ => "❓ Unknown".normal(),
        };
        table.add_row(row![
            &pipeline.id.to_string(),
            &status,
            &pipeline.source,
            &pipeline.rref,
        ]);
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
