#[derive(Debug)]
pub struct ModDescriptor {
    pub name: String,
    pub path: String,
    pub remote_file_id: String,
    pub supported_version: String,
    pub tags: Vec<String>,
    pub picture: Option<String>,
    pub version: Option<String>,
}
