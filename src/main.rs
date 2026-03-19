/*
 *    LogWatch - `tail -f` but include new files that match filters
 *    Copyright (C) 2026 Tobias Ollmann
 *
 *    This program is free software: you can redistribute it and/or modify
 *    it under the terms of the GNU General Public License as published by
 *    the Free Software Foundation, either version 3 of the License, or
 *    (at your option) any later version.
 *
 *    This program is distributed in the hope that it will be useful,
 *    but WITHOUT ANY WARRANTY; without even the implied warranty of
 *    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *    GNU General Public License for more details.
 *
 *    You should have received a copy of the GNU General Public License
 *    along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

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
use log_watch::{matches_extension, recursively_list_files};

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

    let extensions = cli.extension.as_ref().map(|e| {
        e.iter()
            .map(|ext| {
                // Normalize extensions by removing leading dot if present
                let normalized = ext.strip_prefix('.').unwrap_or(ext);
                OsString::from(normalized)
            })
            .collect::<HashSet<_>>()
    });

    let mut offsets = HashMap::new();
    for path in cli.watch {
        watcher.watch(&path, notify::RecursiveMode::Recursive)?;
        let files = recursively_list_files(path)?;
        for f in files {
            if !matches_extension(&f, extensions.as_ref()) {
                continue;
            }
            let m = metadata(&f)?;
            offsets.insert(f, m.len());
        }
    }

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
                    if !matches_extension(path, extensions.as_ref()) {
                        continue;
                    }
                    if last_file != Some(path.clone()) {
                        let mut stdout = stdout().lock();
                        stdout.write_all(path.to_string_lossy().as_bytes())?;
                        stdout.write_all(&[b'\n'])?;
                        stdout.flush()?;
                        last_file = Some(path.clone());
                    }

                    let offset = offsets.entry(path.clone()).or_insert(0);
                    let mut f = File::open(path)?;

                    // Check for file truncation (common in log rotation)
                    let current_size = f.metadata()?.len();
                    if current_size < *offset {
                        // File was truncated, reset offset to current_size.
                        // If the file was shortened, we don't want to print the kept part again.
                        // If the file was truncated, we should get a modification event with current_size = 0 anyway
                        *offset = current_size;
                    }

                    f.seek(std::io::SeekFrom::Start(*offset))?;
                    let mut buf = vec![];
                    f.read_to_end(&mut buf)?;
                    *offset += buf.len() as u64;

                    let mut stdout = stdout().lock();
                    stdout.write_all(&buf)?;
                    stdout.flush()?;
                }
            }
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }

    Ok(())
}
