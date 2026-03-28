use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("ferrous-mod-manager")
        .join("mod-collections")
}

pub fn game_data_dir(app_id: u32) -> PathBuf {
    data_dir().join(app_id.to_string())
}
