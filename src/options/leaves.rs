use std::fs;
use std::path::{Path, PathBuf};

use crate::args::parse_size_argument;
use crate::format::human_size;
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct LeafDirectory {
    path: String,
    file_count: usize,
    total_size: u64,
}

#[derive(Debug, Clone, Copy)]
enum LeafSortMode {
    Size,
    Files,
    Path,
}

#[derive(Debug)]
struct LeafOptions {
    path: PathBuf,
    sort_mode: LeafSortMode,
    limit: Option<usize>,
    min_files: Option<usize>,
    min_size: Option<u64>,
}

pub fn handle_leaves_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(leaves_index) = arguments.iter().position(|argument| argument == "--leaves") else {
        return false;
    };

    let options = leaf_options(arguments, leaves_index);

    print_leaf_directories(&options, palette);

    true
}

fn leaf_options(arguments: &[String], leaves_index: usize) -> LeafOptions {
    let mut index = leaves_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = LeafSortMode::Size;

    let mut limit = None;

    let mut min_files = None;

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

            "--min-files" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --min-files requires a number");

                    std::process::exit(2);
                }

                let Some(parsed_min_files) = parse_usize_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid min-files '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                min_files = Some(parsed_min_files);

                index += 2;
            }

            argument if argument.starts_with("--min-files=") => {
                let min_files_text = argument.trim_start_matches("--min-files=");

                let Some(parsed_min_files) = parse_usize_argument(min_files_text) else {
                    eprintln!("noct: invalid min-files '{}'", min_files_text);

                    std::process::exit(2);
                };

                min_files = Some(parsed_min_files);

                index += 1;
            }

            "--min-size" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --min-size requires a size, for example 100K or 2M");

                    std::process::exit(2);
                }

                let Some(parsed_min_size) = parse_size_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid min-size '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                min_size = Some(parsed_min_size);

                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                let min_size_text = argument.trim_start_matches("--min-size=");

                let Some(parsed_min_size) = parse_size_argument(min_size_text) else {
                    eprintln!("noct: invalid min-size '{}'", min_size_text);

                    std::process::exit(2);
                };

                min_size = Some(parsed_min_size);

                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --leaves --sort requires size, files, or path");

                    std::process::exit(2);
                }

                sort_mode = parse_leaf_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_leaf_sort_mode(sort_text);

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

    LeafOptions {
        path,
        sort_mode,
        limit,
        min_files,
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

fn parse_leaf_sort_mode(value: &str) -> LeafSortMode {
    match value {
        "size" => LeafSortMode::Size,

        "files" | "count" => LeafSortMode::Files,

        "path" | "name" => LeafSortMode::Path,

        _ => {
            eprintln!("noct: unknown --leaves sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_leaf_directories(options: &LeafOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --leaves requires a directory");

        std::process::exit(2);
    }

    let mut leaves = Vec::new();

    collect_leaf_directories(path, path, &mut leaves);

    leaves.retain(|leaf| leaf_matches_options(leaf, options));

    sort_leaf_directories(&mut leaves, options.sort_mode);

    if let Some(limit) = options.limit {
        leaves.truncate(limit);
    }

    print_header(path, palette);

    if leaves.is_empty() {
        println!(
            "  {}no leaf directories found{}",
            palette.report.muted, palette.reset,
        );

        println!();

        print_summary(&leaves, palette);

        return;
    }

    print_column_header(palette);

    for leaf in &leaves {
        print_leaf_directory(leaf, palette);
    }

    println!();

    print_summary(&leaves, palette);
}

fn collect_leaf_directories(
    base_path: &Path,
    current_path: &Path,
    leaves: &mut Vec<LeafDirectory>,
) {
    let entries = match fs::read_dir(current_path) {
        Ok(entries) => entries,

        Err(_) => {
            return;
        }
    };

    let mut file_count = 0usize;

    let mut directory_count = 0usize;

    let mut total_size = 0u64;

    let mut child_dirs = Vec::new();

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

        let file_type = metadata.file_type();

        if metadata.is_dir() && !file_type.is_symlink() {
            directory_count += 1;

            child_dirs.push(entry_path);

            continue;
        }

        if metadata.is_file() {
            file_count += 1;

            total_size = total_size.saturating_add(metadata.len());
        }
    }

    if directory_count == 0 && file_count > 0 {
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

        leaves.push(LeafDirectory {
            path: format!("{}/", relative_path),
            file_count,
            total_size,
        });
    }

    for child_dir in child_dirs {
        collect_leaf_directories(base_path, &child_dir, leaves);
    }
}

fn leaf_matches_options(leaf: &LeafDirectory, options: &LeafOptions) -> bool {
    if let Some(min_files) = options.min_files {
        if leaf.file_count < min_files {
            return false;
        }
    }

    if let Some(min_size) = options.min_size {
        if leaf.total_size < min_size {
            return false;
        }
    }

    true
}

fn sort_leaf_directories(leaves: &mut [LeafDirectory], sort_mode: LeafSortMode) {
    match sort_mode {
        LeafSortMode::Size => {
            leaves.sort_by(|left, right| {
                right
                    .total_size
                    .cmp(&left.total_size)
                    .then_with(|| right.file_count.cmp(&left.file_count))
                    .then_with(|| left.path.cmp(&right.path))
            });
        }

        LeafSortMode::Files => {
            leaves.sort_by(|left, right| {
                right
                    .file_count
                    .cmp(&left.file_count)
                    .then_with(|| right.total_size.cmp(&left.total_size))
                    .then_with(|| left.path.cmp(&right.path))
            });
        }

        LeafSortMode::Path => {
            leaves.sort_by(|left, right| left.path.cmp(&right.path));
        }
    }
}

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!(
        "{}Leaf directories:{}",
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
        "  {}{:>6}{}  {}{:>8}{}  {}{}{}",
        palette.report.label,
        "files",
        palette.reset,
        palette.report.label,
        "size",
        palette.reset,
        palette.report.label,
        "path",
        palette.reset,
    );
}

fn print_leaf_directory(leaf: &LeafDirectory, palette: &AnsiPalette) {
    let size = human_size(leaf.total_size);

    println!(
        "  {}{:>6}{}  {}{:>8}{}  {}{}{}",
        palette.report.count,
        leaf.file_count,
        palette.reset,
        palette.report.size,
        size,
        palette.reset,
        palette.report.path,
        leaf.path,
        palette.reset,
    );
}

fn print_summary(leaves: &[LeafDirectory], palette: &AnsiPalette) {
    let total_files = leaves.iter().map(|leaf| leaf.file_count).sum::<usize>();

    let total_size = leaves
        .iter()
        .map(|leaf| leaf.total_size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} leaf directories found",
        palette.report.total,
        leaves.len(),
        palette.reset,
    );

    println!(
        "  {}{}{} files inside leaves",
        palette.report.count, total_files, palette.reset,
    );

    println!(
        "  {}{}{} listed",
        palette.report.size,
        human_size(total_size,),
        palette.reset,
    );
}
