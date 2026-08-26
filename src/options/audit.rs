use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::format::{gid_exists, uid_exists};
use crate::themes::AnsiPalette;

#[derive(Debug)]
struct AuditItem {
    name: String,
    detail: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum AuditDetailKind {
    Target,
    Attention,
    Command,
}

#[derive(Debug)]
struct AuditReport {
    broken_links: Vec<AuditItem>,
    world_writable: Vec<AuditItem>,
    setuid: Vec<AuditItem>,
    setgid: Vec<AuditItem>,
    no_owner: Vec<AuditItem>,
    no_group: Vec<AuditItem>,
    scripts: Vec<AuditItem>,
}

impl AuditReport {
    fn new() -> Self {
        Self {
            broken_links: Vec::new(),
            world_writable: Vec::new(),
            setuid: Vec::new(),
            setgid: Vec::new(),
            no_owner: Vec::new(),
            no_group: Vec::new(),
            scripts: Vec::new(),
        }
    }

    fn attention_count(&self) -> usize {
        self.broken_links.len()
            + self.world_writable.len()
            + self.setuid.len()
            + self.setgid.len()
            + self.no_owner.len()
            + self.no_group.len()
    }
}

pub fn handle_audit_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(audit_index) = arguments.iter().position(|argument| argument == "--audit") else {
        return false;
    };

    let path = audit_search_path(arguments, audit_index);

    print_audit(&path, palette);

    true
}

fn audit_search_path(arguments: &[String], audit_index: usize) -> PathBuf {
    let mut index = audit_index + 1;

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

fn print_audit(path: &Path, palette: &AnsiPalette) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --audit requires a directory");

        std::process::exit(2);
    }

    let report = audit_directory(path);

    print_audit_header(path, palette);

    print_audit_section(
        "broken links",
        &report.broken_links,
        AuditDetailKind::Target,
        palette,
    );

    print_audit_section(
        "world-writable",
        &report.world_writable,
        AuditDetailKind::Attention,
        palette,
    );

    print_audit_section(
        "setuid",
        &report.setuid,
        AuditDetailKind::Attention,
        palette,
    );

    print_audit_section(
        "setgid",
        &report.setgid,
        AuditDetailKind::Attention,
        palette,
    );

    print_audit_section(
        "no owner",
        &report.no_owner,
        AuditDetailKind::Attention,
        palette,
    );

    print_audit_section(
        "no group",
        &report.no_group,
        AuditDetailKind::Attention,
        palette,
    );

    print_audit_section(
        "executable scripts",
        &report.scripts,
        AuditDetailKind::Command,
        palette,
    );

    print_audit_summary(&report, palette);
}

fn audit_directory(path: &Path) -> AuditReport {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut report = AuditReport::new();

    for result in entries {
        let Ok(dir_entry) = result else {
            continue;
        };

        let entry_path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        let metadata = match fs::symlink_metadata(&entry_path) {
            Ok(metadata) => metadata,

            Err(_) => {
                continue;
            }
        };

        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            match fs::read_link(&entry_path) {
                Ok(target) => {
                    if fs::metadata(&entry_path).is_err() {
                        report.broken_links.push(AuditItem {
                            name: name.clone(),
                            detail: Some(target.display().to_string()),
                        });
                    }
                }

                Err(_) => {
                    report.broken_links.push(AuditItem {
                        name: name.clone(),
                        detail: Some("unreadable target".to_string()),
                    });
                }
            }
        }

        let mode = metadata.permissions().mode();

        if mode & 0o002 != 0 {
            report.world_writable.push(AuditItem {
                name: name.clone(),
                detail: Some(format!("{:04o}", mode & 0o7777)),
            });
        }

        if mode & 0o4000 != 0 {
            report.setuid.push(AuditItem {
                name: name.clone(),
                detail: Some(format!("{:04o}", mode & 0o7777)),
            });
        }

        if mode & 0o2000 != 0 {
            report.setgid.push(AuditItem {
                name: name.clone(),
                detail: Some(format!("{:04o}", mode & 0o7777)),
            });
        }

        if !uid_exists(metadata.uid()) {
            report.no_owner.push(AuditItem {
                name: name.clone(),
                detail: Some(metadata.uid().to_string()),
            });
        }

        if !gid_exists(metadata.gid()) {
            report.no_group.push(AuditItem {
                name: name.clone(),
                detail: Some(metadata.gid().to_string()),
            });
        }

        if metadata.is_file() && mode & 0o111 != 0 {
            if let Some(shebang) = script_shebang(&entry_path) {
                report.scripts.push(AuditItem {
                    name: name.clone(),
                    detail: Some(shebang),
                });
            }
        }
    }

    report
}

fn script_shebang(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;

    let mut reader = BufReader::new(file);

    let mut first_line = String::new();

    reader.read_line(&mut first_line).ok()?;

    let trimmed = first_line.trim();

    if !trimmed.starts_with("#!") {
        return None;
    }

    Some(trimmed.trim_start_matches("#!").trim().to_string())
}

fn print_audit_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Audit:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();
}

fn print_audit_section(
    title: &str,
    items: &[AuditItem],
    detail_kind: AuditDetailKind,
    palette: &AnsiPalette,
) {
    if items.is_empty() {
        return;
    }

    println!("{}{}:{}", palette.report.heading, title, palette.reset,);

    for item in items {
        print_audit_item(item, detail_kind, palette);
    }

    println!();
}

fn print_audit_item(item: &AuditItem, detail_kind: AuditDetailKind, palette: &AnsiPalette) {
    let detail_color = match detail_kind {
        AuditDetailKind::Target => &palette.report.path,

        AuditDetailKind::Attention => &palette.report.attention,

        AuditDetailKind::Command => &palette.report.command,
    };

    match item.detail.as_ref() {
        Some(detail) => {
            println!(
                "  {}{}{}  {}->{}  {}{}{}",
                palette.report.name,
                item.name,
                palette.reset,
                palette.report.muted,
                palette.reset,
                detail_color,
                detail,
                palette.reset,
            );
        }

        None => {
            println!("  {}{}{}", palette.report.name, item.name, palette.reset,);
        }
    }
}

fn print_audit_summary(report: &AuditReport, palette: &AnsiPalette) {
    let attention_count = report.attention_count();

    let script_count = report.scripts.len();

    println!("{}Summary:{}", palette.report.heading, palette.reset,);

    let attention_color = if attention_count > 0 {
        &palette.report.attention
    } else {
        &palette.report.total
    };

    println!(
        "  {}{}{} attention items found",
        attention_color, attention_count, palette.reset,
    );

    println!(
        "  {}{}{} executable scripts found",
        palette.report.command, script_count, palette.reset,
    );

    if attention_count == 0 {
        println!(
            "  {}no dangerous permissions or broken ownership found{}",
            palette.report.total, palette.reset,
        );
    }
}
