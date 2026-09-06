use std::fs;
use std::path::{Path, PathBuf};

use crate::format::human_size;
use crate::themes::AnsiPalette;

const MAX_BAR_WIDTH: usize = 30;

#[derive(Debug)]
struct MapEntry {
    name: String,
    size: u64,
    is_dir: bool,
    had_error: bool,
}

#[derive(Debug)]
struct MapOptions {
    path: PathBuf,
    limit: Option<usize>,
}

pub fn handle_map_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(map_index) = arguments.iter().position(|argument| argument == "--map") else {
        return false;
    };

    let options = map_options(arguments, map_index);

    print_directory_map(&options, palette);

    true
}

fn map_options(arguments: &[String], map_index: usize) -> MapOptions {
    let mut index = map_index + 1;

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

    MapOptions { path, limit }
}

fn parse_limit_argument(value: &str) -> Option<usize> {
    let limit = value.trim().parse::<usize>().ok()?;

    if limit == 0 {
        return None;
    }

    Some(limit)
}

fn print_directory_map(options: &MapOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --map requires a directory");

        std::process::exit(2);
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut map_entries = Vec::new();

    for result in entries {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                map_entries.push(MapEntry {
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

        map_entries.push(MapEntry {
            name,
            size,
            is_dir,
            had_error,
        });
    }

    map_entries.sort_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    if let Some(limit) = options.limit {
        map_entries.truncate(limit);
    }

    println!("{}Directory map:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();

    let total = map_entries
        .iter()
        .fold(0u64, |sum, entry| sum.saturating_add(entry.size));

    let largest = map_entries
        .iter()
        .map(|entry| entry.size)
        .max()
        .unwrap_or(0);

    print_column_header(palette);

    for entry in &map_entries {
        print_map_entry(entry, largest, palette);
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

fn print_column_header(palette: &AnsiPalette) {
    println!(
        "  {}{:<bar_width$}{}  {}{:>8}{}  {}{}{}",
        palette.report.label,
        "usage",
        palette.reset,
        palette.report.size,
        "size",
        palette.reset,
        palette.report.name,
        "name",
        palette.reset,
        bar_width = MAX_BAR_WIDTH,
    );
}

fn print_map_entry(entry: &MapEntry, largest: u64, palette: &AnsiPalette) {
    let bar_width = bar_width_for_size(entry.size, largest);

    let bar_character = if palette.ansi16 { "#" } else { "▨" };

    let bar = if bar_width == 0 {
        "!".to_string()
    } else {
        bar_character.repeat(bar_width)
    };

    let padded_bar = format!("{:<width$}", bar, width = MAX_BAR_WIDTH,);

    let suffix = if entry.is_dir { "/" } else { "" };

    let bar_color = if entry.had_error {
        &palette.report.attention
    } else {
        &palette.report.label
    };

    let name_color = if entry.had_error {
        &palette.report.attention
    } else if entry.is_dir {
        &palette.report.path
    } else {
        &palette.report.name
    };

    println!(
        "  {}{}{}  {}{:>8}{}  {}{}{}{}",
        bar_color,
        padded_bar,
        palette.reset,
        palette.report.size,
        human_size(entry.size,),
        palette.reset,
        name_color,
        entry.name,
        suffix,
        palette.reset,
    );
}

fn bar_width_for_size(size: u64, largest: u64) -> usize {
    if size == 0 || largest == 0 {
        return 0;
    }

    let ratio = size as f64 / largest as f64;

    let width = (ratio.sqrt() * MAX_BAR_WIDTH as f64).round() as usize;

    width.max(1).min(MAX_BAR_WIDTH)
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
