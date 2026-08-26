use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::entry::detect_state;
use crate::format::{human_age, human_size};
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct ProfileEntry {
    display_name: String,
    allocated_size: u64,
    apparent_size: u64,
    modified: Option<SystemTime>,
    state: String,
    is_file: bool,
    is_dir: bool,
    is_symlink: bool,
    is_special: bool,
    is_executable: bool,
    is_hidden: bool,
    had_error: bool,
}

#[derive(Debug)]
struct DirectoryProfile {
    path: PathBuf,
    entries: Vec<ProfileEntry>,
    read_errors: usize,
}

pub fn handle_profile_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(profile_index) = arguments
        .iter()
        .position(|argument| argument == "--profile")
    else {
        return false;
    };

    let path = profile_search_path(arguments, profile_index);

    print_directory_profile(&path, palette);

    true
}

fn profile_search_path(arguments: &[String], profile_index: usize) -> PathBuf {
    let mut index = profile_index + 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            argument if argument.starts_with('-') => {
                index += 1;
            }

            argument => {
                return PathBuf::from(argument);
            }
        }
    }

    PathBuf::from(".")
}

fn print_directory_profile(path: &Path, palette: &AnsiPalette) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --profile requires a directory");

        std::process::exit(2);
    }

    let profile = build_directory_profile(path);

    print_profile_header(palette);

    print_profile_line(
        "Path",
        profile.path.display().to_string(),
        ProfileValueKind::Path,
        palette,
    );

    let entries = &profile.entries;

    let file_count = entries.iter().filter(|entry| entry.is_file).count();

    let dir_count = entries.iter().filter(|entry| entry.is_dir).count();

    let link_count = entries.iter().filter(|entry| entry.is_symlink).count();

    let special_count = entries.iter().filter(|entry| entry.is_special).count();

    let executable_count = entries.iter().filter(|entry| entry.is_executable).count();

    let hidden_count = entries.iter().filter(|entry| entry.is_hidden).count();

    let attention_count = entries
        .iter()
        .filter(|entry| entry.state != "normal" || entry.had_error)
        .count();

    let total_allocated = entries
        .iter()
        .map(|entry| entry.allocated_size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    let total_apparent = entries
        .iter()
        .map(|entry| entry.apparent_size)
        .fold(0u64, |sum, size| sum.saturating_add(size));

    print_profile_line("Entries", entries.len(), ProfileValueKind::Normal, palette);

    print_profile_line("Files", file_count, ProfileValueKind::Normal, palette);

    print_profile_line("Directories", dir_count, ProfileValueKind::Normal, palette);

    print_profile_line("Symlinks", link_count, ProfileValueKind::Normal, palette);

    print_profile_line("Special", special_count, ProfileValueKind::Normal, palette);

    print_profile_line(
        "Executables",
        executable_count,
        ProfileValueKind::Normal,
        palette,
    );

    print_profile_line(
        "Hidden entries",
        hidden_count,
        ProfileValueKind::Normal,
        palette,
    );

    print_profile_line(
        "Total allocated",
        human_size(total_allocated),
        ProfileValueKind::Size,
        palette,
    );

    if total_apparent != total_allocated {
        print_profile_line(
            "Total apparent",
            human_size(total_apparent),
            ProfileValueKind::Size,
            palette,
        );
    }

    if let Some(largest) = largest_entry(entries) {
        print_profile_line(
            "Largest entry",
            format!(
                "{} {} [apparent]",
                largest.display_name,
                human_size(largest.apparent_size),
            ),
            ProfileValueKind::Name,
            palette,
        );
    }

    if let Some(newest) = newest_entry(entries) {
        print_profile_line(
            "Newest entry",
            format!("{} {}", newest.display_name, human_age(newest.modified,)),
            ProfileValueKind::Name,
            palette,
        );
    }

    if let Some(oldest) = oldest_entry(entries) {
        print_profile_line(
            "Oldest entry",
            format!("{} {}", oldest.display_name, human_age(oldest.modified,)),
            ProfileValueKind::Name,
            palette,
        );
    }

    print_profile_line(
        "Attention states",
        attention_count,
        if attention_count > 0 {
            ProfileValueKind::Attention
        } else {
            ProfileValueKind::Good
        },
        palette,
    );

    if profile.read_errors > 0 {
        print_profile_line(
            "Read errors",
            profile.read_errors,
            ProfileValueKind::Attention,
            palette,
        );
    }
}

