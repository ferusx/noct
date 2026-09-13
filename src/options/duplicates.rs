// SPDX-License-Identifier: BSD-3-Clause

use crate::args::parse_size_argument;
use crate::format::human_size;
use crate::themes::AnsiPalette;

use chrono::Local;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

pub fn handle_duplicates_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(duplicates_index) = arguments
        .iter()
        .position(|argument| argument == "--duplicates")
    else {
        return false;
    };

    let recursive = arguments
        .iter()
        .any(|argument| argument == "--recursive" || argument == "-R");

    let min_size = duplicate_min_size(arguments);

    let search_path = duplicate_search_path(arguments, duplicates_index);

    if recursive {
        eprintln!("Scanning for duplicates. Please wait...\n");
    }

    print_duplicates(&search_path, recursive, min_size, palette);

    true
}

fn duplicate_search_path(arguments: &[String], duplicates_index: usize) -> PathBuf {
    let mut index = duplicates_index + 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--recursive" | "-R" => {
                index += 1;
            }

            "--min-size" => {
                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                index += 1;
            }

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

fn duplicate_min_size(arguments: &[String]) -> u64 {
    for index in 0..arguments.len() {
        if arguments[index].starts_with("--min-size=") {
            let value = arguments[index].trim_start_matches("--min-size=");

            let Some(size) = parse_size_argument(value) else {
                eprintln!("noct: invalid min size '{}'", value);

                std::process::exit(2);
            };

            return size;
        }

        if arguments[index] != "--min-size" {
            continue;
        }

        if index + 1 >= arguments.len() {
            eprintln!("noct: --min-size requires a size, for example 1M or 500K");

            std::process::exit(2);
        }

        let Some(size) = parse_size_argument(&arguments[index + 1]) else {
            eprintln!("noct: invalid min size '{}'", arguments[index + 1]);

            std::process::exit(2);
        };

        return size;
    }

    0
}

fn print_duplicates(path: &Path, recursive: bool, min_size: u64, palette: &AnsiPalette) {
    let mut files = Vec::new();

    collect_regular_files(path, recursive, min_size, &mut files);

    let duplicate_groups = find_duplicate_groups(files);

    let summary = duplicate_summary(&duplicate_groups);

    print_duplicates_header(path, recursive, min_size, palette);

    print_duplicates_summary(&summary, palette);

    if duplicate_groups.is_empty() {
        return;
    }

    let report_path =
        match write_duplicate_report(path, recursive, min_size, &duplicate_groups, &summary) {
            Ok(path) => path,

            Err(error) => {
                eprintln!("noct: unable to write duplicate report: {}", error);

                std::process::exit(1);
            }
        };

    println!();

    println!("{}Report:{}", palette.report.heading, palette.reset);

    println!(
        "  {}{}{}",
        palette.report.path,
        report_path.display(),
        palette.reset,
    );
}

fn print_duplicates_header(path: &Path, recursive: bool, min_size: u64, palette: &AnsiPalette) {
    println!("{}Duplicate scan:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!(
        "  {}mode:{} {}{}{}",
        palette.report.label,
        palette.reset,
        palette.report.value,
        if recursive {
            "recursive"
        } else {
            "current directory"
        },
        palette.reset,
    );

    println!(
        "  {}minimum size:{} {}{}{}",
        palette.report.label,
        palette.reset,
        palette.report.size,
        human_size(min_size),
        palette.reset,
    );

    println!();
}

#[derive(Debug)]
struct DuplicateSummary {
    groups: usize,
    files: usize,
    reclaimable_size: u64,
}

fn duplicate_summary(groups: &[Vec<CandidateFile>]) -> DuplicateSummary {
    let files = groups.iter().map(|group| group.len()).sum::<usize>();

    let reclaimable_size = groups
        .iter()
        .map(|group| {
            let size = group.first().map(|file| file.size).unwrap_or(0);

            let removable_copies = group.len().saturating_sub(1) as u64;

            size.saturating_mul(removable_copies)
        })
        .fold(0u64, |sum, size| sum.saturating_add(size));

    DuplicateSummary {
        groups: groups.len(),
        files,
        reclaimable_size,
    }
}

fn print_duplicates_summary(summary: &DuplicateSummary, palette: &AnsiPalette) {
    println!("{}Summary:{}", palette.report.heading, palette.reset);

    println!(
        "  {}{}{} duplicate groups found",
        palette.report.total, summary.groups, palette.reset,
    );

    println!(
        "  {}{}{} files belong to duplicate groups",
        palette.report.count, summary.files, palette.reset,
    );

    println!(
        "  {}{}{} potentially reclaimable",
        palette.report.size,
        human_size(summary.reclaimable_size),
        palette.reset,
    );
}

