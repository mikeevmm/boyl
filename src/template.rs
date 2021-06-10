use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    name: String,
    directory_name: PathBuf,
}

impl Template {
    pub fn infer_from_path(path: PathBuf) -> Self {
        todo!()
    }
}