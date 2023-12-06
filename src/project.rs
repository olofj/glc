use serde_derive::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Project {
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub default_branch: String,
    pub description: Option<String>,
    pub forks_count: usize,
    pub http_url_to_repo: String,
    pub id: usize,
    pub last_activity_at: String,
    pub name: String,
    pub name_with_namespace: String,
    //pub namespace: Option<Namespace>,
    pub path: String,
    pub path_with_namespace: String,
    pub readme_url: String,
    pub ssh_url_to_repo: String,
    pub star_count: usize,
    pub tag_list: Vec<String>,
    pub topics: Vec<String>,
    pub web_url: String,
}
