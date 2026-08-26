use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::format::human_size;
use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct MountEntry {
    path: String,
    dev: u64,
    used: Option<u64>,
    available: Option<u64>,
    total: Option<u64>,
    percent_used: Option<u64>,
    is_boundary: bool,
}

#[derive(Debug)]
struct MountOptions {
    path: PathBuf,
    all: bool,
}

pub fn handle_mounts_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(mounts_index) = arguments.iter().position(|argument| argument == "--mounts") else {
        return false;
    };

    let options = mount_options(arguments, mounts_index);

    print_mount_scan(&options, palette);

    true
}

fn mount_options(arguments: &[String], mounts_index: usize) -> MountOptions {
    let mut index = mounts_index + 1;

    let mut path = PathBuf::from(".");

    let mut all = false;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--all" => {
                all = true;

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

    MountOptions { path, all }
}

fn print_mount_scan(options: &MountOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --mounts requires a directory");

        std::process::exit(2);
    }

    let entries = collect_mount_entries(path, options.all);

    print_header(path, palette);

    if entries.is_empty() {
        println!(
            "  {}no filesystem boundaries found{}",
            palette.report.muted, palette.reset,
        );

        println!();

        print_summary(&entries, palette);

        return;
    }

    print_column_header(palette);

    for entry in &entries {
        print_mount_entry(entry, palette);
    }

    println!();

    print_summary(&entries, palette);
}

fn collect_mount_entries(path: &Path, all: bool) -> Vec<MountEntry> {
    let base_metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,

        Err(_) => {
            return Vec::new();
        }
    };

    let base_dev = base_metadata.dev();

    let mut entries = Vec::new();

    entries.push(make_mount_entry(".".to_string(), path, base_dev, false));

    let read_dir = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(_) => {
            return entries;
        }
    };

    for result in read_dir {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let metadata = match fs::metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                continue;
            }
        };

        if !metadata.is_dir() {
            continue;
        }

        let dev = metadata.dev();

        let is_boundary = dev != base_dev;

        if !all && !is_boundary {
            continue;
        }

        let name = dir_entry.file_name().to_string_lossy().to_string();

        entries.push(make_mount_entry(
            format!("{}/", name),
            &entry_path,
            dev,
            is_boundary,
        ));
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));

    entries
}

fn make_mount_entry(path_label: String, path: &Path, dev: u64, is_boundary: bool) -> MountEntry {
    let stats = filesystem_stats(path);

    let (used, available, total, percent_used) = match stats {
        Some((used, available, total)) => {
            let percent = if total == 0 {
                None
            } else {
                Some(used.saturating_mul(100) / total)
            };

            (Some(used), Some(available), Some(total), percent)
        }

        None => (None, None, None, None),
    };

    MountEntry {
        path: path_label,
        dev,
        used,
        available,
        total,
        percent_used,
        is_boundary,
    }
}

fn filesystem_stats(path: &Path) -> Option<(u64, u64, u64)> {
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;

    let mut stats = unsafe { std::mem::zeroed::<libc::statvfs>() };

    let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) };

    if result != 0 {
        return None;
    }

    let block_size = stats.f_frsize as u64;

    let total = stats.f_blocks.saturating_mul(block_size);

    let available = stats.f_bavail.saturating_mul(block_size);

    let free = stats.f_bfree.saturating_mul(block_size);

    let used = total.saturating_sub(free);

    Some((used, available, total))
}

fn print_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Mount scan:{}", palette.report.heading, palette.reset,);

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
        "  {}{:<22}{}  {}{:>10}{}  {}{:>8}{}  {}{:>8}{}  {}{:>8}{}  {}{:>5}{}",
        palette.report.label,
        "path",
        palette.reset,
        palette.report.label,
        "dev",
        palette.reset,
        palette.report.label,
        "used",
        palette.reset,
        palette.report.label,
        "avail",
        palette.reset,
        palette.report.label,
        "total",
        palette.reset,
        palette.report.label,
        "use%",
        palette.reset,
    );
}

fn print_mount_entry(entry: &MountEntry, palette: &AnsiPalette) {
    let used = optional_size(entry.used);

    let available = optional_size(entry.available);

    let total = optional_size(entry.total);

    let percent = entry
        .percent_used
        .map(|percent| format!("{}%", percent,))
        .unwrap_or_else(|| "-".to_string());

    let path_color = if entry.is_boundary {
        &palette.report.path
    } else {
        &palette.report.total
    };

    let usage_color = usage_color(entry.percent_used, palette);

    println!(
        "  {}{:<22}{}  {}{:>10}{}  {}{:>8}{}  {}{:>8}{}  {}{:>8}{}  {}{:>5}{}",
        path_color,
        entry.path,
        palette.reset,
        palette.report.count,
        entry.dev,
        palette.reset,
        palette.report.size,
        used,
        palette.reset,
        palette.report.size,
        available,
        palette.reset,
        palette.report.size,
        total,
        palette.reset,
        usage_color,
        percent,
        palette.reset,
    );
}

fn optional_size(value: Option<u64>) -> String {
    value.map(human_size).unwrap_or_else(|| "-".to_string())
}

fn usage_color<'a>(percent_used: Option<u64>, palette: &'a AnsiPalette) -> &'a str {
    match percent_used {
        Some(percent) if percent >= 90 => &palette.report.attention,

        _ => &palette.report.size,
    }
}

fn print_summary(entries: &[MountEntry], palette: &AnsiPalette) {
    let boundary_count = entries.iter().filter(|entry| entry.is_boundary).count();

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} filesystem boundaries found",
        palette.report.total, boundary_count, palette.reset,
    );
}
