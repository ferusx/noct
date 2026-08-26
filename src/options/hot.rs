use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::format::{human_age, human_date};
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct HotEntry {
    path: String,
    modified: Option<SystemTime>,
    kind: &'static str,
}

#[derive(Debug, Clone, Copy)]
enum HotSortMode {
    Age,
    Path,
}

#[derive(Debug)]
struct HotOptions {
    path: PathBuf,
    sort_mode: HotSortMode,
    limit: Option<usize>,
    since: Option<Duration>,
}

pub fn handle_hot_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(hot_index) = arguments.iter().position(|argument| argument == "--hot") else {
        return false;
    };

    let options = hot_options(arguments, hot_index);

    print_hot_entries(&options, palette);

    true
}

fn hot_options(arguments: &[String], hot_index: usize) -> HotOptions {
    let mut index = hot_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = HotSortMode::Age;

    let mut limit = Some(50);

    let mut since = Some(Duration::from_secs(60 * 60 * 24));

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

            "--since" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --since requires an age, for example 2h or 7d");

                    std::process::exit(2);
                }

                let Some(parsed_since) = parse_duration_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid since age '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                since = Some(parsed_since);

                index += 2;
            }

            argument if argument.starts_with("--since=") => {
                let since_text = argument.trim_start_matches("--since=");

                let Some(parsed_since) = parse_duration_argument(since_text) else {
                    eprintln!("noct: invalid since age '{}'", since_text);

                    std::process::exit(2);
                };

                since = Some(parsed_since);

                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --hot --sort requires age or path");

                    std::process::exit(2);
                }

                sort_mode = parse_hot_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_hot_sort_mode(sort_text);

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

    HotOptions {
        path,
        sort_mode,
        limit,
        since,
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

fn parse_hot_sort_mode(value: &str) -> HotSortMode {
    match value {
        "age" | "time" | "modified" => HotSortMode::Age,

        "path" | "name" => HotSortMode::Path,

        _ => {
            eprintln!("noct: unknown --hot sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_hot_entries(options: &HotOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --hot requires a directory");

        std::process::exit(2);
    }

    let mut entries = Vec::new();

    collect_hot_entries(path, path, options, &mut entries);

    sort_hot_entries(&mut entries, options.sort_mode);

    if let Some(limit) = options.limit {
        entries.truncate(limit);
    }

    print_header(path, palette);

    if entries.is_empty() {
        println!(
            "  {}no hot entries found{}",
            palette.report.muted, palette.reset,
        );

        println!();

        print_summary(&entries, palette);

        return;
    }

    print_column_header(palette);

    for entry in &entries {
        print_hot_entry(entry, palette);
    }

    println!();

    print_summary(&entries, palette);
}

fn collect_hot_entries(
    base_path: &Path,
    current_path: &Path,
    options: &HotOptions,
    entries: &mut Vec<HotEntry>,
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

        if let Some(since) = options.since {
            if !is_within_since(modified, since) {
                if metadata.is_dir() && !metadata.file_type().is_symlink() {
                    collect_hot_entries(base_path, &entry_path, options, entries);
                }

                continue;
            }
        }

        let relative_path = entry_path
            .strip_prefix(base_path)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| entry_path.display().to_string());

        let kind = if metadata.is_dir() {
            "dir"
        } else if metadata.file_type().is_symlink() {
            "link"
        } else if metadata.is_file() {
            "file"
        } else {
            "special"
        };

        let display_path = display_path_for(&relative_path, kind);

        entries.push(HotEntry {
            path: display_path,
            modified,
            kind,
        });

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            collect_hot_entries(base_path, &entry_path, options, entries);
        }
    }
}

fn is_within_since(modified: Option<SystemTime>, since: Duration) -> bool {
    let Some(modified) = modified else {
        return false;
    };

    SystemTime::now()
        .duration_since(modified)
        .map(|age| age <= since)
        .unwrap_or(false)
}

fn display_path_for(path: &str, kind: &str) -> String {
    let suffix = match kind {
        "dir" => "/",

        "link" => "@",

        _ => "",
    };

    format!("{}{}", path, suffix)
}

fn sort_hot_entries(entries: &mut [HotEntry], sort_mode: HotSortMode) {
    match sort_mode {
        HotSortMode::Age => {
            entries.sort_by(|left, right| {
                right
                    .modified
                    .cmp(&left.modified)
                    .then_with(|| left.path.cmp(&right.path))
            });
        }

        HotSortMode::Path => {
            entries.sort_by(|left, right| left.path.cmp(&right.path));
        }
    }
}

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Hot entries:{}", palette.report.heading, palette.reset,);

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
        "  {}{:>8}{}  {}{:>12}{}  {}{:<7}{}  {}{}{}",
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
        "path",
        palette.reset,
    );
}

fn print_hot_entry(entry: &HotEntry, palette: &AnsiPalette) {
    let age = human_age(entry.modified);

    let date = human_date(entry.modified);

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
        "  {}{:>8}{}  {}{:>12}{}  {}{:<7}{}  {}{}{}",
        palette.report.count,
        age,
        palette.reset,
        palette.columns.time,
        date,
        palette.reset,
        kind_color,
        entry.kind,
        palette.reset,
        path_color,
        entry.path,
        palette.reset,
    );
}

fn print_summary(entries: &[HotEntry], palette: &AnsiPalette) {
    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} hot entries shown",
        palette.report.total,
        entries.len(),
        palette.reset,
    );
}
