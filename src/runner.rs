use anyhow::Result;
use reqwest::Url;
use serde_derive::Deserialize;

use crate::credentials::Credentials;
use crate::project::Project;

#[derive(Deserialize, Clone, Debug)]
pub struct Runner {
    pub id: usize,
    pub description: String,
    pub ip_address: Option<String>,
    pub active: bool,
    pub paused: bool,
    pub is_shared: bool,
    pub runner_type: String,
    pub name: Option<String>,
    pub online: Option<bool>,
    pub status: String,
}

#[derive(Deserialize, Debug)]
pub struct RunnerDetail {
    pub id: usize,
    pub description: String,
    pub ip_address: Option<String>,
    pub active: bool,
    pub online: Option<bool>,
    pub is_shared: bool,
    pub runner_type: String,
    pub version: Option<String>,
    pub revision: Option<String>,
    pub tag_list: Vec<String>,
    pub projects: Option<Vec<Project>>,
    // Add more?
}

pub async fn get_runners(creds: &Credentials) -> Result<Vec<Runner>> {
    let url = format!("{}/api/v4/runners/all?per_page=100", creds.url);
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client.get(url).bearer_auth(&creds.token).send().await?;

    let raw_json = response.text().await?;
    // Parse the raw JSON to a serde_json::Value to get all fields, even those not in RunnerDetail
    let _v: serde_json::Value = serde_json::from_str(&raw_json)?;
    let runners = serde_json::from_str::<Vec<Runner>>(&raw_json);

    match runners {
        Err(e) => {
            println!("Failed parsing");
            println!("output:\n{:#?}", _v);
            println!("raw output:\n{}", raw_json);
            Err(e.into())
        }
        Ok(r) => Ok(r),
    }
}

pub async fn get_runner_detail(creds: &Credentials, r: &Runner) -> Result<RunnerDetail> {
    let url = format!("{}/api/v4/runners/{}", creds.url, r.id);
    let url = Url::parse(&url).unwrap();

    let client = reqwest::Client::new();
    let request = client.get(url).bearer_auth(&creds.token).send().await?;

    let raw_json = request.text().await?;
    // Parse the raw JSON to a serde_json::Value to get all fields, even those not in RunnerDetail
    let _v: serde_json::Value = serde_json::from_str(&raw_json)?;
    let detail = serde_json::from_str::<RunnerDetail>(&raw_json);

    match detail {
        Err(e) => {
            println!("Failed parsing {:#?}", e);
            println!("Untyped parsed JSON:\n{:#?}", _v);
            println!("raw output:\n{}", raw_json);
            Err(e.into())
        }
        Ok(d) => Ok(d),
    }
}
