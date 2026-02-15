#[derive(Debug)]
pub struct ModDescriptor {
    pub name: Option<String>,              //Required
    pub path: Option<String>,              //Required
    pub remote_file_id: Option<String>,    //Requried
    pub supported_version: Option<String>, //Required
    pub tags: Option<Vec<String>>,
    pub picture: Option<String>,
    pub version: Option<String>,
}

pub struct LibraryVdf {
    pub idx: u32,
    pub path: String,
    pub apps: Vec<u32>
}