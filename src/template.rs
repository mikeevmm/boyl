use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub description: Option<String>,
    pub directory_name: PathBuf,
}