// SPDX-License-Identifier: BSD-3-Clause

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::themes::AnsiPalette;

const LONG_NAME_LIMIT: usize = 120;

#[derive(Debug, Clone)]
struct PortableEntry {
    name: String,
    display_name: String,
}

#[derive(Debug, Clone)]
struct PortableWarning {
    kind: &'static str,
    name: String,
    detail: Option<String>,
}

#[derive(Debug)]
struct PortableReport {
    entries_scanned: usize,
    warnings: Vec<PortableWarning>,
    case_collisions: Vec<Vec<PortableEntry>>,
}

#[derive(Debug)]
struct PortableOptions {
    path: PathBuf,
}

pub fn handle_portable_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(portable_index) = arguments
        .iter()
        .position(|argument| argument == "--portable")
    else {
        return false;
    };

    let options = portable_options(arguments, portable_index);

    print_portability_report(&options, palette);

    true
}

fn portable_options(arguments: &[String], portable_index: usize) -> PortableOptions {
    let mut index = portable_index + 1;

    let mut path = PathBuf::from(".");

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
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

    PortableOptions { path }
}

fn print_portability_report(options: &PortableOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --portable requires a directory");

        std::process::exit(2);
    }

    let report = analyze_portability(path);

    print_header(path, palette);

    if report.warnings.is_empty() && report.case_collisions.is_empty() {
        println!(
            "  {}no portability warnings found{}",
            palette.report.total, palette.reset,
        );

        println!();

        print_summary(&report, palette);

        return;
    }

    print_warning_groups(&report.warnings, palette);

    print_case_collisions(&report.case_collisions, palette);

    print_summary(&report, palette);
}

fn analyze_portability(path: &Path) -> PortableReport {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut portable_entries = Vec::new();

    let mut warnings = Vec::new();

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

        let file_type = metadata.file_type();

        let is_dir = metadata.is_dir();

        let is_symlink = file_type.is_symlink();

        let display_name = display_name_for(&name, is_dir, is_symlink);

        let entry = PortableEntry {
            name: name.clone(),
            display_name: display_name.clone(),
        };

        collect_name_warnings(&entry, &mut warnings);

        portable_entries.push(entry);
    }

    portable_entries
        .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    warnings.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    let case_collisions = find_case_collisions(&portable_entries);

    PortableReport {
        entries_scanned: portable_entries.len(),
        warnings,
        case_collisions,
    }
}

fn collect_name_warnings(entry: &PortableEntry, warnings: &mut Vec<PortableWarning>) {
    let name = entry.name.as_str();

    if name.contains(' ') {
        push_warning(warnings, "space", entry, None);
    }

    if name.contains('\t') {
        push_warning(warnings, "tab", entry, Some("contains tab".to_string()));
    }

    if name.contains('\n') || name.contains('\r') {
        push_warning(
            warnings,
            "newline",
            entry,
            Some("contains newline".to_string()),
        );
    }

    if name.contains('\'') || name.contains('"') || name.contains('`') {
        push_warning(warnings, "quote", entry, None);
    }

    if name.starts_with('-') {
        push_warning(
            warnings,
            "leading-dash",
            entry,
            Some("may be parsed as an option".to_string()),
        );
    }

    if name.ends_with(' ') {
        push_warning(warnings, "trailing-space", entry, None);
    }

    if name.ends_with('.') {
        push_warning(
            warnings,
            "trailing-dot",
            entry,
            Some("awkward on Windows".to_string()),
        );
    }

    if name.chars().any(|character| character.is_control()) {
        push_warning(warnings, "control-char", entry, None);
    }

    if !name.is_ascii() {
        push_warning(
            warnings,
            "unicode",
            entry,
            Some("non-ASCII name".to_string()),
        );
    }

    if has_shell_metacharacter(name) {
        push_warning(
            warnings,
            "shell",
            entry,
            Some("needs careful quoting".to_string()),
        );
    }

    if has_windows_reserved_character(name) {
        push_warning(
            warnings,
            "win-char",
            entry,
            Some("reserved on Windows".to_string()),
        );
    }

    if is_windows_reserved_name(name) {
        push_warning(
            warnings,
            "win-name",
            entry,
            Some("reserved Windows device name".to_string()),
        );
    }

    if name.chars().count() >= LONG_NAME_LIMIT {
        push_warning(
            warnings,
            "long",
            entry,
            Some(format!("{} chars", name.chars().count())),
        );
    }
}

