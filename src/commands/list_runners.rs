use crate::runner::{get_runner_detail, get_runners, Runner};
use colored::*;
use prettytable::{cell, format, row, Table};

use crate::credentials::Credentials;
use anyhow::Result;
use futures::future::try_join_all;

fn opt(s: Option<String>) -> String {
    match s {
        Some(s) => s,
        None => "-".to_string()
    }
}

pub async fn list_runners(creds: &Credentials) -> Result<()> {
    let runners: Vec<Runner> = get_runners(creds).await?;

    let runner_details: Vec<_> = runners
        .iter()
        .map(|r| get_runner_detail(&creds, &r))
        .collect();

    let mut table = Table::new();

    // Set the headers
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.add_row(row![
        "ID",
        "Version",
        "Description",
        "IP",
        "Tags",
        "Online",
        "Active",
        "Shared",
        "Type"
    ]);

    let runner_details = try_join_all(runner_details).await?;
    for d in runner_details {
        let online = match d.online {
            Some(true) => "true".green(),
            _ => "false".bright_red(),
        };
        table.add_row(row![
            cell![&d.id.to_string()],
            cell![&opt(d.version)],
            cell![&d.description],
            cell![&opt(d.ip_address)],
            cell![&d.tag_list.join(", ")],
            cell![&online],
            cell![&d.active.to_string()],
            cell![&d.is_shared.to_string()],
            cell![&d.runner_type],
        ]);
    }

    table.printstd();

    Ok(())
}
