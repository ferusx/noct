// SPDX-License-Identifier: BSD-3-Clause

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::args::parse_size_argument;
use crate::format::{human_age, human_date, human_size};
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct ColdEntry {
    path: String,
    modified: Option<SystemTime>,
    kind: &'static str,
    size: u64,
}

#[derive(Debug, Clone, Copy)]
enum ColdSortMode {
    Age,
    Size,
    Path,
}

#[derive(Debug)]
struct ColdOptions {
    path: PathBuf,
    sort_mode: ColdSortMode,
    reverse: bool,
    limit: Option<usize>,
    older_than: Duration,
    min_size: Option<u64>,
}

pub fn handle_cold_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(cold_index) = arguments.iter().position(|argument| argument == "--cold") else {
        return false;
    };

    let options = cold_options(arguments, cold_index);

    print_cold_entries(&options, palette);

    true
}

fn cold_options(arguments: &[String], cold_index: usize) -> ColdOptions {
    let mut index = cold_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = ColdSortMode::Age;

    let mut reverse = false;

    let mut limit = None;

    let mut older_than = Duration::from_secs(60 * 60 * 24 * 180);

    let mut min_size = None;

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

                let Some(parsed_limit) = parse_usize_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid limit '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                let limit_text = argument.trim_start_matches("--limit=");

                let Some(parsed_limit) = parse_usize_argument(limit_text) else {
                    eprintln!("noct: invalid limit '{}'", limit_text);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 1;
            }

            "--older-than" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --older-than requires an age, for example 30d or 6mo");

                    std::process::exit(2);
                }

                let Some(parsed_age) = parse_duration_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid older-than age '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                older_than = parsed_age;

                index += 2;
            }

            argument if argument.starts_with("--older-than=") => {
                let age_text = argument.trim_start_matches("--older-than=");

                let Some(parsed_age) = parse_duration_argument(age_text) else {
                    eprintln!("noct: invalid older-than age '{}'", age_text);

                    std::process::exit(2);
                };

                older_than = parsed_age;

                index += 1;
            }

            "--min-size" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --min-size requires a size, for example 10M or 1G");

                    std::process::exit(2);
                }

                let Some(parsed_size) = parse_size_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid min size '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                min_size = Some(parsed_size);

                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                let size_text = argument.trim_start_matches("--min-size=");

                let Some(parsed_size) = parse_size_argument(size_text) else {
                    eprintln!("noct: invalid min size '{}'", size_text);

                    std::process::exit(2);
                };

                min_size = Some(parsed_size);

                index += 1;
            }

            "-r" | "--reverse" => {
                reverse = true;

                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --cold --sort requires age, size, or path");

                    std::process::exit(2);
                }

                sort_mode = parse_cold_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_cold_sort_mode(sort_text);

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

    ColdOptions {
        path,
        sort_mode,
        reverse,
        limit,
        older_than,
        min_size,
    }
}

fn parse_usize_argument(value: &str) -> Option<usize> {
    let number = value.trim().parse::<usize>().ok()?;

    if number == 0 {
        return None;
    }

    Some(number)
}

fn parse_duration_argument(value: &str) -> Option<Duration> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    let mut number_part = String::new();

    let mut unit_part = String::new();

    for character in trimmed.chars() {
        if character.is_ascii_digit() {
            if !unit_part.is_empty() {
                return None;
            }

            number_part.push(character);
        } else {
            unit_part.push(character);
        }
    }

    if number_part.is_empty() || unit_part.is_empty() {
        return None;
    }

    let number = number_part.parse::<u64>().ok()?;

    let seconds_per_unit = match unit_part.to_lowercase().as_str() {
        "m" | "min" | "mins" | "minute" | "minutes" => 60,

        "h" | "hr" | "hrs" | "hour" | "hours" => 60 * 60,

        "d" | "day" | "days" => 60 * 60 * 24,

        "w" | "week" | "weeks" => 60 * 60 * 24 * 7,

        "mo" | "mon" | "month" | "months" => 60 * 60 * 24 * 30,

        "y" | "yr" | "yrs" | "year" | "years" => 60 * 60 * 24 * 365,

        _ => {
            return None;
        }
    };

    number
        .checked_mul(seconds_per_unit)
        .map(Duration::from_secs)
}

