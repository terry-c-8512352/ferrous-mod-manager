#[derive(Debug)]
pub struct ModDescriptor {
    pub tags: Vec<String>,
    pub name: String,
    pub path: String,
    pub supported_version: String,
    pub remote_file_id: String,
    pub picture: Option<String>,
    pub version: Option<String>,
}
