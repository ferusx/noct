use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::format::{human_size, user_name};
use crate::sizes::{MountTable, SizeInfo};
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct OwnerStats {
    uid: u32,
    owner: String,
    entries: usize,
    files: usize,
    dirs: usize,
    links: usize,
    special: usize,
    allocated_size: u64,
    apparent_size: u64,
    has_virtual_size: bool,
}

#[derive(Debug, Clone, Copy)]
enum OwnerSortMode {
    Size,
    Entries,
    Owner,
}

#[derive(Debug)]
struct OwnerOptions {
    path: PathBuf,
    sort_mode: OwnerSortMode,
}

pub fn handle_owners_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(owners_index) = arguments.iter().position(|argument| argument == "--owners") else {
        return false;
    };

    let options = owner_options(arguments, owners_index);

    print_owner_profile(&options, palette);

    true
}

fn owner_options(arguments: &[String], owners_index: usize) -> OwnerOptions {
    let mut index = owners_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = OwnerSortMode::Size;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --owners --sort requires size, entries, or owner");

                    std::process::exit(2);
                }

                sort_mode = parse_owner_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_owner_sort_mode(sort_text);

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

    OwnerOptions { path, sort_mode }
}

fn parse_owner_sort_mode(value: &str) -> OwnerSortMode {
    match value {
        "size" => OwnerSortMode::Size,

        "entries" | "count" => OwnerSortMode::Entries,

        "owner" | "name" | "user" => OwnerSortMode::Owner,

        _ => {
            eprintln!("noct: unknown --owners sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_owner_profile(options: &OwnerOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --owners requires a directory");

        std::process::exit(2);
    }

    let mut by_owner: BTreeMap<u32, OwnerStats> = BTreeMap::new();

    let mut total_read_errors = 0usize;

    let mount_table = MountTable::load();

    scan_owner_tree(path, &mount_table, &mut by_owner, &mut total_read_errors);

    let mut stats: Vec<OwnerStats> = by_owner.into_values().collect();

    sort_owner_stats(&mut stats, options.sort_mode);

    print_owner_header(path, palette);

    if stats.is_empty() {
        println!(
            "  {}no entries found{}",
            palette.report.muted, palette.reset,
        );

        return;
    }

    let owner_width = stats
        .iter()
        .map(|entry| entry.owner.len())
        .max()
        .unwrap_or(5)
        .max("owner".len());

    let entry_width = stats
        .iter()
        .map(|entry| entry.entries.to_string().len())
        .max()
        .unwrap_or(7)
        .max("entries".len());

    let files_width = stats
        .iter()
        .map(|entry| entry.files.to_string().len())
        .max()
        .unwrap_or(5)
        .max("files".len());

    let dirs_width = stats
        .iter()
        .map(|entry| entry.dirs.to_string().len())
        .max()
        .unwrap_or(4)
        .max("dirs".len());

    let links_width = stats
        .iter()
        .map(|entry| entry.links.to_string().len())
        .max()
        .unwrap_or(5)
        .max("links".len());

    let special_width = stats
        .iter()
        .map(|entry| entry.special.to_string().len())
        .max()
        .unwrap_or(4)
        .max("spec".len());

    let size_width = stats
        .iter()
        .map(|entry| human_size(entry.allocated_size).len())
        .max()
        .unwrap_or(4)
        .max("size".len());

    print_column_header(
        owner_width,
        entry_width,
        files_width,
        dirs_width,
        links_width,
        special_width,
        size_width,
        palette,
    );

    for entry in &stats {
        print_owner_entry(
            entry,
            owner_width,
            entry_width,
            files_width,
            dirs_width,
            links_width,
            special_width,
            size_width,
            palette,
        );
    }

    println!();

    print_owner_total(&stats, total_read_errors, palette);
}
fn scan_owner_tree(
    path: &Path,
    mount_table: &MountTable,
    by_owner: &mut BTreeMap<u32, OwnerStats>,
    total_read_errors: &mut usize,
) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(_) => {
            *total_read_errors += 1;
            return;
        }
    };

    for result in entries {
        let Ok(dir_entry) = result else {
            *total_read_errors += 1;
            continue;
        };

        let entry_path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                *total_read_errors += 1;
                continue;
            }
        };

        add_owner_entry(&entry_path, &metadata, mount_table, by_owner);

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            scan_owner_tree(&entry_path, mount_table, by_owner, total_read_errors);
        }
    }
}

