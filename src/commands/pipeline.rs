use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Pipeline {
    pub id: u32,
    pub project_id: u32,
    #[serde(rename = "ref")]
    pub rref: String,
    pub status: String,
    pub source: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub web_url: String,
}