fn write_duplicate_report(
    search_path: &Path,
    recursive: bool,
    min_size: u64,
    groups: &[Vec<CandidateFile>],
    summary: &DuplicateSummary,
) -> std::io::Result<PathBuf> {
    let invocation_directory = env::current_dir()?;

    let now = Local::now();

    let filename = format!(
        "noct-duplicates-{}-{:03}.txt",
        now.format("%Y%m%d-%H%M%S"),
        now.timestamp_subsec_millis(),
    );

    let report_path = invocation_directory.join(filename);

    let mut report = String::new();

    report.push_str("Noct duplicate report\n");
    report.push_str("=====================\n\n");

    report.push_str(&format!("Created: {}\n", now.format("%Y-%m-%d %H:%M:%S"),));

    report.push_str(&format!("Search path: {}\n", search_path.display(),));

    report.push_str(&format!(
        "Mode: {}\n",
        if recursive {
            "recursive"
        } else {
            "current directory"
        },
    ));

    report.push_str(&format!("Minimum size: {}\n\n", human_size(min_size),));

    report.push_str("Summary\n");
    report.push_str("-------\n");

    report.push_str(&format!("Duplicate groups: {}\n", summary.groups,));

    report.push_str(&format!("Files in duplicate groups: {}\n", summary.files,));

    report.push_str(&format!(
        "Potentially reclaimable: {}\n\n",
        human_size(summary.reclaimable_size),
    ));

    report.push_str("Duplicate groups\n");
    report.push_str("----------------\n\n");

    for (group_index, group) in groups.iter().enumerate() {
        let size = group.first().map(|file| file.size).unwrap_or(0);

        report.push_str(&format!(
            "Group {}: {} each, {} files\n",
            group_index + 1,
            human_size(size),
            group.len(),
        ));

        for file in group {
            report.push_str(&format!("  {}\n", file.path.display()));
        }

        report.push('\n');
    }

    fs::write(&report_path, report)?;

    Ok(report_path)
}

#[derive(Debug, Clone)]
struct CandidateFile {
    path: PathBuf,
    size: u64,
}

fn collect_regular_files(
    path: &Path,
    recursive: bool,
    min_size: u64,
    files: &mut Vec<CandidateFile>,
) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            return;
        }
    };

    if metadata.is_file() {
        if metadata.len() >= min_size {
            files.push(CandidateFile {
                path: path.to_path_buf(),
                size: metadata.len(),
            });
        }

        return;
    }

    if !metadata.is_dir() {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            return;
        }
    };

    for result in entries {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let Ok(entry_metadata) = fs::symlink_metadata(&entry_path) else {
            continue;
        };

        if entry_metadata.is_file() {
            if entry_metadata.len() >= min_size {
                files.push(CandidateFile {
                    path: entry_path,
                    size: entry_metadata.len(),
                });
            }

            continue;
        }

        if recursive && entry_metadata.is_dir() {
            collect_regular_files(&entry_path, recursive, min_size, files);
        }
    }
}

fn find_duplicate_groups(files: Vec<CandidateFile>) -> Vec<Vec<CandidateFile>> {
    let mut by_size: BTreeMap<u64, Vec<CandidateFile>> = BTreeMap::new();

    for file in files {
        by_size.entry(file.size).or_default().push(file);
    }

    let mut duplicate_groups = Vec::new();

    for (_size, same_size_files) in by_size {
        if same_size_files.len() < 2 {
            continue;
        }

        let mut by_hash: HashMap<u64, Vec<CandidateFile>> = HashMap::new();

        for file in same_size_files {
            let Some(hash) = hash_file(&file.path) else {
                continue;
            };

            by_hash.entry(hash).or_default().push(file);
        }

        for (_hash, same_hash_files) in by_hash {
            if same_hash_files.len() < 2 {
                continue;
            }

            let verified_groups = verify_duplicate_candidates(same_hash_files);

            for group in verified_groups {
                if group.len() >= 2 {
                    duplicate_groups.push(group);
                }
            }
        }
    }

    duplicate_groups
}

fn hash_file(path: &Path) -> Option<u64> {
    let file = fs::File::open(path).ok()?;

    let mut reader = BufReader::new(file);

    let mut hasher = DefaultHasher::new();

    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer).ok()?;

        if bytes_read == 0 {
            break;
        }

        buffer[..bytes_read].hash(&mut hasher);
    }

    Some(hasher.finish())
}

fn verify_duplicate_candidates(candidates: Vec<CandidateFile>) -> Vec<Vec<CandidateFile>> {
    let mut groups: Vec<Vec<CandidateFile>> = Vec::new();

    'candidate_loop: for candidate in candidates {
        for group in &mut groups {
            let Some(first_file) = group.first() else {
                continue;
            };

            if files_are_equal(&candidate.path, &first_file.path) {
                group.push(candidate);

                continue 'candidate_loop;
            }
        }

        groups.push(vec![candidate]);
    }

    groups
}

fn files_are_equal(left: &Path, right: &Path) -> bool {
    let left_file = match fs::File::open(left) {
        Ok(file) => file,

        Err(_) => {
            return false;
        }
    };

    let right_file = match fs::File::open(right) {
        Ok(file) => file,

        Err(_) => {
            return false;
        }
    };

    let mut left_reader = BufReader::new(left_file);

    let mut right_reader = BufReader::new(right_file);

    let mut left_buffer = [0u8; 8192];

    let mut right_buffer = [0u8; 8192];

    loop {
        let left_read = match left_reader.read(&mut left_buffer) {
            Ok(bytes_read) => bytes_read,

            Err(_) => {
                return false;
            }
        };

        let right_read = match right_reader.read(&mut right_buffer) {
            Ok(bytes_read) => bytes_read,

            Err(_) => {
                return false;
            }
        };

        if left_read != right_read {
            return false;
        }

        if left_read == 0 {
            return true;
        }

        if left_buffer[..left_read] != right_buffer[..right_read] {
            return false;
        }
    }
}