fn push_warning(
    warnings: &mut Vec<PortableWarning>,
    kind: &'static str,
    entry: &PortableEntry,
    detail: Option<String>,
) {
    warnings.push(PortableWarning {
        kind,
        name: entry.display_name.clone(),
        detail,
    });
}

fn has_shell_metacharacter(name: &str) -> bool {
    const SHELL_CHARS: [char; 15] = [
        '$', '&', '|', ';', '<', '>', '(', ')', '[', ']', '*', '?', '!', '\\', '#',
    ];

    name.chars()
        .any(|character| SHELL_CHARS.contains(&character))
}

fn has_windows_reserved_character(name: &str) -> bool {
    const WINDOWS_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    name.chars()
        .any(|character| WINDOWS_CHARS.contains(&character))
}

fn is_windows_reserved_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim().to_uppercase();

    matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn find_case_collisions(entries: &[PortableEntry]) -> Vec<Vec<PortableEntry>> {
    let mut by_lowercase: BTreeMap<String, Vec<PortableEntry>> = BTreeMap::new();

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
        "{}Portability warnings:{}",
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

fn print_warning_groups(warnings: &[PortableWarning], palette: &AnsiPalette) {
    let mut grouped: BTreeMap<&str, Vec<&PortableWarning>> = BTreeMap::new();

    for warning in warnings {
        grouped.entry(warning.kind).or_default().push(warning);
    }

    for (kind, items) in grouped {
        print_group_heading(kind, items.len(), palette);

        for item in items {
            print_warning_item(item, palette);
        }

        println!();
    }
}

fn print_group_heading(kind: &str, count: usize, palette: &AnsiPalette) {
    println!(
        "{}{}:{} {}{}{}",
        palette.report.heading, kind, palette.reset, palette.report.count, count, palette.reset,
    );
}

fn print_warning_item(warning: &PortableWarning, palette: &AnsiPalette) {
    match warning.detail.as_ref() {
        Some(detail) => {
            println!(
                "  {}{:<14}{}  {}{}{}  {}({}){}",
                palette.report.attention,
                warning.kind,
                palette.reset,
                palette.report.name,
                warning.name,
                palette.reset,
                palette.report.muted,
                detail,
                palette.reset,
            );
        }

        None => {
            println!(
                "  {}{:<14}{}  {}{}{}",
                palette.report.attention,
                warning.kind,
                palette.reset,
                palette.report.name,
                warning.name,
                palette.reset,
            );
        }
    }
}

fn print_case_collisions(groups: &[Vec<PortableEntry>], palette: &AnsiPalette) {
    if groups.is_empty() {
        return;
    }

    println!(
        "{}case-collisions:{} {}{}{}",
        palette.report.heading,
        palette.reset,
        palette.report.count,
        groups.len(),
        palette.reset,
    );

    for group in groups {
        let names = group
            .iter()
            .map(|entry| entry.display_name.clone())
            .collect::<Vec<String>>()
            .join(" / ");

        println!(
            "  {}case-pair{}       {}{}{}",
            palette.report.attention, palette.reset, palette.report.name, names, palette.reset,
        );
    }

    println!();
}

fn print_summary(report: &PortableReport, palette: &AnsiPalette) {
    let total_warnings = report.warnings.len() + report.case_collisions.len();

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{} entries scanned",
        palette.report.total, report.entries_scanned, palette.reset,
    );

    let warning_color = if total_warnings > 0 {
        &palette.report.attention
    } else {
        &palette.report.total
    };

    println!(
        "  {}{}{} warning items found",
        warning_color, total_warnings, palette.reset,
    );
}
