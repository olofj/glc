use serde_derive::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Runner {
    pub id: usize,
    pub description: String,
    pub ip_address: String,
    pub active: bool,
    pub paused: bool,
    pub is_shared: bool,
    pub runner_type: String,
    pub name: Option<String>,
    pub online: bool,
    pub status: String,
}
