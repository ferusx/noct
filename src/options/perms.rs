use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct PermissionStats {
    mode: u32,
    entries: usize,
    files: usize,
    dirs: usize,
    links: usize,
    special: usize,
    attention: usize,
}

#[derive(Debug, Clone, Copy)]
enum PermissionSortMode {
    Count,
    Mode,
    Attention,
}

#[derive(Debug)]
struct PermissionOptions {
    path: PathBuf,
    sort_mode: PermissionSortMode,
}

pub fn handle_perms_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(perms_index) = arguments.iter().position(|argument| argument == "--perms") else {
        return false;
    };

    let options = permission_options(arguments, perms_index);

    print_permission_profile(&options, palette);

    true
}

fn permission_options(arguments: &[String], perms_index: usize) -> PermissionOptions {
    let mut index = perms_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = PermissionSortMode::Count;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --perms --sort requires count, mode, or attention");

                    std::process::exit(2);
                }

                sort_mode = parse_permission_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_permission_sort_mode(sort_text);

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

    PermissionOptions { path, sort_mode }
}

fn parse_permission_sort_mode(value: &str) -> PermissionSortMode {
    match value {
        "count" | "entries" => PermissionSortMode::Count,

        "mode" | "permission" | "permissions" => PermissionSortMode::Mode,

        "attention" | "danger" => PermissionSortMode::Attention,

        _ => {
            eprintln!("noct: unknown --perms sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_permission_profile(options: &PermissionOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --perms requires a directory");

        std::process::exit(2);
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    let mut by_mode: BTreeMap<u32, PermissionStats> = BTreeMap::new();

    let mut total_entries = 0usize;

    let mut total_attention = 0usize;

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

        let mode = metadata.permissions().mode() & 0o7777;

        let stats = by_mode.entry(mode).or_insert_with(|| PermissionStats {
            mode,
            entries: 0,
            files: 0,
            dirs: 0,
            links: 0,
            special: 0,
            attention: 0,
        });

        stats.entries += 1;
        total_entries += 1;

        let file_type = metadata.file_type();

        if metadata.is_dir() {
            stats.dirs += 1;
        } else if file_type.is_symlink() {
            stats.links += 1;
        } else if metadata.is_file() {
            stats.files += 1;
        } else if file_type.is_socket()
            || file_type.is_fifo()
            || file_type.is_block_device()
            || file_type.is_char_device()
        {
            stats.special += 1;
        } else {
            stats.special += 1;
        }

        if is_attention_mode(mode) {
            stats.attention += 1;
            total_attention += 1;
        }
    }

    let mut stats: Vec<PermissionStats> = by_mode.into_values().collect();

    sort_permission_stats(&mut stats, options.sort_mode);

    print_permission_header(path, palette);

    if stats.is_empty() {
        println!(
            "  {}no entries found{}",
            palette.report.muted, palette.reset,
        );

        return;
    }

    print_column_header(palette);

    for entry in &stats {
        print_permission_entry(entry, palette);
    }

    println!();

    print_permission_total(total_entries, total_attention, palette);
}

fn print_permission_header(path: &Path, palette: &AnsiPalette) {
    println!(
        "{}Permission profile:{}",
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
        "  {}{:<6}{}  {}{:>7}{}  {}{:>5}{}  {}{:>5}{}  {}{:>5}{}  {}{:>5}{}  {}{}{}",
        palette.report.label,
        "mode",
        palette.reset,
        palette.report.label,
        "entries",
        palette.reset,
        palette.report.label,
        "files",
        palette.reset,
        palette.report.label,
        "dirs",
        palette.reset,
        palette.report.label,
        "links",
        palette.reset,
        palette.report.label,
        "spec",
        palette.reset,
        palette.report.label,
        "state",
        palette.reset,
    );
}

fn print_permission_entry(entry: &PermissionStats, palette: &AnsiPalette) {
    let mode = format!("{:04o}", entry.mode,);

    let state = if entry.attention > 0 {
        "attention"
    } else {
        "normal"
    };

    let state_color = if entry.attention > 0 {
        &palette.report.attention
    } else {
        &palette.report.muted
    };

    println!(
        "  {}{:<6}{}  {}{:>7}{}  {}{:>5}{}  {}{:>5}{}  {}{:>5}{}  {}{:>5}{}  {}{}{}",
        palette.report.package,
        mode,
        palette.reset,
        palette.report.count,
        entry.entries,
        palette.reset,
        palette.report.count,
        entry.files,
        palette.reset,
        palette.report.count,
        entry.dirs,
        palette.reset,
        palette.report.count,
        entry.links,
        palette.reset,
        palette.report.count,
        entry.special,
        palette.reset,
        state_color,
        state,
        palette.reset,
    );
}

fn print_permission_total(total_entries: usize, total_attention: usize, palette: &AnsiPalette) {
    println!(
        "{}Total:{} {}{}{} entries, {}{}{} attention",
        palette.report.total,
        palette.reset,
        palette.report.count,
        total_entries,
        palette.reset,
        if total_attention > 0 {
            &palette.report.attention
        } else {
            &palette.report.count
        },
        total_attention,
        palette.reset,
    );
}

fn sort_permission_stats(stats: &mut [PermissionStats], sort_mode: PermissionSortMode) {
    match sort_mode {
        PermissionSortMode::Count => {
            stats.sort_by(|left, right| {
                right
                    .entries
                    .cmp(&left.entries)
                    .then_with(|| left.mode.cmp(&right.mode))
            });
        }

        PermissionSortMode::Mode => {
            stats.sort_by(|left, right| left.mode.cmp(&right.mode));
        }

        PermissionSortMode::Attention => {
            stats.sort_by(|left, right| {
                right
                    .attention
                    .cmp(&left.attention)
                    .then_with(|| right.entries.cmp(&left.entries))
                    .then_with(|| left.mode.cmp(&right.mode))
            });
        }
    }
}

fn is_attention_mode(mode: u32) -> bool {
    mode & 0o002 != 0 || mode & 0o4000 != 0 || mode & 0o2000 != 0 || mode & 0o777 == 0o777
}
