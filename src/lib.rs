use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Check if a path matches the given extension filter.
/// Returns true if no filter is provided, or if the file extension matches one in the filter.
pub fn matches_extension(path: &Path, extensions: Option<&HashSet<OsString>>) -> bool {
    match extensions {
        None => true, // No filter, accept all files
        Some(exts) => {
            if let Some(ext) = path.extension() {
                exts.contains(ext)
            } else {
                false // File has no extension, doesn't match filter
            }
        }
    }
}

/// Recursively list all files in a directory or return a single file if the path is a file.
/// This function will traverse all subdirectories and collect all file paths.
pub fn recursively_list_files(path: PathBuf) -> std::io::Result<Vec<PathBuf>> {
    let mut pending = vec![path];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        if path.is_file() {
            files.push(path);
        } else {
            let entries = path.read_dir()?;
            for entry in entries {
                pending.push(entry?.path());
            }
        }
    }

    Ok(files)
}