fn add_owner_entry(
    path: &Path,
    metadata: &fs::Metadata,
    mount_table: &MountTable,
    by_owner: &mut BTreeMap<u32, OwnerStats>,
) {
    let uid = metadata.uid();

    let stats = by_owner.entry(uid).or_insert_with(|| OwnerStats {
        uid,
        owner: user_name(uid),
        entries: 0,
        files: 0,
        dirs: 0,
        links: 0,
        special: 0,
        allocated_size: 0,
        apparent_size: 0,
        has_virtual_size: false,
    });

    stats.entries += 1;

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

    let size_info = SizeInfo::from_metadata(metadata);

    stats.apparent_size = stats.apparent_size.saturating_add(size_info.apparent_bytes);

    stats.allocated_size = stats
        .allocated_size
        .saturating_add(size_info.allocated_bytes);

    let _filesystem = mount_table.filesystem_for_path(path);
}

fn sort_owner_stats(stats: &mut [OwnerStats], sort_mode: OwnerSortMode) {
    match sort_mode {
        OwnerSortMode::Size => {
            stats.sort_by(|left, right| {
                right
                    .allocated_size
                    .cmp(&left.allocated_size)
                    .then_with(|| right.entries.cmp(&left.entries))
                    .then_with(|| left.owner.cmp(&right.owner))
            });
        }

        OwnerSortMode::Entries => {
            stats.sort_by(|left, right| {
                right
                    .entries
                    .cmp(&left.entries)
                    .then_with(|| right.allocated_size.cmp(&left.allocated_size))
                    .then_with(|| left.owner.cmp(&right.owner))
            });
        }

        OwnerSortMode::Owner => {
            stats.sort_by(|left, right| left.owner.cmp(&right.owner));
        }
    }
}

fn print_owner_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Owner profile:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();
}

fn print_column_header(
    owner_width: usize,
    entry_width: usize,
    files_width: usize,
    dirs_width: usize,
    links_width: usize,
    special_width: usize,
    size_width: usize,
    palette: &AnsiPalette,
) {
    println!(
        "  {}{:<owner_width$}{}  {}{:>entry_width$}{}  {}{:>files_width$}{}  {}{:>dirs_width$}{}  {}{:>links_width$}{}  {}{:>special_width$}{}  {}{:>size_width$}{}",
        palette.report.label,
        "owner",
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
        "size",
        palette.reset,
        owner_width = owner_width,
        entry_width = entry_width,
        files_width = files_width,
        dirs_width = dirs_width,
        links_width = links_width,
        special_width = special_width,
        size_width = size_width,
    );
}

fn print_owner_entry(
    entry: &OwnerStats,
    owner_width: usize,
    entry_width: usize,
    files_width: usize,
    dirs_width: usize,
    links_width: usize,
    special_width: usize,
    size_width: usize,
    palette: &AnsiPalette,
) {
    let size = human_size(entry.allocated_size);

    let owner_color = if entry.owner == entry.uid.to_string() {
        &palette.report.attention
    } else {
        &palette.report.package
    };

    println!(
        "  {}{:<owner_width$}{}  {}{:>entry_width$}{}  {}{:>files_width$}{}  {}{:>dirs_width$}{}  {}{:>links_width$}{}  {}{:>special_width$}{}  {}{:>size_width$}{}",
        owner_color,
        entry.owner,
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
        palette.report.size,
        size,
        palette.reset,
        owner_width = owner_width,
        entry_width = entry_width,
        files_width = files_width,
        dirs_width = dirs_width,
        links_width = links_width,
        special_width = special_width,
        size_width = size_width,
    );
}

fn print_owner_total(stats: &[OwnerStats], total_read_errors: usize, palette: &AnsiPalette) {
    let total_entries = stats.iter().map(|entry| entry.entries).sum::<usize>();

    let total_allocated = stats
        .iter()
        .map(|entry| entry.allocated_size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    let total_apparent = stats
        .iter()
        .map(|entry| entry.apparent_size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    let has_virtual_size = stats.iter().any(|entry| entry.has_virtual_size);

    println!(
        "{}Total:{} {} entries, {} owners, {} allocated",
        palette.report.total,
        palette.reset,
        total_entries,
        stats.len(),
        human_size(total_allocated),
    );

    if total_apparent != total_allocated {
        println!(
            "{}Apparent:{} {}",
            palette.report.label,
            palette.reset,
            human_size(total_apparent),
        );
    }

    if has_virtual_size {
        println!(
            "{}Note:{} apparent size includes virtual filesystem data and does not represent disk usage.",
            palette.report.muted, palette.reset,
        );
    }

    if total_read_errors > 0 {
        println!(
            "{}Read errors:{} {}{}{}",
            palette.report.attention,
            palette.reset,
            palette.report.attention,
            total_read_errors,
            palette.reset,
        );
    }
}
