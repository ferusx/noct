// SPDX-License-Identifier: BSD-3-Clause

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::themes::AnsiPalette;

const LONG_NAME_LIMIT: usize = 28;

#[derive(Debug, Clone)]
struct NameEntry {
    name: String,
    display_name: String,
    is_dir: bool,
    is_symlink: bool,
    length: usize,
}

#[derive(Debug)]
struct NameReport {
    entries: Vec<NameEntry>,
    spaces: Vec<NameEntry>,
    uppercase: Vec<NameEntry>,
    hidden: Vec<NameEntry>,
    backup_like: Vec<NameEntry>,
    temp_like: Vec<NameEntry>,
    save_like: Vec<NameEntry>,
    numeric_suffix: Vec<NameEntry>,
    no_extension: Vec<NameEntry>,
    leading_dash: Vec<NameEntry>,
    shell_special: Vec<NameEntry>,
    long_names: Vec<NameEntry>,
    short_names: Vec<NameEntry>,
    case_collisions: Vec<Vec<NameEntry>>,
}

#[derive(Debug)]
struct NameOptions {
    path: PathBuf,
    limit: Option<usize>,
    since: Option<Duration>,
}

pub fn handle_names_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(names_index) = arguments.iter().position(|argument| argument == "--names") else {
        return false;
    };

    let options = name_options(arguments, names_index);

    print_name_analysis(&options, palette);

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

fn name_options(arguments: &[String], names_index: usize) -> NameOptions {
    let mut index = names_index + 1;

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

    NameOptions { path, limit, since }
}

fn print_name_analysis(options: &NameOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --names requires a directory");

        std::process::exit(2);
    }

    let report = analyze_names(path, options.since);

    print_header(path, palette);

    print_section(
        "Names with spaces",
        &report.spaces,
        Some("space"),
        options.limit,
        palette,
    );

    print_section(
        "Names with uppercase letters",
        &report.uppercase,
        Some("uppercase"),
        options.limit,
        palette,
    );

    print_section(
        "Hidden names",
        &report.hidden,
        Some("hidden"),
        options.limit,
        palette,
    );

    print_section(
        "Backup-looking names",
        &report.backup_like,
        Some("backup"),
        options.limit,
        palette,
    );

    print_section(
        "Temp-looking names",
        &report.temp_like,
        Some("temp"),
        options.limit,
        palette,
    );

    print_section(
        "Save/autosave-looking names",
        &report.save_like,
        Some("save"),
        options.limit,
        palette,
    );

    print_section(
        "Numeric suffix names",
        &report.numeric_suffix,
        Some("numbered"),
        options.limit,
        palette,
    );

    print_section(
        "Names without extensions",
        &report.no_extension,
        Some("no-ext"),
        options.limit,
        palette,
    );

    print_section(
        "Leading dash names",
        &report.leading_dash,
        Some("attention"),
        options.limit,
        palette,
    );

    print_section(
        "Shell-special names",
        &report.shell_special,
        Some("shell"),
        options.limit,
        palette,
    );

    print_section(
        "Longest names",
        &report.long_names,
        Some("long"),
        options.limit,
        palette,
    );

    print_section(
        "Shortest names",
        &report.short_names,
        Some("short"),
        options.limit,
        palette,
    );

    print_case_collisions(&report.case_collisions, options.limit, palette);

    print_summary(&report, palette);
}

