use serde_derive::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub token: String,
    pub url: String,
}

pub fn load_credentials() -> std::io::Result<Credentials> {
    let home_dir = env::var("HOME").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;
    let creds_path = format!("{}/.creds", home_dir);
    let file = File::open(&creds_path).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Failed to open credentials file at {}", creds_path),
        )
    })?;
    let reader = BufReader::new(file);
    let creds: Credentials = serde_yaml::from_reader(reader).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse credentials file: {}", e),
        )
    })?;
    Ok(creds)
}
