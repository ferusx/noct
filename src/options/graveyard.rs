// SPDX-License-Identifier: BSD-3-Clause

use crate::args::parse_size_argument;
use crate::format::{human_age, human_size};
use crate::themes::AnsiPalette;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const OLD_SECONDS: u64 = 60 * 60 * 24 * 180;
const BIG_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
struct GraveyardEntry {
    name: String,
    display_name: String,
    size: u64,
    modified: Option<SystemTime>,
    kind: &'static str,
    tags: Vec<&'static str>,
    reason: String,
    score: u32,
}

#[derive(Debug, Clone, Copy)]
enum GraveyardSortMode {
    Score,
    Size,
    Age,
    Name,
}

#[derive(Debug)]
struct GraveyardOptions {
    path: PathBuf,
    sort_mode: GraveyardSortMode,
    recursive: bool,
    limit: Option<usize>,
    min_size: Option<u64>,
    older_than: Option<Duration>,
    kind: Option<String>,
}

pub fn handle_graveyard_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(graveyard_index) = arguments
        .iter()
        .position(|argument| argument == "--graveyard")
    else {
        return false;
    };

    let options = graveyard_options(arguments, graveyard_index);

    print_graveyard(&options, palette);

    true
}

fn graveyard_options(arguments: &[String], graveyard_index: usize) -> GraveyardOptions {
    let mut index = graveyard_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = GraveyardSortMode::Score;

    let mut recursive = false;

    let mut limit = None;

    let mut min_size = None;

    let mut older_than = None;

    let mut kind = None;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--recursive" | "-R" => {
                recursive = true;

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

            "--min-size" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --min-size requires a size, for example 100M or 2G");

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

            "--older-than" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --older-than requires an age, for example 30d or 6mo");

                    std::process::exit(2);
                }

                let Some(parsed_age) = parse_duration_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid older-than age '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                older_than = Some(parsed_age);

                index += 2;
            }

            argument if argument.starts_with("--older-than=") => {
                let age_text = argument.trim_start_matches("--older-than=");

                let Some(parsed_age) = parse_duration_argument(age_text) else {
                    eprintln!("noct: invalid older-than age '{}'", age_text);

                    std::process::exit(2);
                };

                older_than = Some(parsed_age);

                index += 1;
            }

            "--kind" => {
                if index + 1 >= arguments.len() {
                    eprintln!(
                        "noct: --kind requires one of cache, trash, backup, temp, save, log, archive, empty, large, old"
                    );

                    std::process::exit(2);
                }

                let Some(parsed_kind) = normalize_kind(&arguments[index + 1]) else {
                    eprintln!("noct: unknown graveyard kind '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                kind = Some(parsed_kind);

                index += 2;
            }

            argument if argument.starts_with("--kind=") => {
                let kind_text = argument.trim_start_matches("--kind=");

                let Some(parsed_kind) = normalize_kind(kind_text) else {
                    eprintln!("noct: unknown graveyard kind '{}'", kind_text);

                    std::process::exit(2);
                };

                kind = Some(parsed_kind);

                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --graveyard --sort requires score, size, age, or name");

                    std::process::exit(2);
                }

                sort_mode = parse_graveyard_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_graveyard_sort_mode(sort_text);

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

    GraveyardOptions {
        path,
        sort_mode,
        recursive,
        limit,
        min_size,
        older_than,
        kind,
    }
}
fn parse_graveyard_sort_mode(value: &str) -> GraveyardSortMode {
    match value {
        "score" | "rank" => GraveyardSortMode::Score,

        "size" => GraveyardSortMode::Size,

        "age" | "old" => GraveyardSortMode::Age,

        "name" => GraveyardSortMode::Name,

        _ => {
            eprintln!("noct: unknown --graveyard sort mode '{}'", value);

            std::process::exit(2);
        }
    }
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

fn normalize_kind(value: &str) -> Option<String> {
    let kind = value.trim().to_lowercase();

    match kind.as_str() {
        "cache" | "trash" | "backup" | "temp" | "save" | "log" | "archive" | "empty" | "large"
        | "old" => Some(kind),

        _ => None,
    }
}

fn print_graveyard(options: &GraveyardOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --graveyard requires a directory");

        std::process::exit(2);
    }

    let mut entries = collect_graveyard_entries(path, options);

    sort_graveyard_entries(&mut entries, options.sort_mode);

    if let Some(limit) = options.limit {
        entries.truncate(limit);
    }

    print_header(path, palette);

    if entries.is_empty() {
        println!(
            "  {}no cleanup candidates found{}",
            palette.report.total, palette.reset,
        );

        println!();

        print_summary(&entries, palette);

        return;
    }

    let name_width = graveyard_name_width(&entries);

    print_column_header(name_width, palette);

    for entry in &entries {
        print_graveyard_entry(entry, name_width, palette);
    }

    println!();

    print_summary(&entries, palette);
}

fn collect_graveyard_entries(path: &Path, options: &GraveyardOptions) -> Vec<GraveyardEntry> {
    let mut candidates = Vec::new();

    collect_graveyard_entries_inner(path, path, options, &mut candidates);

    candidates
}

fn collect_graveyard_entries_inner(
    base_path: &Path,
    current_path: &Path,
    options: &GraveyardOptions,
    candidates: &mut Vec<GraveyardEntry>,
) {
    let entries = match fs::read_dir(current_path) {
        Ok(entries) => entries,

        Err(_) => {
            return;
        }
    };

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

        let is_dir = metadata.is_dir();

        let is_symlink = metadata.file_type().is_symlink();

        let relative_name = entry_path
            .strip_prefix(base_path)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| name.clone());

        let display_name = display_name_for(&relative_name, is_dir, is_symlink);

        let modified = metadata.modified().ok();

        let size = if is_dir {
            directory_size(&entry_path)
        } else {
            metadata.len()
        };

        if let Some(candidate) = classify_candidate(name, display_name, size, modified, is_dir) {
            if candidate_matches_options(&candidate, options) {
                candidates.push(candidate);
            }
        }

        if options.recursive && is_dir && !is_symlink {
            collect_graveyard_entries_inner(base_path, &entry_path, options, candidates);
        }
    }
}

