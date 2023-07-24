use colored::*;
use prettytable::{format, row, Table};
use reqwest::Url;

use crate::commands::credentials::Credentials;
use crate::commands::pipeline::Pipeline;

pub async fn list_projects(creds: &Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v4/projects", creds.url);
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client.get(url).bearer_auth(&creds.token).send().await?;

    let pipelines: Vec<Pipeline> = response.json().await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.add_row(row!["ID", "Status", "Source", "ref"]);

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
