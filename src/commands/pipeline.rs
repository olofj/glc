use std::collections::HashMap;
use std::io::{self, Write};

use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::header::LINK;
use reqwest::Url;
use serde_derive::Deserialize;

use crate::commands::credentials::Credentials;

#[derive(Deserialize, Clone, Debug)]
pub struct Pipeline {
    pub id: u32,
    pub project_id: u32,
    #[serde(rename = "ref")]
    pub rref: String,
    pub status: String,
    pub sha: String,
    pub source: String,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub finished_at: Option<String>,
    pub web_url: String,
}

// Returns number of seconds since the rfc3339 timestamp
fn seconds_ago(datetime: &str) -> isize {
    let timestamp: chrono::DateTime<Utc> = DateTime::parse_from_rfc3339(datetime)
        .expect("Failed to parse timestamp")
        .into();
    let now = Utc::now();

    (now - timestamp).num_seconds() as isize
}

pub async fn get_pipelines(
    creds: &Credentials,
    project: &str,
    max_age: isize,
    source: Option<String>,
    rref: Option<String>,
) -> Result<Vec<Pipeline>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/api/v4/projects/{}/pipelines?per_page=100",
        creds.url, project
    );
    let url = Url::parse(&url)?;
    let mut pipelines: Vec<Pipeline> = Vec::new();
    let client = reqwest::Client::new();
    let mut next_url: Option<String> = Some(url.to_string());
    let mut stdout = io::stdout();

    println!(
        "Searching for pipelines matching Ref: {} Source: {}",
        rref.as_ref().unwrap_or(&"any".to_string()),
        source.as_ref().unwrap_or(&"any".to_string())
    );
    print!("Pipelines: ");
    stdout.flush().unwrap();

    while let Some(url) = next_url {
        let response = client.get(url).bearer_auth(&creds.token).send().await?;
        let link_header = response
            .headers()
            .get(LINK)
            .ok_or("Missing Link header")?
            .to_str()?;

        print!(".");
        stdout.flush().unwrap();
        next_url = parse_next_page(link_header);

        let mut pipelines_page: Vec<Pipeline> = response.json().await?;
        let res_max_age = pipelines_page
            .iter()
            .map(|p| seconds_ago(p.created_at.as_ref().unwrap()))
            .max()
            .unwrap();
        pipelines_page.retain(|p| seconds_ago(p.created_at.as_ref().unwrap()) <= max_age);
        if let Some(src) = source.clone() {
            pipelines_page.retain(|p| p.source == src);
        }
        if let Some(rref) = rref.clone() {
            pipelines_page.retain(|p| p.rref == rref);
        }
        if res_max_age > max_age {
            next_url = None;
        }
        pipelines.append(&mut pipelines_page);
    }
    println!(" {} matched", pipelines.len());

    Ok(pipelines.into_iter().rev().collect())
}

fn parse_next_page(link_header: &str) -> Option<String> {
    let links: HashMap<String, String> = link_header
        .split(',')
        .map(|line| {
            let re = Regex::new(r#"<([^>]*)>;\s*rel="([^"]*)""#).unwrap();

            re.captures(line)
                .map(|cap| {
                    let url = &cap[1];
                    let rel = &cap[2];
                    (rel.into(), url.into())
                })
                .unwrap()
        })
        .collect();
    links.get("next").cloned()
}
