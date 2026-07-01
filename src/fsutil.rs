use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

/// Upper bound for any config-style file this app reads (descriptors, VDF,
/// dlc_load.json, collection JSON). These are all small in practice; the cap
/// only exists so a pathological file can't exhaust memory.
pub const MAX_READ_BYTES: u64 = 16 * 1024 * 1024;

/// `fs::read_to_string` with a size cap, checked on the already-opened handle
/// so the file can't be swapped between the check and the read.
pub fn read_to_string_limited(path: &Path, max_bytes: u64) -> io::Result<String> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    if len > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} is {len} bytes, over the {max_bytes}-byte limit",
                path.display()
            ),
        ));
    }
    let mut contents = String::with_capacity(len as usize);
    file.take(max_bytes).read_to_string(&mut contents)?;
    Ok(contents)
}

/// Write `contents` to `path` atomically: write to a temp file in the same
/// directory, fsync, then rename over the destination. A crash mid-write can
/// no longer leave a truncated file, and an attacker-planted symlink at `path`
/// is replaced rather than followed.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let tmp = dir
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.{}.tmp", std::process::id()));

    let result = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fsutil_test_{}_{name}", std::process::id()))
    }

    #[test]
    fn test_write_atomic_creates_and_replaces() {
        let path = temp_path("replace.txt");
        write_atomic(&path, "first").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");
        write_atomic(&path, "second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_write_atomic_replaces_symlink_instead_of_following_it() {
        #[cfg(unix)]
        {
            let target = temp_path("symlink_target.txt");
            let link = temp_path("symlink_link.txt");
            fs::write(&target, "original").unwrap();
            let _ = fs::remove_file(&link);
            std::os::unix::fs::symlink(&target, &link).unwrap();

            write_atomic(&link, "new contents").unwrap();
            // The symlink itself was replaced by a regular file; its old
            // target must be untouched.
            assert_eq!(fs::read_to_string(&target).unwrap(), "original");
            assert!(!fs::symlink_metadata(&link).unwrap().is_symlink());
            assert_eq!(fs::read_to_string(&link).unwrap(), "new contents");

            let _ = fs::remove_file(&target);
            let _ = fs::remove_file(&link);
        }
    }

    #[test]
    fn test_read_limited_rejects_oversized_file() {
        let path = temp_path("oversized.txt");
        fs::write(&path, "0123456789").unwrap();
        let err = read_to_string_limited(&path, 5).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(read_to_string_limited(&path, 10).unwrap(), "0123456789");
        let _ = fs::remove_file(&path);
    }
}
