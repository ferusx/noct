use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::format::human_size;
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct ExtensionStats {
    extension: String,
    count: usize,
    size: u64,
}

#[derive(Debug, Clone, Copy)]
enum ExtensionSortMode {
    Size,
    Count,
    Name,
}

#[derive(Debug)]
struct ExtensionOptions {
    path: PathBuf,
    sort_mode: ExtensionSortMode,
}

pub fn handle_exts_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(exts_index) = arguments.iter().position(|argument| argument == "--exts") else {
        return false;
    };

    let options = extension_options(arguments, exts_index);

    print_extension_profile(&options, palette);

    true
}

fn extension_options(arguments: &[String], exts_index: usize) -> ExtensionOptions {
    let mut index = exts_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = ExtensionSortMode::Size;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --exts --sort requires size, count, or name");

                    std::process::exit(2);
                }

                sort_mode = parse_extension_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_extension_sort_mode(sort_text);

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

    ExtensionOptions { path, sort_mode }
}

fn parse_extension_sort_mode(value: &str) -> ExtensionSortMode {
    match value {
        "size" => ExtensionSortMode::Size,

        "count" | "files" => ExtensionSortMode::Count,

        "name" | "extension" | "ext" => ExtensionSortMode::Name,

        _ => {
            eprintln!("noct: unknown --exts sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_extension_profile(options: &ExtensionOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --exts requires a directory");

        std::process::exit(2);
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    let mut by_extension: BTreeMap<String, ExtensionStats> = BTreeMap::new();

    let mut total_files = 0usize;

    let mut total_size = 0u64;

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

        if !metadata.is_file() {
            continue;
        }

        let extension = extension_label(&entry_path);

        let size = metadata.len();

        let stats = by_extension
            .entry(extension.clone())
            .or_insert_with(|| ExtensionStats {
                extension,
                count: 0,
                size: 0,
            });

        stats.count += 1;

        stats.size = stats.size.saturating_add(size);

        total_files += 1;

        total_size = total_size.saturating_add(size);
    }

    let mut stats: Vec<ExtensionStats> = by_extension.into_values().collect();

    sort_extension_stats(&mut stats, options.sort_mode);

    print_extension_header(path, palette);

    if stats.is_empty() {
        println!(
            "  {}no regular files found{}",
            palette.report.muted, palette.reset,
        );

        return;
    }

    let extension_width = stats
        .iter()
        .map(|entry| entry.extension.len())
        .max()
        .unwrap_or(3)
        .max("ext".len());

    let count_width = stats
        .iter()
        .map(|entry| entry.count.to_string().len())
        .max()
        .unwrap_or(5)
        .max("files".len());

    let size_width = stats
        .iter()
        .map(|entry| human_size(entry.size).len())
        .max()
        .unwrap_or(4)
        .max("size".len());

    print_column_header(extension_width, count_width, size_width, palette);

    for entry in &stats {
        print_extension_entry(entry, extension_width, count_width, size_width, palette);
    }

    println!();

    print_extension_total(total_files, total_size, palette);
}

fn print_extension_header(path: &Path, palette: &AnsiPalette) {
    println!(
        "{}Extension profile:{}",
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

fn print_column_header(
    extension_width: usize,
    count_width: usize,
    size_width: usize,
    palette: &AnsiPalette,
) {
    println!(
        "  {}{:<extension_width$}{}  {}{:>count_width$}{}  {}{:>size_width$}{}",
        palette.report.label,
        "ext",
        palette.reset,
        palette.report.label,
        "files",
        palette.reset,
        palette.report.label,
        "size",
        palette.reset,
        extension_width = extension_width,
        count_width = count_width,
        size_width = size_width,
    );
}

fn print_extension_entry(
    entry: &ExtensionStats,
    extension_width: usize,
    count_width: usize,
    size_width: usize,
    palette: &AnsiPalette,
) {
    let size = human_size(entry.size);

    println!(
        "  {}{:<extension_width$}{}  {}{:>count_width$}{}  {}{:>size_width$}{}",
        palette.report.package,
        entry.extension,
        palette.reset,
        palette.report.count,
        entry.count,
        palette.reset,
        palette.report.size,
        size,
        palette.reset,
        extension_width = extension_width,
        count_width = count_width,
        size_width = size_width,
    );
}

fn print_extension_total(total_files: usize, total_size: u64, palette: &AnsiPalette) {
    println!(
        "{}Total:{} {}{}{} files, {}{}{}",
        palette.report.total,
        palette.reset,
        palette.report.count,
        total_files,
        palette.reset,
        palette.report.size,
        human_size(total_size,),
        palette.reset,
    );
}

fn sort_extension_stats(stats: &mut [ExtensionStats], sort_mode: ExtensionSortMode) {
    match sort_mode {
        ExtensionSortMode::Size => {
            stats.sort_by(|left, right| {
                right
                    .size
                    .cmp(&left.size)
                    .then_with(|| left.extension.cmp(&right.extension))
            });
        }

        ExtensionSortMode::Count => {
            stats.sort_by(|left, right| {
                right
                    .count
                    .cmp(&left.count)
                    .then_with(|| right.size.cmp(&left.size))
                    .then_with(|| left.extension.cmp(&right.extension))
            });
        }

        ExtensionSortMode::Name => {
            stats.sort_by(|left, right| left.extension.cmp(&right.extension));
        }
    }
}

fn extension_label(path: &Path) -> String {
    let Some(file_name) = path.file_name() else {
        return "none".to_string();
    };

    let file_name = file_name.to_string_lossy().to_lowercase();

    let compound_extensions = [
        ".tar.gz",
        ".tar.xz",
        ".tar.bz2",
        ".tar.zst",
        ".tar.lz",
        ".tar.lzma",
        ".tar.br",
    ];

    for extension in compound_extensions {
        if file_name.ends_with(extension) {
            return extension.trim_start_matches('.').to_string();
        }
    }

    std::path::Path::new(file_name.as_str())
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "none".to_string())
}
