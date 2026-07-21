//! Filesystem helpers used across skap.

use std::path::Path;

use anyhow::{Context, Result};

/// Atomic write: write to `<path>.tmp`, then rename to `<path>`.
/// Ensures the parent directory exists.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let tmp = path.with_extension(match path.extension() {
        Some(ext) => format!("{}.tmp", ext.to_string_lossy()),
        None => "tmp".to_string(),
    });
    std::fs::write(&tmp, contents).with_context(|| format!("failed to write {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("failed to rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Move a directory tree from `from` to `to`. Tries a plain rename first
/// (fast, atomic); if that fails because the two paths live on different
/// filesystems/mounts (`EXDEV`), falls back to a recursive copy followed
/// by removing the source.
pub fn move_dir(from: &Path, to: &Path) -> Result<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(libc_exdev()) => {
            copy_dir_recursive(from, to).with_context(|| {
                format!("failed to copy {} -> {}", from.display(), to.display())
            })?;
            std::fs::remove_dir_all(from).with_context(|| {
                format!(
                    "copied to {} but failed to remove {}",
                    to.display(),
                    from.display()
                )
            })?;
            Ok(())
        }
        Err(e) => {
            Err(e).with_context(|| format!("failed to move {} -> {}", from.display(), to.display()))
        }
    }
}

/// `EXDEV` ("Invalid cross-device link"). Hardcoded instead of pulling in
/// the `libc` crate for a single errno constant; this value is stable
/// across all Linux/macOS/BSD targets skap ships for.
fn libc_exdev() -> i32 {
    18
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else if file_type.is_symlink() {
            let target = std::fs::read_link(&src)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &dst)?;
            #[cfg(not(unix))]
            std::fs::copy(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// Pretty-print an absolute path with `~` for the user's home directory.
#[allow(dead_code)]
pub fn home_relative(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("skap-fs-test-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_atomic_creates_and_overwrites() {
        let dir = scratch_dir("write-atomic");
        let file = dir.join("nested").join("config.toml");
        write_atomic(&file, "a = 1\n").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "a = 1\n");
        write_atomic(&file, "a = 2\n").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "a = 2\n");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn move_dir_same_filesystem_preserves_contents() {
        let base = scratch_dir("move-same-fs");
        let from = base.join("from");
        let to = base.join("to");
        std::fs::create_dir_all(from.join("sub")).unwrap();
        std::fs::write(from.join("a.txt"), "hello").unwrap();
        std::fs::write(from.join("sub").join("b.txt"), "world").unwrap();

        move_dir(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(std::fs::read_to_string(to.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            std::fs::read_to_string(to.join("sub").join("b.txt")).unwrap(),
            "world"
        );
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn copy_dir_recursive_preserves_nested_structure() {
        let base = scratch_dir("copy-recursive");
        let from = base.join("from");
        let to = base.join("to");
        std::fs::create_dir_all(from.join("a").join("b")).unwrap();
        std::fs::write(from.join("a").join("b").join("f.txt"), "data").unwrap();

        copy_dir_recursive(&from, &to).unwrap();

        assert_eq!(
            std::fs::read_to_string(to.join("a").join("b").join("f.txt")).unwrap(),
            "data"
        );
        // Source is untouched by a plain copy.
        assert!(from.join("a").join("b").join("f.txt").exists());
        std::fs::remove_dir_all(&base).ok();
    }
}