fn parse_cold_sort_mode(value: &str) -> ColdSortMode {
    match value {
        "age" | "old" | "time" | "modified" => ColdSortMode::Age,

        "size" => ColdSortMode::Size,

        "path" | "name" => ColdSortMode::Path,

        _ => {
            eprintln!("noct: unknown --cold sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_cold_entries(options: &ColdOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --cold requires a directory");

        std::process::exit(2);
    }

    let mut entries = Vec::new();

    collect_cold_entries(path, path, options, &mut entries);

    sort_cold_entries(&mut entries, options.sort_mode);

    if options.reverse {
        entries.reverse();
    }

    if let Some(limit) = options.limit {
        entries.truncate(limit);
    }

    print_header(path, palette);

    if entries.is_empty() {
        println!(
            "  {}no cold entries found{}",
            palette.report.muted, palette.reset,
        );

        println!();

        print_summary(&entries, palette);

        return;
    }

    print_column_header(palette);

    for entry in &entries {
        print_cold_entry(entry, palette);
    }

    println!();

    print_summary(&entries, palette);
}

fn collect_cold_entries(
    base_path: &Path,
    current_path: &Path,
    options: &ColdOptions,
    entries: &mut Vec<ColdEntry>,
) {
    let read_dir = match fs::read_dir(current_path) {
        Ok(read_dir) => read_dir,

        Err(_) => {
            return;
        }
    };

    for result in read_dir {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                continue;
            }
        };

        let modified = metadata.modified().ok();

        let kind = if metadata.is_dir() {
            "dir"
        } else if metadata.file_type().is_symlink() {
            "link"
        } else if metadata.is_file() {
            "file"
        } else {
            "special"
        };

        let size = if metadata.is_dir() && !metadata.file_type().is_symlink() {
            directory_size(&entry_path)
        } else {
            metadata.len()
        };

        if is_older_than(modified, options.older_than) {
            if let Some(min_size) = options.min_size {
                if size < min_size {
                    if metadata.is_dir() && !metadata.file_type().is_symlink() {
                        collect_cold_entries(base_path, &entry_path, options, entries);
                    }

                    continue;
                }
            }

            let relative_path = entry_path
                .strip_prefix(base_path)
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|_| entry_path.display().to_string());

            let display_path = display_path_for(&relative_path, kind);

            entries.push(ColdEntry {
                path: display_path,
                modified,
                kind,
                size,
            });
        }

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            collect_cold_entries(base_path, &entry_path, options, entries);
        }
    }
}

fn is_older_than(modified: Option<SystemTime>, limit: Duration) -> bool {
    let Some(modified) = modified else {
        return false;
    };

    SystemTime::now()
        .duration_since(modified)
        .map(|age| age >= limit)
        .unwrap_or(false)
}

fn directory_size(path: &Path) -> u64 {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(_) => {
            return 0;
        }
    };

    let mut total = 0u64;

    for result in entries {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                continue;
            }
        };

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            total = total.saturating_add(directory_size(&entry_path));
        } else {
            total = total.saturating_add(metadata.len());
        }
    }

    total
}

fn display_path_for(path: &str, kind: &str) -> String {
    let suffix = match kind {
        "dir" => "/",

        "link" => "@",

        _ => "",
    };

    format!("{}{}", path, suffix)
}

fn sort_cold_entries(entries: &mut [ColdEntry], sort_mode: ColdSortMode) {
    match sort_mode {
        ColdSortMode::Age => {
            entries.sort_by(|left, right| {
                left.modified
                    .cmp(&right.modified)
                    .then_with(|| left.path.cmp(&right.path))
            });
        }

        ColdSortMode::Size => {
            entries.sort_by(|left, right| {
                right
                    .size
                    .cmp(&left.size)
                    .then_with(|| left.modified.cmp(&right.modified))
                    .then_with(|| left.path.cmp(&right.path))
            });
        }

        ColdSortMode::Path => {
            entries.sort_by(|left, right| left.path.cmp(&right.path));
        }
    }
}

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Cold entries:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();
}

fn print_column_header(palette: &AnsiPalette) {
    println!(
        "  {}{:>8}{}  {}{:>12}{}  {}{:<7}{}  {}{:>8}{}  {}{}{}",
        palette.report.label,
        "age",
        palette.reset,
        palette.report.label,
        "modified",
        palette.reset,
        palette.report.label,
        "kind",
        palette.reset,
        palette.report.label,
        "size",
        palette.reset,
        palette.report.label,
        "path",
        palette.reset,
    );
}

fn print_cold_entry(entry: &ColdEntry, palette: &AnsiPalette) {
    let age = human_age(entry.modified);

    let date = human_date(entry.modified);

    let size = human_size(entry.size);

    let kind_color = match entry.kind {
        "dir" => &palette.report.path,

        "link" => &palette.report.package,

        "file" => &palette.report.total,

        _ => &palette.report.attention,
    };

    let path_color = match entry.kind {
        "dir" => &palette.report.path,

        "link" => &palette.report.package,

        "file" => &palette.report.name,

        _ => &palette.report.attention,
    };

    println!(
        "  {}{:>8}{}  {}{:>12}{}  {}{:<7}{}  {}{:>8}{}  {}{}{}",
        palette.report.count,
        age,
        palette.reset,
        palette.columns.time,
        date,
        palette.reset,
        kind_color,
        entry.kind,
        palette.reset,
        palette.report.size,
        size,
        palette.reset,
        path_color,
        entry.path,
        palette.reset,
    );
}

fn print_summary(entries: &[ColdEntry], palette: &AnsiPalette) {
    let total_size = entries
        .iter()
        .map(|entry| entry.size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} cold entries shown",
        palette.report.total,
        entries.len(),
        palette.reset,
    );

    println!(
        "  {}{}{} listed",
        palette.report.size,
        human_size(total_size,),
        palette.reset,
    );

    println!();

    println!(
        "{}Tip: Use --limit N to restrict entries, or --reverse to invert the selected sort.{}",
        palette.report.muted, palette.reset,
    );
}