fn analyze_names(path: &Path, since: Option<Duration>) -> NameReport {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut all_entries = Vec::new();

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

        let name = dir_entry.file_name().to_string_lossy().to_string();

        let modified = metadata.modified().ok();

        if let Some(since) = since {
            if !is_within_since(modified, since) {
                continue;
            }
        }

        let file_type = metadata.file_type();

        let is_dir = metadata.is_dir();

        let is_symlink = file_type.is_symlink();

        let display_name = display_name_for(&name, is_dir, is_symlink);

        let length = name.chars().count();

        all_entries.push(NameEntry {
            name,
            display_name,
            is_dir,
            is_symlink,
            length,
        });
    }

    all_entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    let spaces = matching_entries(&all_entries, |entry| {
        entry
            .name
            .chars()
            .any(|character| character.is_whitespace())
    });

    let uppercase = matching_entries(&all_entries, |entry| {
        entry.name.chars().any(|character| character.is_uppercase())
    });

    let hidden = matching_entries(&all_entries, |entry| entry.name.starts_with('.'));

    let backup_like = matching_entries(&all_entries, |entry| is_backup_like(&entry.name));

    let temp_like = matching_entries(&all_entries, |entry| is_temp_like(&entry.name));

    let save_like = matching_entries(&all_entries, |entry| is_save_like(&entry.name));

    let numeric_suffix = matching_entries(&all_entries, |entry| has_numeric_suffix(&entry.name));

    let no_extension = matching_entries(&all_entries, |entry| {
        !entry.is_dir
            && !entry.is_symlink
            && !entry.name.starts_with('.')
            && Path::new(&entry.name).extension().is_none()
    });

    let leading_dash = matching_entries(&all_entries, |entry| entry.name.starts_with('-'));

    let shell_special = matching_entries(&all_entries, |entry| {
        has_shell_special_character(&entry.name)
    });

    let mut long_names = all_entries.clone();

    long_names.sort_by(|left, right| {
        right
            .length
            .cmp(&left.length)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    long_names = long_names
        .into_iter()
        .filter(|entry| entry.length >= LONG_NAME_LIMIT)
        .take(10)
        .collect();

    let mut short_names = all_entries.clone();

    short_names.sort_by(|left, right| {
        left.length
            .cmp(&right.length)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    short_names = short_names.into_iter().take(10).collect();

    let case_collisions = find_case_collisions(&all_entries);

    NameReport {
        entries: all_entries,
        spaces,
        uppercase,
        hidden,
        backup_like,
        temp_like,
        save_like,
        numeric_suffix,
        no_extension,
        leading_dash,
        shell_special,
        long_names,
        short_names,
        case_collisions,
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

fn matching_entries(
    entries: &[NameEntry],
    predicate: impl Fn(&NameEntry) -> bool,
) -> Vec<NameEntry> {
    entries
        .iter()
        .filter(|entry| predicate(entry))
        .cloned()
        .collect()
}

fn display_name_for(name: &str, is_dir: bool, is_symlink: bool) -> String {
    let suffix = if is_dir {
        "/"
    } else if is_symlink {
        "@"
    } else {
        ""
    };

    format!("{}{}", name, suffix)
}

fn is_backup_like(name: &str) -> bool {
    let lower = name.to_lowercase();

    lower.ends_with('~')
        || lower.ends_with(".bak")
        || lower.ends_with(".backup")
        || lower.ends_with(".old")
        || lower.ends_with(".orig")
        || lower.ends_with(".original")
        || lower.ends_with(".copy")
        || lower.ends_with(".swp")
        || lower.ends_with(".tmp")
        || lower.ends_with(".temp")
        || lower.contains("backup")
        || lower.contains("_old")
        || lower.contains("-old")
}

fn is_temp_like(name: &str) -> bool {
    let lower = name.to_lowercase();

    lower.contains("tmp")
        || lower.contains("temp")
        || lower.ends_with(".swp")
        || lower.ends_with(".part")
        || lower.ends_with(".crdownload")
        || lower.ends_with(".download")
}

fn is_save_like(name: &str) -> bool {
    let lower = name.to_lowercase();

    lower.contains(".save")
        || lower.ends_with(".autosave")
        || lower.ends_with(".auto")
        || lower.ends_with(".recover")
        || lower.ends_with(".recovered")
}

fn has_numeric_suffix(name: &str) -> bool {
    let trimmed = name.trim_end_matches('/');

    let Some(last_part) = trimmed.rsplit('.').next() else {
        return false;
    };

    if last_part.is_empty() {
        return false;
    }

    last_part
        .chars()
        .all(|character| character.is_ascii_digit())
}

fn has_shell_special_character(name: &str) -> bool {
    const SPECIALS: [char; 18] = [
        ' ', '\t', '\n', '\'', '"', '`', '$', '&', '|', ';', '<', '>', '(', ')', '[', ']', '*', '?',
    ];

    name.chars().any(|character| SPECIALS.contains(&character))
}

fn find_case_collisions(entries: &[NameEntry]) -> Vec<Vec<NameEntry>> {
    let mut by_lowercase: BTreeMap<String, Vec<NameEntry>> = BTreeMap::new();

    for entry in entries {
        by_lowercase
            .entry(entry.name.to_lowercase())
            .or_default()
            .push(entry.clone());
    }

    by_lowercase
        .into_values()
        .filter(|group| group.len() > 1)
        .collect()
}

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Name analysis:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();
}

fn print_section(
    title: &str,
    entries: &[NameEntry],
    label: Option<&str>,
    limit: Option<usize>,
    palette: &AnsiPalette,
) {
    if entries.is_empty() {
        return;
    }

    println!("{}{}:{}", palette.report.heading, title, palette.reset,);

    let visible_count = limit.unwrap_or(entries.len()).min(entries.len());

    for entry in entries.iter().take(visible_count) {
        print_name_entry(entry, label, palette);
    }

    if visible_count < entries.len() {
        print_more_line(entries.len() - visible_count, palette);
    }

    println!();
}

fn print_name_entry(entry: &NameEntry, label: Option<&str>, palette: &AnsiPalette) {
    match label {
        Some(label) => {
            let label_color = if label == "attention" {
                &palette.report.attention
            } else {
                &palette.report.label
            };

            println!(
                "  {}{:<10}{}  {}{}{}",
                label_color,
                label,
                palette.reset,
                palette.report.name,
                entry.display_name,
                palette.reset,
            );
        }

        None => {
            println!(
                "  {}{}{}",
                palette.report.name, entry.display_name, palette.reset,
            );
        }
    }
}

fn print_more_line(remaining: usize, palette: &AnsiPalette) {
    println!(
        "  {}... {} more{}",
        palette.report.muted, remaining, palette.reset,
    );
}

fn print_case_collisions(groups: &[Vec<NameEntry>], limit: Option<usize>, palette: &AnsiPalette) {
    if groups.is_empty() {
        return;
    }

    println!(
        "{}Case-collision groups:{}",
        palette.report.heading, palette.reset,
    );

    let visible_count = limit.unwrap_or(groups.len()).min(groups.len());

    for group in groups.iter().take(visible_count) {
        for entry in group {
            print_name_entry(entry, Some("case"), palette);
        }

        println!();
    }

    if visible_count < groups.len() {
        print_more_line(groups.len() - visible_count, palette);
    }
}

fn print_summary(report: &NameReport, palette: &AnsiPalette) {
    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} entries scanned",
        palette.report.total,
        report.entries.len(),
        palette.reset,
    );

    println!(
        "  {}{}{} with spaces, {}{}{} hidden, {}{}{} backup/temp/save-looking",
        palette.report.count,
        report.spaces.len(),
        palette.reset,
        palette.report.count,
        report.hidden.len(),
        palette.reset,
        palette.report.count,
        report.backup_like.len() + report.temp_like.len() + report.save_like.len(),
        palette.reset,
    );

    println!(
        "  {}{}{} shell-special, {}{}{} case-collision groups",
        palette.report.count,
        report.shell_special.len(),
        palette.reset,
        palette.report.count,
        report.case_collisions.len(),
        palette.reset,
    );
}