fn candidate_matches_options(candidate: &GraveyardEntry, options: &GraveyardOptions) -> bool {
    if let Some(min_size) = options.min_size {
        if candidate.size < min_size {
            return false;
        }
    }

    if let Some(older_than) = options.older_than {
        if !is_older_than(candidate.modified, older_than) {
            return false;
        }
    }

    if let Some(kind) = options.kind.as_ref() {
        if !candidate.tags.iter().any(|tag| *tag == kind.as_str()) {
            return false;
        }
    }

    true
}

fn classify_candidate(
    name: String,
    display_name: String,
    size: u64,
    modified: Option<SystemTime>,
    _is_dir: bool,
) -> Option<GraveyardEntry> {
    let lower = name.to_lowercase();

    let mut kind = None;

    let mut tags: Vec<&'static str> = Vec::new();

    let mut reason_parts: Vec<&'static str> = Vec::new();

    let mut score = 0u32;

    if is_cache_name(&lower) {
        kind = Some("cache");

        tags.push("cache");

        reason_parts.push("cache-like name");

        score += 80;
    }

    if is_trash_name(&lower) {
        kind = Some("trash");

        tags.push("trash");

        reason_parts.push("trash-like name");

        score += 90;
    }

    if is_backup_like(&lower) {
        kind = Some("backup");

        tags.push("backup");

        reason_parts.push("backup-like name");

        score += 60;
    }

    if is_temp_like(&lower) {
        kind = Some("temp");

        tags.push("temp");

        reason_parts.push("temp-like name");

        score += 65;
    }

    if is_save_like(&lower) {
        kind = Some("save");

        tags.push("save");

        reason_parts.push("save/autosave name");

        score += 55;
    }

    if is_log_like(&lower) {
        kind = Some("log");

        tags.push("log");

        reason_parts.push("log file");

        score += 35;
    }

    if is_archive_like(&lower) {
        kind = Some("archive");

        tags.push("archive");

        reason_parts.push("archive file");

        score += 30;
    }

    if size == 0 {
        kind = Some("empty");

        tags.push("empty");

        reason_parts.push("zero bytes");

        score += 25;
    }

    if size >= BIG_BYTES {
        if kind.is_none() {
            kind = Some("large");
        }

        tags.push("large");

        reason_parts.push("large");

        score += 20;
    }

    if is_older_than(modified, Duration::from_secs(OLD_SECONDS)) {
        if kind.is_none() {
            kind = Some("old");
        }

        tags.push("old");

        reason_parts.push("old");

        score += 15;
    }

    let kind = kind?;

    Some(GraveyardEntry {
        name,
        display_name,
        size,
        modified,
        kind,
        tags,
        reason: reason_parts.join(", "),
        score,
    })
}

