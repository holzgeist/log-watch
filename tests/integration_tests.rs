use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

// Import the functions we want to test from the binary crate
// Note: These need to be made public in main.rs for testing

#[test]
fn test_recursively_list_files_single_file() -> std::io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("test.log");
    fs::File::create(&file_path)?;
    
    let files = log_watch::recursively_list_files(file_path.clone())?;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], file_path);
    
    Ok(())
}

#[test]
fn test_recursively_list_files_empty_directory() -> std::io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    let files = log_watch::recursively_list_files(temp_dir.path().to_path_buf())?;
    assert_eq!(files.len(), 0);
    
    Ok(())
}

#[test]
fn test_recursively_list_files_flat_directory() -> std::io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    fs::File::create(temp_dir.path().join("file1.log"))?;
    fs::File::create(temp_dir.path().join("file2.txt"))?;
    fs::File::create(temp_dir.path().join("file3.log"))?;
    
    let mut files = log_watch::recursively_list_files(temp_dir.path().to_path_buf())?;
    files.sort();
    
    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|p| p.file_name().unwrap() == "file1.log"));
    assert!(files.iter().any(|p| p.file_name().unwrap() == "file2.txt"));
    assert!(files.iter().any(|p| p.file_name().unwrap() == "file3.log"));
    
    Ok(())
}

#[test]
fn test_recursively_list_files_nested_directories() -> std::io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    fs::File::create(temp_dir.path().join("root.log"))?;
    
    let sub_dir1 = temp_dir.path().join("subdir1");
    fs::create_dir(&sub_dir1)?;
    fs::File::create(sub_dir1.join("nested1.log"))?;
    
    let sub_dir2 = temp_dir.path().join("subdir2");
    fs::create_dir(&sub_dir2)?;
    fs::File::create(sub_dir2.join("nested2.txt"))?;
    
    let deep_dir = sub_dir1.join("deep");
    fs::create_dir(&deep_dir)?;
    fs::File::create(deep_dir.join("deep.log"))?;
    
    let mut files = log_watch::recursively_list_files(temp_dir.path().to_path_buf())?;
    files.sort();
    
    assert_eq!(files.len(), 4);
    assert!(files.iter().any(|p| p.file_name().unwrap() == "root.log"));
    assert!(files.iter().any(|p| p.file_name().unwrap() == "nested1.log"));
    assert!(files.iter().any(|p| p.file_name().unwrap() == "nested2.txt"));
    assert!(files.iter().any(|p| p.file_name().unwrap() == "deep.log"));
    
    Ok(())
}

#[test]
fn test_recursively_list_files_ignores_subdirectories() -> std::io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    fs::File::create(temp_dir.path().join("file.log"))?;
    
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir)?;
    
    let files = log_watch::recursively_list_files(temp_dir.path().to_path_buf())?;
    
    // Should only contain the file, not the directory itself
    assert_eq!(files.len(), 1);
    assert!(files[0].is_file());
    
    Ok(())
}

#[test]
fn test_recursively_list_files_with_hidden_files() -> std::io::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    fs::File::create(temp_dir.path().join(".hidden"))?;
    fs::File::create(temp_dir.path().join("visible.log"))?;
    
    let files = log_watch::recursively_list_files(temp_dir.path().to_path_buf())?;
    
    // Should include hidden files
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|p| p.file_name().unwrap() == ".hidden"));
    assert!(files.iter().any(|p| p.file_name().unwrap() == "visible.log"));
    
    Ok(())
}

#[test]
fn test_matches_extension_no_filter() {
    let path = PathBuf::from("test.log");
    assert!(log_watch::matches_extension(&path, None));
    
    let path = PathBuf::from("test.txt");
    assert!(log_watch::matches_extension(&path, None));
    
    let path = PathBuf::from("no_extension");
    assert!(log_watch::matches_extension(&path, None));
}

#[test]
fn test_matches_extension_with_filter() {
    let mut exts = HashSet::new();
    exts.insert(OsString::from("log"));
    exts.insert(OsString::from("txt"));
    
    let path = PathBuf::from("test.log");
    assert!(log_watch::matches_extension(&path, Some(&exts)));
    
    let path = PathBuf::from("test.txt");
    assert!(log_watch::matches_extension(&path, Some(&exts)));
    
    let path = PathBuf::from("test.rs");
    assert!(!log_watch::matches_extension(&path, Some(&exts)));
}

#[test]
fn test_matches_extension_no_extension() {
    let mut exts = HashSet::new();
    exts.insert(OsString::from("log"));
    
    let path = PathBuf::from("no_extension");
    assert!(!log_watch::matches_extension(&path, Some(&exts)));
    
    let path = PathBuf::from("README");
    assert!(!log_watch::matches_extension(&path, Some(&exts)));
}

#[test]
fn test_matches_extension_case_sensitive() {
    let mut exts = HashSet::new();
    exts.insert(OsString::from("log"));
    
    let path = PathBuf::from("test.LOG");
    // Extensions are case-sensitive on Unix-like systems
    #[cfg(unix)]
    assert!(!log_watch::matches_extension(&path, Some(&exts)));
    
    // On Windows, they might be case-insensitive, but OsString comparison is exact
    #[cfg(windows)]
    assert!(!log_watch::matches_extension(&path, Some(&exts)));
}

#[test]
fn test_matches_extension_with_dots_in_filename() {
    let mut exts = HashSet::new();
    exts.insert(OsString::from("log"));
    
    let path = PathBuf::from("my.file.name.log");
    assert!(log_watch::matches_extension(&path, Some(&exts)));
    
    let path = PathBuf::from("my.file.name.txt");
    assert!(!log_watch::matches_extension(&path, Some(&exts)));
}