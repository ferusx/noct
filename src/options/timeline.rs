use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::format::{human_age, human_date};
use crate::themes::AnsiPalette;

#[derive(Debug)]
struct TimelineEntry {
    name: String,
    modified: Option<SystemTime>,
    is_dir: bool,
    is_symlink: bool,
}

#[derive(Debug)]
struct TimelineOptions {
    path: PathBuf,
    limit: Option<usize>,
    since: Option<Duration>,
}

pub fn handle_timeline_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(timeline_index) = arguments
        .iter()
        .position(|argument| argument == "--timeline")
    else {
        return false;
    };

    let options = timeline_options(arguments, timeline_index);

    print_timeline(&options, palette);

    true
}

fn parse_limit_argument(value: &str) -> Option<usize> {
    let limit = value.trim().parse::<usize>().ok()?;

    if limit == 0 {
        return None;
    }

    Some(limit)
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

fn timeline_options(arguments: &[String], timeline_index: usize) -> TimelineOptions {
    let mut index = timeline_index + 1;

    let mut path = PathBuf::from(".");

    let mut limit = None;

    let mut since = None;

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

            "--since" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --since requires an age, for example 12h or 7d");

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

            argument if argument.starts_with('-') => {
                index += 1;
            }

            argument => {
                path = PathBuf::from(argument);

                index += 1;
            }
        }
    }

    TimelineOptions { path, limit, since }
}

fn print_timeline(options: &TimelineOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --timeline requires a directory");

        std::process::exit(2);
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut timeline_entries = Vec::new();

    for result in entries {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                timeline_entries.push(TimelineEntry {
                    name,
                    modified: None,
                    is_dir: false,
                    is_symlink: false,
                });

                continue;
            }
        };

        let modified = metadata.modified().ok();

        if let Some(since) = options.since {
            if !is_within_since(modified, since) {
                continue;
            }
        }

        timeline_entries.push(TimelineEntry {
            name,
            modified,
            is_dir: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
        });
    }

    timeline_entries.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    println!("{}Timeline:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();

    let mut buckets: Vec<Vec<TimelineEntry>> = vec![
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];

    let limited_entries: Vec<TimelineEntry> = match options.limit {
        Some(limit) => timeline_entries.into_iter().take(limit).collect(),

        None => timeline_entries,
    };

    for entry in limited_entries {
        let bucket_index = timeline_bucket_index(entry.modified);

        buckets[bucket_index].push(entry);
    }

    let headings = [
        "Last hour",
        "Today",
        "This week",
        "This month",
        "This year",
        "Older",
        "Unknown",
    ];

    for (index, heading) in headings.iter().enumerate() {
        if buckets[index].is_empty() {
            continue;
        }

        print_timeline_heading(heading, palette);

        for entry in &buckets[index] {
            print_timeline_entry(entry, palette);
        }

        println!();
    }
}

fn timeline_bucket_index(modified: Option<SystemTime>) -> usize {
    let Some(modified) = modified else {
        return 6;
    };

    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::from_secs(0));

    let seconds = age.as_secs();

    if seconds <= 60 * 60 {
        0
    } else if seconds <= 60 * 60 * 24 {
        1
    } else if seconds <= 60 * 60 * 24 * 7 {
        2
    } else if seconds <= 60 * 60 * 24 * 30 {
        3
    } else if seconds <= 60 * 60 * 24 * 365 {
        4
    } else {
        5
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

fn print_timeline_heading(heading: &str, palette: &AnsiPalette) {
    println!("{}{}:{}", palette.report.heading, heading, palette.reset,);
}

fn print_timeline_entry(entry: &TimelineEntry, palette: &AnsiPalette) {
    let age = human_age(entry.modified);

    let date = human_date(entry.modified);

    let suffix = if entry.is_dir {
        "/"
    } else if entry.is_symlink {
        "@"
    } else {
        ""
    };

    let name_color = if entry.modified.is_none() {
        &palette.report.muted
    } else if entry.is_dir {
        &palette.report.path
    } else if entry.is_symlink {
        &palette.report.package
    } else {
        &palette.report.name
    };

    println!(
        "  {}{:>6}{}  {}{:>12}{}  {}{}{}{}",
        palette.report.count,
        age,
        palette.reset,
        palette.columns.time,
        date,
        palette.reset,
        name_color,
        entry.name,
        suffix,
        palette.reset,
    );
}