fn is_cache_name(lower: &str) -> bool {
    lower == ".cache" || lower == "cache" || lower.contains("cache")
}

fn is_trash_name(lower: &str) -> bool {
    lower == ".trash"
        || lower == "trash"
        || lower.contains("trash")
        || lower.contains("graveyard")
        || lower.contains("recycle")
}

fn is_backup_like(lower: &str) -> bool {
    lower.ends_with('~')
        || lower.ends_with(".bak")
        || lower.ends_with(".backup")
        || lower.ends_with(".old")
        || lower.ends_with(".orig")
        || lower.ends_with(".original")
        || lower.ends_with(".copy")
        || lower.contains("backup")
        || lower.contains("_old")
        || lower.contains("-old")
}

fn is_temp_like(lower: &str) -> bool {
    lower.contains("tmp")
        || lower.contains("temp")
        || lower.ends_with(".swp")
        || lower.ends_with(".part")
        || lower.ends_with(".crdownload")
        || lower.ends_with(".download")
}

fn is_save_like(lower: &str) -> bool {
    lower.contains(".save")
        || lower.ends_with(".autosave")
        || lower.ends_with(".auto")
        || lower.ends_with(".recover")
        || lower.ends_with(".recovered")
}

fn is_log_like(lower: &str) -> bool {
    lower.ends_with(".log")
        || lower.ends_with(".log.1")
        || lower.contains("_log")
        || lower.contains("-log")
}

fn is_archive_like(lower: &str) -> bool {
    lower.ends_with(".zip")
        || lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tar.zst")
        || lower.ends_with(".7z")
        || lower.ends_with(".rar")
        || lower.ends_with(".gz")
        || lower.ends_with(".xz")
        || lower.ends_with(".bz2")
        || lower.ends_with(".zst")
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

fn sort_graveyard_entries(entries: &mut [GraveyardEntry], sort_mode: GraveyardSortMode) {
    match sort_mode {
        GraveyardSortMode::Score => {
            entries.sort_by(|left, right| {
                right
                    .score
                    .cmp(&left.score)
                    .then_with(|| right.size.cmp(&left.size))
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        GraveyardSortMode::Size => {
            entries.sort_by(|left, right| {
                right
                    .size
                    .cmp(&left.size)
                    .then_with(|| right.score.cmp(&left.score))
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        GraveyardSortMode::Age => {
            entries.sort_by(|left, right| {
                left.modified
                    .cmp(&right.modified)
                    .then_with(|| right.score.cmp(&left.score))
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        GraveyardSortMode::Name => {
            entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        }
    }
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

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!(
        "{}Cleanup candidates:{}",
        palette.report.heading, palette.reset,
    );

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();
}

fn graveyard_name_width(entries: &[GraveyardEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.display_name.len())
        .max()
        .unwrap_or(0)
        .max("name".len())
}

fn print_column_header(name_width: usize, palette: &AnsiPalette) {
    println!(
        "  {}{:>8}{}  {}{:<8}{}  {}{:>5}{}  {}{:<name_width$}{}  {}{}{}",
        palette.report.label,
        "size",
        palette.reset,
        palette.report.label,
        "kind",
        palette.reset,
        palette.report.label,
        "age",
        palette.reset,
        palette.report.label,
        "name",
        palette.reset,
        palette.report.label,
        "reason",
        palette.reset,
        name_width = name_width,
    );
}

fn print_graveyard_entry(entry: &GraveyardEntry, name_width: usize, palette: &AnsiPalette) {
    let age = human_age(entry.modified);

    println!(
        "  {}{:>8}{}  {}{:<8}{}  {}{:>5}{}  {}{:<name_width$}{}  {}{}{}",
        palette.report.size,
        human_size(entry.size,),
        palette.reset,
        palette.report.attention,
        entry.kind,
        palette.reset,
        palette.report.count,
        age,
        palette.reset,
        palette.report.name,
        entry.display_name,
        palette.reset,
        palette.report.path,
        entry.reason,
        palette.reset,
        name_width = name_width,
    );
}

fn print_summary(entries: &[GraveyardEntry], palette: &AnsiPalette) {
    let total_size = entries
        .iter()
        .map(|entry| entry.size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} candidates found",
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
}
