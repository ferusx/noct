// SPDX-License-Identifier: BSD-3-Clause

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::themes::AnsiPalette;

pub fn handle_package_bins_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    for index in 0..arguments.len() {
        if arguments[index] != "--package-bins" {
            continue;
        }

        if index + 1 >= arguments.len() {
            eprintln!("noct: --package-bins requires a package name");

            std::process::exit(2);
        }

        print_package_bins(&arguments[index + 1], palette);

        return true;
    }

    false
}

pub fn handle_owns_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    for index in 0..arguments.len() {
        if arguments[index] != "--owns" {
            continue;
        }

        if index + 1 >= arguments.len() {
            eprintln!("noct: --owns requires a path or command name");

            std::process::exit(2);
        }

        print_owner_lookup(&arguments[index + 1], palette);

        return true;
    }

    false
}

fn print_package_bins(package_name: &str, palette: &AnsiPalette) {
    let Some(package_files) = package_file_list(package_name) else {
        if print_command_lookup(package_name, palette) {
            return;
        }

        eprintln!(
            "noct: '{}' is neither an installed package nor a command found in PATH",
            package_name
        );

        std::process::exit(1);
    };

    let mut executable_paths = BTreeSet::new();

    let mut command_names = BTreeSet::new();

    for path_text in package_files {
        if !looks_like_command_path(&path_text) {
            continue;
        }

        let path = Path::new(&path_text);

        let Ok(metadata) = fs::metadata(path) else {
            continue;
        };

        if !metadata.is_file() {
            continue;
        }

        if metadata.permissions().mode() & 0o111 == 0 {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };

        let command_name = file_name.to_string_lossy().to_string();

        executable_paths.insert(path_text);

        command_names.insert(command_name);
    }

    print_heading("Package:", palette);

    print_value(package_name, &palette.report.package, palette);

    println!();

    print_heading("Version:", palette);

    match package_version(package_name) {
        Some(version) => {
            print_value(version, &palette.report.version, palette);
        }

        None => {
            print_muted_value("unknown", palette);
        }
    }

    println!();

    print_heading("Executable paths:", palette);

    if executable_paths.is_empty() {
        print_muted_value("none", palette);
    } else {
        for executable_path in &executable_paths {
            print_value(executable_path, &palette.report.path, palette);
        }
    }

    println!();

    print_heading("Executables:", palette);

    if command_names.is_empty() {
        print_muted_value("none", palette);
    } else {
        for command_name in &command_names {
            print_value(command_name, &palette.report.command, palette);
        }
    }
}

fn package_file_list(package_name: &str) -> Option<Vec<String>> {
    #[cfg(target_os = "freebsd")]
    {
        return freebsd_package_file_list(package_name);
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        linux_package_file_list(package_name)
    }
}

