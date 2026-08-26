use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::format::human_age;
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct EmptyDirectory {
    path: String,
    depth: usize,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy)]
enum EmptyDirSortMode {
    Path,
    Age,
    Depth,
}

#[derive(Debug)]
struct EmptyDirOptions {
    path: PathBuf,
    sort_mode: EmptyDirSortMode,
    limit: Option<usize>,
    min_depth: Option<usize>,
}

pub fn handle_empty_dirs_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(empty_dirs_index) = arguments
        .iter()
        .position(|argument| argument == "--empty-dirs")
    else {
        return false;
    };

    let options = empty_dir_options(arguments, empty_dirs_index);

    print_empty_directories(&options, palette);

    true
}

fn empty_dir_options(arguments: &[String], empty_dirs_index: usize) -> EmptyDirOptions {
    let mut index = empty_dirs_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = EmptyDirSortMode::Path;

    let mut limit = None;

    let mut min_depth = None;

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

            "--min-depth" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --min-depth requires a number");

                    std::process::exit(2);
                }

                let Some(parsed_min_depth) = parse_usize_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid min-depth '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                min_depth = Some(parsed_min_depth);

                index += 2;
            }

            argument if argument.starts_with("--min-depth=") => {
                let min_depth_text = argument.trim_start_matches("--min-depth=");

                let Some(parsed_min_depth) = parse_usize_argument(min_depth_text) else {
                    eprintln!("noct: invalid min-depth '{}'", min_depth_text);

                    std::process::exit(2);
                };

                min_depth = Some(parsed_min_depth);

                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --empty-dirs --sort requires path, age, or depth");

                    std::process::exit(2);
                }

                sort_mode = parse_empty_dir_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_empty_dir_sort_mode(sort_text);

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

    EmptyDirOptions {
        path,
        sort_mode,
        limit,
        min_depth,
    }
}

fn parse_usize_argument(value: &str) -> Option<usize> {
    let number = value.trim().parse::<usize>().ok()?;

    if number == 0 {
        return None;
    }

    Some(number)
}

fn parse_empty_dir_sort_mode(value: &str) -> EmptyDirSortMode {
    match value {
        "path" | "name" => EmptyDirSortMode::Path,

        "age" | "old" => EmptyDirSortMode::Age,

        "depth" | "deep" => EmptyDirSortMode::Depth,

        _ => {
            eprintln!("noct: unknown --empty-dirs sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_empty_directories(options: &EmptyDirOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --empty-dirs requires a directory");

        std::process::exit(2);
    }

    let mut empty_dirs = Vec::new();

    collect_empty_directories(path, path, 0, &mut empty_dirs);

    empty_dirs.retain(|entry| entry_matches_options(entry, options));

    sort_empty_directories(&mut empty_dirs, options.sort_mode);

    if let Some(limit) = options.limit {
        empty_dirs.truncate(limit);
    }

    print_header(path, palette);

    if empty_dirs.is_empty() {
        println!(
            "  {}no empty directories found{}",
            palette.report.total, palette.reset,
        );

        println!();

        print_summary(&empty_dirs, palette);

        return;
    }

    print_column_header(palette);

    for entry in &empty_dirs {
        print_empty_directory(entry, palette);
    }

    println!();

    print_summary(&empty_dirs, palette);
}

fn collect_empty_directories(
    base_path: &Path,
    current_path: &Path,
    depth: usize,
    empty_dirs: &mut Vec<EmptyDirectory>,
) -> bool {
    let entries = match fs::read_dir(current_path) {
        Ok(entries) => entries,

        Err(_) => {
            return false;
        }
    };

    let mut saw_any_entry = false;

    let mut all_child_dirs_empty = true;

    for result in entries {
        let Ok(dir_entry) = result else {
            saw_any_entry = true;

            all_child_dirs_empty = false;

            continue;
        };

        saw_any_entry = true;

        let entry_path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                all_child_dirs_empty = false;

                continue;
            }
        };

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            let child_empty =
                collect_empty_directories(base_path, &entry_path, depth + 1, empty_dirs);

            if !child_empty {
                all_child_dirs_empty = false;
            }
        } else {
            all_child_dirs_empty = false;
        }
    }

    if !saw_any_entry {
        let relative_path = current_path
            .strip_prefix(base_path)
            .map(|path| {
                if path.as_os_str().is_empty() {
                    ".".to_string()
                } else {
                    path.to_string_lossy().to_string()
                }
            })
            .unwrap_or_else(|_| current_path.display().to_string());

        let modified = fs::symlink_metadata(current_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok());

        empty_dirs.push(EmptyDirectory {
            path: format!("{}/", relative_path),
            depth,
            modified,
        });

        return true;
    }

    all_child_dirs_empty
}

fn entry_matches_options(entry: &EmptyDirectory, options: &EmptyDirOptions) -> bool {
    if let Some(min_depth) = options.min_depth {
        if entry.depth < min_depth {
            return false;
        }
    }

    true
}

fn sort_empty_directories(entries: &mut [EmptyDirectory], sort_mode: EmptyDirSortMode) {
    match sort_mode {
        EmptyDirSortMode::Path => {
            entries.sort_by(|left, right| left.path.cmp(&right.path));
        }

        EmptyDirSortMode::Age => {
            entries.sort_by(|left, right| {
                left.modified
                    .cmp(&right.modified)
                    .then_with(|| left.path.cmp(&right.path))
            });
        }

        EmptyDirSortMode::Depth => {
            entries.sort_by(|left, right| {
                right
                    .depth
                    .cmp(&left.depth)
                    .then_with(|| left.path.cmp(&right.path))
            });
        }
    }
}

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!(
        "{}Empty directories:{}",
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

fn print_column_header(palette: &AnsiPalette) {
    println!(
        "  {}{:>6}{}  {}{:>5}{}  {}{}{}",
        palette.report.label,
        "age",
        palette.reset,
        palette.report.label,
        "depth",
        palette.reset,
        palette.report.label,
        "path",
        palette.reset,
    );
}

fn print_empty_directory(entry: &EmptyDirectory, palette: &AnsiPalette) {
    let age = human_age(entry.modified);

    println!(
        "  {}{:>6}{}  {}{:>5}{}  {}{}{}",
        palette.report.count,
        age,
        palette.reset,
        palette.report.size,
        entry.depth,
        palette.reset,
        palette.report.path,
        entry.path,
        palette.reset,
    );
}

fn print_summary(entries: &[EmptyDirectory], palette: &AnsiPalette) {
    let deepest = entries.iter().map(|entry| entry.depth).max().unwrap_or(0);

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} empty directories found",
        palette.report.total,
        entries.len(),
        palette.reset,
    );

    println!(
        "  {}deepest:{} {}{}{} levels",
        palette.report.label, palette.reset, palette.report.size, deepest, palette.reset,
    );
}