fn build_directory_profile(path: &Path) -> DirectoryProfile {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut profile_entries = Vec::new();

    let mut read_errors = 0usize;

    for result in entries {
        let Ok(dir_entry) = result else {
            read_errors += 1;
            continue;
        };

        let entry_path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                read_errors += 1;

                profile_entries.push(ProfileEntry {
                    display_name: name.clone(),
                    allocated_size: 0,
                    apparent_size: 0,
                    modified: None,
                    state: "unreadable".to_string(),
                    is_file: false,
                    is_dir: false,
                    is_symlink: false,
                    is_special: false,
                    is_executable: false,
                    is_hidden: false,
                    had_error: true,
                });

                continue;
            }
        };

        let file_type = metadata.file_type();

        let is_dir = metadata.is_dir();

        let is_symlink = file_type.is_symlink();

        let is_file = metadata.is_file();

        let is_special = file_type.is_socket()
            || file_type.is_fifo()
            || file_type.is_block_device()
            || file_type.is_char_device();

        let is_executable = is_file && metadata.permissions().mode() & 0o111 != 0;

        let is_hidden = name.starts_with('.');

        let display_name = display_name_for(&name, is_dir, is_symlink, is_executable);

        let state = detect_state(&entry_path, &metadata);

        let (allocated_size, apparent_size, had_error) = profile_entry_size(&entry_path, &metadata);

        if had_error {
            read_errors += 1;
        }

        profile_entries.push(ProfileEntry {
            display_name,
            allocated_size,
            apparent_size,
            modified: metadata.modified().ok(),
            state,
            is_file,
            is_dir,
            is_symlink,
            is_special,
            is_executable,
            is_hidden,
            had_error,
        });
    }

    DirectoryProfile {
        path: path.to_path_buf(),
        entries: profile_entries,
        read_errors,
    }
}

fn profile_entry_size(path: &Path, metadata: &fs::Metadata) -> (u64, u64, bool) {
    if metadata.is_file() {
        let allocated = metadata.blocks().saturating_mul(512);
        let apparent = metadata.len();

        return (allocated, apparent, false);
    }

    if !metadata.is_dir() {
        let allocated = metadata.blocks().saturating_mul(512);
        let apparent = metadata.len();

        return (allocated, apparent, false);
    }

    directory_size(path)
}

fn directory_size(path: &Path) -> (u64, u64, bool) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(_) => {
            return (0, 0, true);
        }
    };

    let mut allocated_total = 0u64;
    let mut apparent_total = 0u64;
    let mut had_error = false;

    for result in entries {
        let Ok(dir_entry) = result else {
            had_error = true;
            continue;
        };

        let entry_path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                had_error = true;
                continue;
            }
        };

        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            let (allocated, apparent, child_had_error) = directory_size(&entry_path);

            allocated_total = allocated_total.saturating_add(allocated);
            apparent_total = apparent_total.saturating_add(apparent);

            if child_had_error {
                had_error = true;
            }
        } else {
            allocated_total = allocated_total.saturating_add(metadata.blocks().saturating_mul(512));

            apparent_total = apparent_total.saturating_add(metadata.len());
        }
    }

    (allocated_total, apparent_total, had_error)
}

fn largest_entry(entries: &[ProfileEntry]) -> Option<&ProfileEntry> {
    entries
        .iter()
        .max_by(|left, right| left.apparent_size.cmp(&right.apparent_size))
}
fn newest_entry(entries: &[ProfileEntry]) -> Option<&ProfileEntry> {
    entries
        .iter()
        .filter(|entry| entry.modified.is_some())
        .max_by(|left, right| left.modified.cmp(&right.modified))
}

fn oldest_entry(entries: &[ProfileEntry]) -> Option<&ProfileEntry> {
    entries
        .iter()
        .filter(|entry| entry.modified.is_some())
        .min_by(|left, right| left.modified.cmp(&right.modified))
}

fn display_name_for(name: &str, is_dir: bool, is_symlink: bool, is_executable: bool) -> String {
    let suffix = if is_dir {
        "/"
    } else if is_symlink {
        "@"
    } else if is_executable {
        "*"
    } else {
        ""
    };

    format!("{}{}", name, suffix)
}

#[derive(Debug, Clone, Copy)]
enum ProfileValueKind {
    Normal,
    Path,
    Size,
    Name,
    Attention,
    Good,
}

fn print_profile_header(palette: &AnsiPalette) {
    println!(
        "{}Directory profile:{}",
        palette.report.heading, palette.reset,
    );

    println!();
}

fn print_profile_line(
    label: &str,
    value: impl std::fmt::Display,
    kind: ProfileValueKind,
    palette: &AnsiPalette,
) {
    let value_color = match kind {
        ProfileValueKind::Normal => &palette.report.value,

        ProfileValueKind::Path => &palette.report.path,

        ProfileValueKind::Size => &palette.report.size,

        ProfileValueKind::Name => &palette.report.name,

        ProfileValueKind::Attention => &palette.report.attention,

        ProfileValueKind::Good => &palette.report.total,
    };

    println!(
        "  {}{:<18}{} {}{}{}",
        palette.report.label,
        format!("{}:", label,),
        palette.reset,
        value_color,
        value,
        palette.reset,
    );
}
