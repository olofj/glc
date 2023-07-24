use serde_derive::{Deserialize, Serialize};
use serde_yaml;
use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    token: String,
    url: String,
}

pub fn login(token: &str, url: &str) -> std::io::Result<()> {
    let creds = Credentials {
        token: token.to_string(),
        url: url.to_string(),
    };
    let creds_string = serde_yaml::to_string(&creds).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::Other, "Failed to serialize credentials")
    })?;
    let home_dir = env::var("HOME").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;
    let creds_path = format!("{}/.creds", home_dir);
    let mut file = File::create(&creds_path).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create credentials file at {}", creds_path),
        )
    })?;
    file.write_all(creds_string.as_bytes())
}
