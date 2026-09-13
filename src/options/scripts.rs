// SPDX-License-Identifier: BSD-3-Clause

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::themes::AnsiPalette;

#[derive(Debug, Clone)]
struct ScriptEntry {
    name: String,
    interpreter: String,
    command: String,
}

#[derive(Debug, Clone)]
struct InterpreterGroup {
    interpreter: String,
    scripts: Vec<ScriptEntry>,
}

#[derive(Debug, Clone, Copy)]
enum ScriptSortMode {
    Count,
    Interpreter,
    Name,
}

#[derive(Debug)]
struct ScriptOptions {
    path: PathBuf,
    sort_mode: ScriptSortMode,
    limit: Option<usize>,
}

pub fn handle_scripts_request(arguments: &[String], palette: &AnsiPalette) -> bool {
    let Some(scripts_index) = arguments
        .iter()
        .position(|argument| argument == "--scripts")
    else {
        return false;
    };

    let options = script_options(arguments, scripts_index);

    print_scripts(&options, palette);

    true
}

fn script_options(arguments: &[String], scripts_index: usize) -> ScriptOptions {
    let mut index = scripts_index + 1;

    let mut path = PathBuf::from(".");

    let mut sort_mode = ScriptSortMode::Count;

    let mut limit = None;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--limit" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --limit requires a number");

                    std::process::exit(2);
                }

                let Some(parsed_limit) = parse_limit_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid limit '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                let limit_text = argument.trim_start_matches("--limit=");

                let Some(parsed_limit) = parse_limit_argument(limit_text) else {
                    eprintln!("noct: invalid limit '{}'", limit_text);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 1;
            }

            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --scripts --sort requires count, interpreter, or name");

                    std::process::exit(2);
                }

                sort_mode = parse_script_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_script_sort_mode(sort_text);

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

    ScriptOptions {
        path,
        sort_mode,
        limit,
    }
}

fn parse_limit_argument(value: &str) -> Option<usize> {
    let limit = value.trim().parse::<usize>().ok()?;

    if limit == 0 {
        return None;
    }

    Some(limit)
}

fn parse_script_sort_mode(value: &str) -> ScriptSortMode {
    match value {
        "count" | "scripts" => ScriptSortMode::Count,

        "interpreter" | "interp" => ScriptSortMode::Interpreter,

        "name" => ScriptSortMode::Name,

        _ => {
            eprintln!("noct: unknown --scripts sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_scripts(options: &ScriptOptions, palette: &AnsiPalette) {
    let path = options.path.as_path();

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --scripts requires a directory");

        std::process::exit(2);
    }

    let scripts = collect_scripts(path);

    let mut groups = group_scripts(scripts);

    sort_script_groups(&mut groups, options.sort_mode);

    print_scripts_header(path, palette);

    if groups.is_empty() {
        println!(
            "  {}no executable scripts found{}",
            palette.report.muted, palette.reset,
        );

        return;
    }

    let total_scripts = groups
        .iter()
        .map(|group| group.scripts.len())
        .sum::<usize>();

    for group in &groups {
        print_script_group(group, options.limit, palette);
    }

    print_scripts_total(total_scripts, groups.len(), palette);
}

fn collect_scripts(path: &Path) -> Vec<ScriptEntry> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", path.display(), error);

            std::process::exit(1);
        }
    };

    let mut scripts = Vec::new();

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

        if !metadata.is_file() {
            continue;
        }

        if metadata.permissions().mode() & 0o111 == 0 {
            continue;
        }

        let Some(command) = script_shebang(&entry_path) else {
            continue;
        };

        let interpreter = interpreter_label(&command);

        scripts.push(ScriptEntry {
            name,
            interpreter,
            command,
        });
    }

    scripts
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

    let command = trimmed.trim_start_matches("#!").trim().to_string();

    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

fn interpreter_label(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return "unknown".to_string();
    }

    let first = parts[0];

    if first.ends_with("/env") || first == "env" {
        if parts.len() >= 2 {
            return command_name(parts[1]);
        }

        return "env".to_string();
    }

    command_name(first)
}

fn command_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| value.to_string())
}

fn group_scripts(scripts: Vec<ScriptEntry>) -> Vec<InterpreterGroup> {
    let mut by_interpreter: BTreeMap<String, Vec<ScriptEntry>> = BTreeMap::new();

    for script in scripts {
        by_interpreter
            .entry(script.interpreter.clone())
            .or_default()
            .push(script);
    }

    let mut groups = Vec::new();

    for (interpreter, mut scripts) in by_interpreter {
        scripts.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

        groups.push(InterpreterGroup {
            interpreter,
            scripts,
        });
    }

    groups
}

fn sort_script_groups(groups: &mut [InterpreterGroup], sort_mode: ScriptSortMode) {
    match sort_mode {
        ScriptSortMode::Count => {
            groups.sort_by(|left, right| {
                right
                    .scripts
                    .len()
                    .cmp(&left.scripts.len())
                    .then_with(|| left.interpreter.cmp(&right.interpreter))
            });
        }

        ScriptSortMode::Interpreter => {
            groups.sort_by(|left, right| left.interpreter.cmp(&right.interpreter));
        }

        ScriptSortMode::Name => {
            groups.sort_by(|left, right| {
                left.scripts
                    .first()
                    .map(|script| script.name.to_lowercase())
                    .unwrap_or_default()
                    .cmp(
                        &right
                            .scripts
                            .first()
                            .map(|script| script.name.to_lowercase())
                            .unwrap_or_default(),
                    )
            });
        }
    }
}

fn print_scripts_header(path: &Path, palette: &AnsiPalette) {
    println!("{}Scripts:{}", palette.report.heading, palette.reset,);

    println!(
        "  {}{}{}",
        palette.report.path,
        path.display(),
        palette.reset,
    );

    println!();
}

fn print_script_group(group: &InterpreterGroup, limit: Option<usize>, palette: &AnsiPalette) {
    let shown = limit
        .map(|limit| group.scripts.len().min(limit))
        .unwrap_or(group.scripts.len());

    println!(
        "{}{}{}  {}{}{} script{}",
        palette.report.total,
        group.interpreter,
        palette.reset,
        palette.report.count,
        group.scripts.len(),
        palette.reset,
        plural_s(group.scripts.len()),
    );

    for script in group.scripts.iter().take(shown) {
        print_script_entry(script, palette);
    }

    let hidden = group.scripts.len().saturating_sub(shown);

    if hidden > 0 {
        println!(
            "  {}({} more...){}",
            palette.report.muted, hidden, palette.reset,
        );
    }

    println!();
}

fn print_script_entry(script: &ScriptEntry, palette: &AnsiPalette) {
    println!(
        "  {}{:<28}{}  {}{}{}",
        palette.report.name,
        script.name,
        palette.reset,
        palette.report.command,
        script.command,
        palette.reset,
    );
}

fn print_scripts_total(total_scripts: usize, total_interpreters: usize, palette: &AnsiPalette) {
    println!(
        "{}Total:{} {}{}{} scripts, {}{}{} interpreters",
        palette.report.total,
        palette.reset,
        palette.report.count,
        total_scripts,
        palette.reset,
        palette.report.count,
        total_interpreters,
        palette.reset,
    );
}

fn plural_s(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
