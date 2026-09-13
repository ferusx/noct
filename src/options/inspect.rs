// SPDX-License-Identifier: BSD-3-Clause

use std::env;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

use super::package::{
    find_command_in_path, is_command_path, package_owner_for_path_deep, package_version,
};
use crate::config::FilesystemColorsMode;
use crate::entry::{detect_state, file_kind_label, file_type_char};
use crate::format::{
    extension_label, group_name, human_age, human_date, human_size, octal_mode, user_name, yes_no,
};
use crate::render::{RenderOptions, render_permissions};
use crate::themes::AnsiPalette;

pub fn handle_inspect_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    for index in 0..arguments.len() {
        if arguments[index] != "--inspect" {
            continue;
        }

        if index + 1 >= arguments.len() {
            eprintln!("noct: --inspect requires a path or command name");

            std::process::exit(2);
        }

        print_inspection(&arguments[index + 1], palette);

        return true;
    }

    false
}

fn print_info_line(label: &str, value: impl std::fmt::Display, palette: &AnsiPalette) {
    let value_color = match label {
        "Name" => &palette.report.name,

        "Path" | "Parent" | "Resolved" | "Link target" => &palette.report.path,

        "Command name" => &palette.report.command,

        "Kind" | "File type" | "Extension" => &palette.report.package,

        "Size" => &palette.report.size,

        "Bytes" | "UID" | "GID" | "Age" => &palette.report.count,

        "Modified" | "Accessed" | "Created" => &palette.columns.time,

        "Package owner" => {
            if value.to_string() == "unknown" {
                &palette.report.muted
            } else {
                &palette.report.package
            }
        }

        "Package version" => &palette.report.version,

        "State" => {
            if value.to_string() == "normal" {
                &palette.report.total
            } else {
                &palette.report.attention
            }
        }

        _ => &palette.report.value,
    };

    println!(
        "{}{:<16}{} {}{}{}",
        palette.report.label,
        format!("{}:", label,),
        palette.reset,
        value_color,
        value,
        palette.reset,
    );
}

fn print_inspection(target: &str, palette: &AnsiPalette) {
    let path = if target.contains('/') {
        PathBuf::from(target)
    } else {
        let direct_path = PathBuf::from(target);

        if direct_path.exists() {
            direct_path
        } else if let Some(command_path) = find_command_in_path(target) {
            command_path
        } else {
            direct_path
        }
    };

    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to inspect '{}': {}", target, error);

            if !target.contains('/') {
                eprintln!(
                    "noct: '{}' was not found as a local path or command in PATH",
                    target
                );
            }

            std::process::exit(1);
        }
    };

    let canonical_path = fs::canonicalize(&path).ok();

    let absolute_path = if path.is_absolute() {
        path.clone()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&path)
    };

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| target.to_string());

    let parent = absolute_path
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| "none".to_string());

    let state = detect_state(&path, &metadata);

    let mode = metadata.permissions().mode();

    let permissions = render_permissions(
        file_type_char(&metadata),
        mode,
        &RenderOptions {
            use_colors: palette.enabled,
            filesystem_colors: FilesystemColorsMode::Theme,
            show_icons: false,
            show_permissions: false,
            show_age: false,
            show_user: false,
            show_date: false,
            show_state: false,
            show_state_labels: false,
            grid_padding: 2,
            long_gap: "  ",
            output_width: None,
        },
        palette,
    );

    let owner = user_name(metadata.uid());

    let group = group_name(metadata.gid());

    let modified = metadata.modified().ok();

    let accessed = metadata.accessed().ok();

    let created = metadata.created().ok();

    let is_symlink = metadata.file_type().is_symlink();

    let is_executable = metadata.is_file() && metadata.permissions().mode() & 0o111 != 0;

    let is_command = is_command_path(&absolute_path, &metadata);

    print_info_line("Name", &file_name, palette);

    print_info_line("Path", absolute_path.display(), palette);

    print_info_line("Parent", parent, palette);

    if let Some(canonical_path) = canonical_path.as_ref() {
        print_info_line("Resolved", canonical_path.display(), palette);
    }

    if is_command {
        print_info_line("Kind", "command", palette);

        print_info_line("File type", file_kind_label(&metadata), palette);

        if let Some(command_name) = absolute_path.file_name() {
            print_info_line("Command name", command_name.to_string_lossy(), palette);
        }
    } else {
        print_info_line("Kind", file_kind_label(&metadata), palette);
    }

    print_info_line("State", state, palette);

    print_info_line("Size", human_size(metadata.len()), palette);

    print_info_line("Bytes", metadata.len(), palette);

    print_info_line("Permissions", permissions, palette);

    print_info_line("Mode", octal_mode(mode), palette);

    print_info_line("Owner", owner, palette);

    print_info_line("UID", metadata.uid(), palette);

    print_info_line("Group", group, palette);

    print_info_line("GID", metadata.gid(), palette);

    print_info_line("Modified", human_date(modified), palette);

    print_info_line("Accessed", human_date(accessed), palette);

    print_info_line("Created", human_date(created), palette);

    print_info_line("Age", human_age(modified), palette);

    print_info_line("Extension", extension_label(&path), palette);

    print_info_line("Hidden", yes_no(file_name.starts_with('.')), palette);

    print_info_line("Executable", yes_no(is_executable), palette);

    print_info_line("Symlink", yes_no(is_symlink), palette);

    if is_symlink {
        match fs::read_link(&path) {
            Ok(target_path) => {
                print_info_line("Link target", target_path.display(), palette);
            }

            Err(error) => {
                print_info_line("Link target", format!("unavailable ({})", error,), palette);
            }
        }
    }

    if let Some(package_name) = package_owner_for_path_deep(&absolute_path) {
        print_info_line("Package owner", &package_name, palette);

        if let Some(version) = package_version(&package_name) {
            print_info_line("Package version", version, palette);
        }
    } else {
        print_info_line("Package owner", "unknown", palette);
    }
}
