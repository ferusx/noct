// SPDX-License-Identifier: BSD-3-Clause

use std::fs;
use std::path::{Path, PathBuf};

use crate::format::human_size;
use crate::themes::AnsiPalette;

#[derive(Debug)]
struct DuEntry {
    name: String,
    size: u64,
    is_dir: bool,
    had_error: bool,
}

#[derive(Debug)]
struct DuOptions {
    path: PathBuf,
    limit: Option<usize>,
}

pub fn handle_du_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(du_index) = arguments.iter().position(|argument| argument == "--du") else {
        return false;
    };

    let options = du_options(arguments, du_index);

    print_disk_usage(&options, palette);

    true
}

fn du_options(arguments: &[String], du_index: usize) -> DuOptions {
    let mut index = du_index + 1;

    let mut path = PathBuf::from(".");

    let mut limit = None;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--limit" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --limit requires a number");

                    std::process::exit(2);
                }

                let Some(parsed_limit) = parse_limit_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid limit '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                let limit_text = argument.trim_start_matches("--limit=");

                let Some(parsed_limit) = parse_limit_argument(limit_text) else {
                    eprintln!("noct: invalid limit '{}'", limit_text);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 1;
            }

            argument if argument.starts_with('-') => {
                index += 1;
            }

            argument => {
                path = PathBuf::from(argument);

                index += 1;
            }
        }
    }

    DuOptions { path, limit }
}

fn parse_limit_argument(value: &str) -> Option<usize> {
    let limit = value.trim().parse::<usize>().ok()?;

    if limit == 0 {
        return None;
    }

    Some(limit)
}

fn print_disk_usage(options: &DuOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if metadata.is_file() {
        print_single_file_usage(path, metadata.len(), palette);

        return;
    }

    if !metadata.is_dir() {
        print_single_file_usage(path, metadata.len(), palette);

        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    let mut du_entries = Vec::new();

    for result in entries {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                du_entries.push(DuEntry {
                    name,
                    size: 0,
                    is_dir: false,
                    had_error: true,
                });

                continue;
            }
        };

        let is_dir = metadata.is_dir();

        let (size, had_error) = entry_size(&entry_path, &metadata);

        du_entries.push(DuEntry {
            name,
            size,
            is_dir,
            had_error,
        });
    }

    du_entries.sort_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    if let Some(limit) = options.limit {
        du_entries.truncate(limit);
    }

    println!("{}Disk usage:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();

    let mut total = 0u64;

    for entry in &du_entries {
        total = total.saturating_add(entry.size);

        print_du_entry(entry, palette);
    }

    println!();

    println!(
        "{}Total listed size:{} {}{}{}",
        palette.report.total,
        palette.reset,
        palette.report.size,
        human_size(total),
        palette.reset,
    );
}

fn print_single_file_usage(path: &Path, size: u64, palette: &AnsiPalette) {
    println!(
        "{}{:>8}{}  {}{}{}",
        palette.report.size,
        human_size(size),
        palette.reset,
        palette.report.path,
        path.display(),
        palette.reset,
    );
}

fn print_du_entry(entry: &DuEntry, palette: &AnsiPalette) {
    let suffix = if entry.is_dir { "/" } else { "" };

    let name_color = if entry.had_error {
        &palette.report.attention
    } else if entry.is_dir {
        &palette.report.path
    } else {
        &palette.report.name
    };

    println!(
        "{}{:>8}{}  {}{}{}{}",
        palette.report.size,
        human_size(entry.size),
        palette.reset,
        name_color,
        entry.name,
        suffix,
        palette.reset,
    );
}

fn entry_size(path: &Path, metadata: &fs::Metadata) -> (u64, bool) {
    if metadata.is_file() {
        return (metadata.len(), false);
    }

    if !metadata.is_dir() {
        return (metadata.len(), false);
    }

    directory_size(path)
}

fn directory_size(path: &Path) -> (u64, bool) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(_) => {
            return (0, true);
        }
    };

    let mut total = 0u64;

    let mut had_error = false;

    for result in entries {
        let Ok(dir_entry) = result else {
            had_error = true;
            continue;
        };

        let entry_path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                had_error = true;
                continue;
            }
        };

        if metadata.is_dir() {
            let (size, child_had_error) = directory_size(&entry_path);

            total = total.saturating_add(size);

            if child_had_error {
                had_error = true;
            }
        } else {
            total = total.saturating_add(metadata.len());
        }
    }

    (total, had_error)
}
