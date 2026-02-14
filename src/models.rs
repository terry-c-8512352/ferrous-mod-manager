#[derive(Debug)]
pub struct ModDescriptor {
    pub name: Option<String>,
    pub path: Option<String>,
    pub remote_file_id: Option<String>,
    pub supported_version: Option<String>,
    pub tags: Option<Vec<String>>,
    pub picture: Option<String>,
    pub version: Option<String>,
}
