use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::themes::AnsiPalette;

pub fn handle_links_to_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    for index in 0..arguments.len() {
        if arguments[index] != "--links-to" && arguments[index] != "--link-to" {
            continue;
        }

        if index + 1 >= arguments.len() {
            eprintln!("noct: --links-to requires a symlink path");

            std::process::exit(2);
        }

        print_link_target(&arguments[index + 1], palette);

        return true;
    }

    false
}

fn print_link_target(target: &str, palette: &AnsiPalette) {
    let supplied_path = PathBuf::from(target);

    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let absolute_path = if supplied_path.is_absolute() {
        supplied_path
    } else {
        current_dir.join(supplied_path)
    };

    let metadata = match fs::symlink_metadata(&absolute_path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!(
                "noct: unable to inspect {}: {}",
                absolute_path.display(),
                error,
            );

            std::process::exit(1);
        }
    };

    if !metadata.file_type().is_symlink() {
        eprintln!(
            "noct: --links-to requires a symlink: {}",
            absolute_path.display(),
        );

        std::process::exit(2);
    }

    let link_target = match fs::read_link(&absolute_path) {
        Ok(target) => target,

        Err(error) => {
            eprintln!(
                "noct: unable to read symlink {}: {}",
                absolute_path.display(),
                error,
            );

            std::process::exit(1);
        }
    };

    let target_absolute = if link_target.is_absolute() {
        link_target.clone()
    } else {
        absolute_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&link_target)
    };

    println!("{}Link:{}", palette.report.heading, palette.reset);

    println!(
        "  {}{}{}",
        palette.report.path,
        absolute_path.display(),
        palette.reset,
    );

    println!();

    println!("{}Points to:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.value,
        link_target.display(),
        palette.reset,
    );

    println!();

    println!("{}Resolved:{}", palette.report.heading, palette.reset);

    println!(
        "  {}{}{}",
        palette.report.path,
        target_absolute.display(),
        palette.reset,
    );
}
