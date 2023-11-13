use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;

use anyhow::Result;
use reqwest::StatusCode;
use zip::read::ZipArchive;
use zip::result::ZipError;

use crate::credentials::Credentials;

pub async fn get_artifact(
    credentials: &Credentials,
    project: &str,
    job: usize,
    artifact: String,
) -> Result<()> {
    let client = reqwest::Client::new();

    let url = format!(
        "{}/api/v4/projects/{}/jobs/{}/artifacts",
        credentials.url, project, job
    );

    let response = client
        .get(&url)
        .bearer_auth(&credentials.token)
        .send()
        .await?;

    // Check that we didn't receive an HTTP error status
    if response.status() != StatusCode::OK {
        println!("{:#?}", response);
        return Err(anyhow::anyhow!(
            "Received a non-OK HTTP status: {}",
            response.status()
        ));
    }

    // Get the response body
    let bytes = response.bytes().await?;

    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)?;
    let names = archive
        .file_names()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    match archive.by_name(artifact.as_str()) {
        Err(ZipError::FileNotFound) => {
            println!("File {} not found. Available files:", artifact);
            for f in names {
                println!("    {}", f);
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Received ZipError: {}", e));
        }
        Ok(mut file) => {
            let path = Path::new(&artifact);
            let filename: &OsStr = path.file_name().unwrap();
            // Open a file to write the artifact to
            let mut out = File::create(filename)?;
            io::copy(&mut file, &mut out)?;

            out.sync_all()?;

            let file_size = out.metadata()?.len();
            println!("Extracted {} bytes to {}", file_size, artifact);
        }
    };

    Ok(())
}
