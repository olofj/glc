use serde_derive::{Deserialize, Serialize};
use serde_yaml;
use std::env;
use std::fs::File;
use std::io::{self, Write};

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    token: String,
    url: String,
}

pub fn login(url: &str) -> std::io::Result<()> {
    let mut url = url.to_string();

    if !url.ends_with('/') {
        url.push('/');
    }

    println!("Please visit {}-/profile/personal_access_tokens and create a token with the following permissions:", url);
    println!("read_api, read_user, read_repository, read_registry. Then copy paste the token to the prompt below.");
    print!("Token: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();

    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let creds = Credentials {
        token: input.to_string(),
        url: url,
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
