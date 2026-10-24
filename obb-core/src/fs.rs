// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Atomically writes `bytes` to `path` using a sibling temporary file.
///
/// Writing to a sibling temporary file in the same directory ensures that the final rename
/// remains on the same filesystem, which is required for atomic replacement.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;

    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = parent.join(tmp_name);

    let res = (|| -> io::Result<()> {
        let mut file = File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;

        // If destination already exists, preserve its permissions on the temp file
        // before renaming, preventing existing restrictive permissions from being relaxed.
        if let Ok(metadata) = fs::metadata(path) {
            let _ = fs::set_permissions(&tmp_path, metadata.permissions());
        }

        fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    if let Err(err) = res {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("obb_test_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test temp dir");
        dir
    }

    #[test]
    fn test_write_new_file() {
        let dir = test_dir("write_new");
        let file_path = dir.join("test.txt");
        let content = b"hello pathfinder";

        atomic_write(&file_path, content).expect("atomic write succeeds");
        let read_back = fs::read(&file_path).expect("read file back");
        assert_eq!(read_back, content);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_overwrite_existing_file() {
        let dir = test_dir("overwrite");
        let file_path = dir.join("test.txt");

        atomic_write(&file_path, b"initial content").expect("first write succeeds");
        atomic_write(&file_path, b"updated content").expect("overwrite write succeeds");

        let read_back = fs::read(&file_path).expect("read file back");
        assert_eq!(read_back, b"updated content");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_no_tmp_left_behind() {
        let dir = test_dir("no_tmp");
        let file_path = dir.join("test.txt");
        let tmp_path = dir.join("test.txt.tmp");

        atomic_write(&file_path, b"content").expect("atomic write succeeds");
        assert!(file_path.exists());
        assert!(!tmp_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_nonexistent_parent_dir_returns_error() {
        let dir = test_dir("nonexistent_parent");
        let missing_subdir = dir.join("missing_dir").join("test.txt");

        let result = atomic_write(&missing_subdir, b"content");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn test_preserves_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = test_dir("preserve_perm");
        let file_path = dir.join("test.txt");

        atomic_write(&file_path, b"first").expect("initial write");
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600)).expect("set mode 0o600");

        atomic_write(&file_path, b"second").expect("overwrite");
        let metadata = fs::metadata(&file_path).expect("metadata");
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        let _ = fs::remove_dir_all(&dir);
    }
}
