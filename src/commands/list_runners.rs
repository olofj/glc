use prettytable::{format, row, Cell, Row, Table};
use reqwest::Url;
use serde_derive::Deserialize;

use crate::credentials::Credentials;

#[derive(Deserialize, Debug)]
struct RunnerShort {
    id: usize,
}

#[derive(Deserialize, Debug)]
struct RunnerDetail {
    id: usize,
    description: String,
    ip_address: String,
    active: bool,
    is_shared: bool,
    runner_type: String,
    version: String,
    tag_list: Vec<String>,
    // Add more fields as per your requirement
}

pub async fn list_runners(creds: &Credentials) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v4/runners/all?per_page=100", creds.url);
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client.get(url).bearer_auth(&creds.token).send().await?;

    let runners: Vec<usize> = response
        .json::<Vec<RunnerShort>>()
        .await?
        .into_iter()
        .map(|r| r.id)
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
        "Active",
        "Shared",
        "Type"
    ]);

    println!("{} runners", runners.len());

    let responses: Vec<_> = runners
        .into_iter()
        .map(|r| {
            let url = format!("{}/api/v4/runners/{}", creds.url, r);
            let url = Url::parse(&url).unwrap();
            let client = reqwest::Client::new();
            let request = client.get(url).bearer_auth(&creds.token).send();
            async move {
                let response = request.await;
                response
            }
        })
        .collect::<Vec<_>>();

    let futures = futures::future::join_all(responses);
    let results: Vec<_> = futures.await.into_iter().filter_map(Result::ok).collect();

    for r in results {
        let raw_json = r.text().await?;
        // Parse the raw JSON to a serde_json::Value to get all fields, even those not in RunnerDetail
        let _v: serde_json::Value = serde_json::from_str(&raw_json)?;
        // Print the pretty JSON
        //println!("raw runner: {}", serde_json::to_string_pretty(&_v)?);
        let d: RunnerDetail = serde_json::from_str(&raw_json)?;

        table.add_row(Row::new(vec![
            Cell::new(&d.id.to_string()),
            Cell::new(&d.version),
            Cell::new(&d.description),
            Cell::new(&d.ip_address),
            Cell::new(&d.tag_list.join(",")),
            Cell::new(&d.active.to_string()),
            Cell::new(&d.is_shared.to_string()),
            Cell::new(&d.runner_type),
        ]));
    }

    table.printstd();

    Ok(())
}