#[cfg(not(target_os = "freebsd"))]
fn linux_package_file_list(package_name: &str) -> Option<Vec<String>> {
    let output = Command::new("dpkg")
        .arg("-L")
        .arg(package_name)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    Some(
        stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

#[cfg(target_os = "freebsd")]
fn freebsd_package_file_list(package_name: &str) -> Option<Vec<String>> {
    let output = Command::new("pkg")
        .arg("info")
        .arg("-l")
        .arg(package_name)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let files = stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('/'))
        .map(str::to_string)
        .collect::<Vec<_>>();

    Some(files)
}

pub fn print_command_lookup(command_name: &str, palette: &AnsiPalette) -> bool {
    let Some(command_path) = find_command_in_path(command_name) else {
        return false;
    };

    print_heading("Command:", palette);

    print_value(command_name, &palette.report.command, palette);

    println!();

    print_heading("Path:", palette);

    print_value(command_path.display(), &palette.report.path, palette);

    println!();

    print_heading("Owned by package:", palette);

    match package_owner_for_path_deep(&command_path) {
        Some(package_name) => {
            print_value(&package_name, &palette.report.package, palette);

            println!();

            print_heading("Version:", palette);

            match package_version(&package_name) {
                Some(version) => {
                    print_value(version, &palette.report.version, palette);
                }

                None => {
                    print_muted_value("unknown", palette);
                }
            }
        }

        None => {
            print_muted_value("unknown", palette);
        }
    }

    true
}

fn print_owner_lookup(target: &str, palette: &AnsiPalette) {
    let path = if target.contains('/') {
        PathBuf::from(target)
    } else {
        let Some(command_path) = find_command_in_path(target) else {
            eprintln!("noct: command '{}' was not found in PATH", target);

            std::process::exit(1);
        };

        command_path
    };

    if !path.exists() {
        eprintln!("noct: path '{}' does not exist", path.display());

        std::process::exit(1);
    }

    print_heading("Target:", palette);

    let target_color = if target.contains('/') {
        &palette.report.path
    } else {
        &palette.report.command
    };

    print_value(target, target_color, palette);

    println!();

    print_heading("Path:", palette);

    print_value(path.display(), &palette.report.path, palette);

    println!();

    print_heading("Owned by package:", palette);

    match package_owner_for_path_deep(&path) {
        Some(package_name) => {
            print_value(&package_name, &palette.report.package, palette);

            println!();

            print_heading("Version:", palette);

            match package_version(&package_name) {
                Some(version) => {
                    print_value(version, &palette.report.version, palette);
                }

                None => {
                    print_muted_value("unknown", palette);
                }
            }
        }

        None => {
            print_muted_value("unknown", palette);
        }
    }
}

pub fn find_command_in_path(command_name: &str) -> Option<PathBuf> {
    if command_name.contains('/') {
        let path = PathBuf::from(command_name);

        if path_is_executable_file(&path) {
            return Some(path);
        }

        return None;
    }

    let paths = env::var_os("PATH")?;

    for directory in env::split_paths(&paths) {
        let candidate = directory.join(command_name);

        if path_is_executable_file(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn path_is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

fn package_owner_for_path(path: &Path) -> Option<String> {
    #[cfg(target_os = "freebsd")]
    {
        return freebsd_package_owner_for_path(path);
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        linux_package_owner_for_path(path)
    }
}

#[cfg(not(target_os = "freebsd"))]
fn linux_package_owner_for_path(path: &Path) -> Option<String> {
    let output = Command::new("dpkg-query")
        .arg("-S")
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let first_line = stdout.lines().next()?.trim();

    let package_part = first_line.split(':').next()?.trim();

    if package_part.is_empty() {
        None
    } else {
        Some(package_part.to_string())
    }
}

#[cfg(target_os = "freebsd")]
fn freebsd_package_owner_for_path(path: &Path) -> Option<String> {
    let output = Command::new("pkg")
        .arg("which")
        .arg("-q")
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let package_identifier = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if package_identifier.is_empty() {
        return None;
    }

    let output = Command::new("pkg")
        .arg("query")
        .arg("%n")
        .arg(&package_identifier)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let package_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if package_name.is_empty() {
        None
    } else {
        Some(package_name)
    }
}

pub fn package_owner_for_path_deep(path: &Path) -> Option<String> {
    if let Some(package_name) = package_owner_for_path(path) {
        return Some(package_name);
    }

    let Ok(canonical_path) = fs::canonicalize(path) else {
        return None;
    };

    if canonical_path == path {
        return None;
    }

    package_owner_for_path(&canonical_path)
}

pub fn package_version(package_name: &str) -> Option<String> {
    #[cfg(target_os = "freebsd")]
    {
        return freebsd_package_version(package_name);
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        linux_package_version(package_name)
    }
}

#[cfg(not(target_os = "freebsd"))]
fn linux_package_version(package_name: &str) -> Option<String> {
    let output = Command::new("dpkg-query")
        .arg("-W")
        .arg("-f=${Version}")
        .arg(package_name)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

#[cfg(target_os = "freebsd")]
fn freebsd_package_version(package_name: &str) -> Option<String> {
    let output = Command::new("pkg")
        .arg("query")
        .arg("%v")
        .arg(package_name)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

pub fn looks_like_command_path(path: &str) -> bool {
    const COMMAND_DIRS: [&str; 6] = [
        "/bin/",
        "/sbin/",
        "/usr/bin/",
        "/usr/sbin/",
        "/usr/local/bin/",
        "/usr/local/sbin/",
    ];

    COMMAND_DIRS
        .iter()
        .any(|command_dir| path.starts_with(command_dir))
}

pub fn is_command_path(path: &Path, metadata: &fs::Metadata) -> bool {
    if !metadata.is_file() {
        return false;
    }

    if metadata.permissions().mode() & 0o111 == 0 {
        return false;
    }

    let path_text = path.to_string_lossy();

    looks_like_command_path(&path_text)
}

fn print_heading(heading: &str, palette: &AnsiPalette) {
    println!("{}{}{}", palette.report.heading, heading, palette.reset,);
}

fn print_value(value: impl std::fmt::Display, color: &str, palette: &AnsiPalette) {
    println!("  {}{}{}", color, value, palette.reset,);
}

fn print_muted_value(value: &str, palette: &AnsiPalette) {
    print_value(value, &palette.report.muted, palette);
}
