use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs::{File, metadata},
    io::{Read, Seek, Write, stdout},
    path::PathBuf,
    sync::mpsc,
};

use clap::Parser;
use notify::Watcher;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Directory or file to watch
    #[arg(required = true, short, long, value_name = "DIR")]
    watch: Vec<PathBuf>,
    /// Extensions to filter
    #[arg(short, long, value_name = "EXT")]
    extension: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
enum LogWatchError {
    #[error("failed to watch directory")]
    NotifyError(#[from] notify::Error),

    #[error("io error")]
    IoError(#[from] std::io::Error),
}

fn main() -> Result<(), LogWatchError> {
    let cli = Cli::parse();

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;

    let mut offsets = HashMap::new();
    for path in cli.watch {
        watcher.watch(&path, notify::RecursiveMode::Recursive)?;
        let files = recursively_list_files(path)?;
        for f in files {
            let m = metadata(&f)?;
            offsets.insert(f, m.len());
        }
    }

    let extensions = cli.extension.map(|e| {
        e.into_iter()
            .map(|e| OsString::from(e))
            .collect::<HashSet<_>>()
    });

    let mut last_file = None;

    for res in rx {
        match res {
            Ok(event) => {
                if event.kind.is_remove() {
                    for path in &event.paths {
                        offsets.remove(path);
                    }
                    continue;
                }
                if !event.kind.is_modify() {
                    continue;
                }
                let paths = event.paths.iter().collect::<HashSet<_>>();
                for path in paths {
                    if !path.try_exists()? {
                        // file move
                        offsets.remove(path);
                        continue;
                    }
                    if let Some(extensions) = extensions.as_ref() {
                        let extension = path.extension().map(|p| p.to_os_string());
                        if let Some(extension) = extension {
                            if !extensions.contains(&extension) {
                                continue;
                            }
                        }
                    }
                    if last_file != Some(path.clone()) {
                        stdout()
                            .lock()
                            .write_all(path.to_string_lossy().as_bytes())?;
                        stdout().lock().write_all(&[b'\n'])?;
                        last_file = Some(path.clone());
                    }
                    let offset = offsets.entry(path.clone()).or_insert(0);
                    let mut f = File::open(path)?;
                    f.seek(std::io::SeekFrom::Start(*offset))?;
                    let mut buf = vec![];
                    f.read_to_end(&mut buf)?;
                    *offset += buf.len() as u64;
                    stdout().lock().write_all(&buf)?;
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

fn recursively_list_files(path: PathBuf) -> std::io::Result<Vec<PathBuf>> {
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
