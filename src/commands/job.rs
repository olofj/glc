use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Artifact {
    pub file_type: String,
    pub size: usize,
    pub filename: String,
    pub file_format: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Job {
    pub id: usize,
    pub status: String,
    pub stage: String,
    pub name: String,
    #[serde(rename = "ref")]
    pub rref: String,
    pub tag: bool,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration: Option<f64>,
    pub queued_duration: Option<f64>,
    pub failure_reason: Option<String>,
    pub artifacts: Vec<Artifact>,
    // include other fields you are interested in
}
