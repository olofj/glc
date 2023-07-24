use colored::*;
use prettytable::{format, Cell, Row, Table};
use reqwest::Url;

use crate::commands::credentials::Credentials;
use crate::commands::pipeline::Pipeline;

pub async fn list_pipelines(
    creds: &Credentials,
    project: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v4/projects/{}/pipelines", creds.url, project);
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client.get(url).bearer_auth(&creds.token).send().await?;

    let pipelines: Vec<Pipeline> = response.json().await?;

    // Create a new table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    // Add a row per time
    for pipeline in pipelines {
        let status = match pipeline.status.as_str() {
            "success" => "✅ Success".green(),
            "failed" => "❌ Failed".red(),
            "running" => "⏳ Running".yellow(),
            _ => "❓ Unknown".normal(),
        };
        table.add_row(Row::new(vec![
            Cell::new(&pipeline.id.to_string()),
            Cell::new(&status),
            Cell::new(&pipeline.source),
            Cell::new(&pipeline.rref),
        ]));
    }

    // Print the table to stdout
    table.printstd();

    Ok(())
}
