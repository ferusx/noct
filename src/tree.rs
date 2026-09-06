use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::args::SortMode;
use crate::classify::{FilesystemColorClass, classify_entry, standard_filesystem_color};
use crate::config::FilesystemColorsMode;
use crate::render::icon_for_class;
use crate::themes::AnsiPalette;

const BRANCH: &str = "├── ";

const LAST_BRANCH: &str = "└── ";

const VERTICAL: &str = "│   ";

const EMPTY_PREFIX: &str = "    ";

#[derive(Debug)]
struct TreeOptions {
    path: PathBuf,
    root_label: String,
    show_hidden: bool,
    dirs_only: bool,
    max_depth: Option<usize>,
    sort_mode: SortMode,
    show_summary: bool,
    show_icons: bool,
}

#[derive(Debug)]
struct TreeEntry {
    path: PathBuf,
    name: String,
    metadata: fs::Metadata,
}

#[derive(Debug, Default)]
struct TreeCounts {
    files: usize,
    directories: usize,
    links: usize,
    special: usize,
}

pub fn handle_tree_request(
    arguments: &[String],
    palette: &AnsiPalette,
    filesystem_colors: FilesystemColorsMode,
) -> bool {
    let has_tree = arguments
        .iter()
        .any(|argument| argument == "-T" || argument == "--tree");

    if !has_tree {
        return false;
    }

    let options = tree_options(arguments);

    print_tree(&options, palette, filesystem_colors);

    true
}

fn tree_options(arguments: &[String]) -> TreeOptions {
    let mut path = PathBuf::from("..");

    let mut root_label = ".".to_string();

    let mut explicit_path = false;

    let mut show_hidden = false;

    let mut dirs_only = false;

    let mut max_depth = None;

    let mut sort_mode = SortMode::Name;

    let mut show_summary = false;

    let mut show_icons = false;

    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "-T" | "--tree" => {
                index += 1;
            }

            "-a" | "--all" | "-A" | "--almost-all" => {
                show_hidden = true;

                index += 1;
            }

            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            "--icons" => {
                show_icons = true;

                index += 1;
            }

            "-D" | "--dirs-only" => {
                dirs_only = true;

                index += 1;
            }

            "--summary" => {
                show_summary = true;

                index += 1;
            }

            "-L" | "--level" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: -L, --level requires a depth");

                    std::process::exit(2);
                }

                let Some(depth) = parse_depth(&arguments[index + 1]) else {
                    eprintln!("noct: invalid tree depth '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                max_depth = Some(depth);

                index += 2;
            }

            argument if argument.starts_with("--level=") => {
                let depth_text = argument.trim_start_matches("--level=");

                let Some(depth) = parse_depth(depth_text) else {
                    eprintln!("noct: invalid tree depth '{}'", depth_text);

                    std::process::exit(2);
                };

                max_depth = Some(depth);

                index += 1;
            }

            "-s" | "--sort" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --tree --sort requires name, size, age, or extension");

                    std::process::exit(2);
                }

                sort_mode = parse_tree_sort_mode(&arguments[index + 1]);

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                let sort_text = argument.trim_start_matches("--sort=");

                sort_mode = parse_tree_sort_mode(sort_text);

                index += 1;
            }

            "-U" | "--unsorted" => {
                sort_mode = SortMode::Unsorted;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                index += 1;
            }

            argument => {
                path = PathBuf::from(argument);

                root_label = argument.to_string();

                explicit_path = true;

                index += 1;
            }
        }
    }

    if !explicit_path {
        root_label = ".".to_string();
    }

    TreeOptions {
        path,
        root_label,
        show_hidden,
        dirs_only,
        max_depth,
        sort_mode,
        show_summary,
        show_icons,
    }
}

fn parse_depth(value: &str) -> Option<usize> {
    value.trim().parse::<usize>().ok()
}

fn parse_tree_sort_mode(value: &str) -> SortMode {
    match value {
        "name" => SortMode::Name,

        "size" => SortMode::Size,

        "age" | "time" | "modified" => SortMode::Age,

        "extension" | "ext" => SortMode::Extension,

        _ => {
            eprintln!("noct: unknown --tree sort mode '{}'", value);

            std::process::exit(2);
        }
    }
}

fn print_tree(
    options: &TreeOptions,
    palette: &AnsiPalette,
    filesystem_colors: FilesystemColorsMode,
) {
    let metadata = match fs::metadata(&options.path) {
        Ok(metadata) => metadata,

        Err(error) => {
            eprintln!("noct: unable to read {}: {}", options.path.display(), error,);

            std::process::exit(1);
        }
    };

    if !metadata.is_dir() {
        eprintln!("noct: --tree requires a directory");

        std::process::exit(2);
    }

    print_root(&options.root_label, palette, filesystem_colors);

    let mut counts = TreeCounts::default();

    let may_descend = options.max_depth.map(|depth| depth > 0).unwrap_or(true);

    if may_descend {
        print_tree_directory(
            &options.path,
            1,
            &[],
            options,
            &mut counts,
            palette,
            filesystem_colors,
        );
    }

    if options.show_summary {
        println!();

        print_tree_summary(&counts, palette);
    }
}

fn print_root(root_label: &str, palette: &AnsiPalette, filesystem_colors: FilesystemColorsMode) {
    let color = match filesystem_colors {
        FilesystemColorsMode::Directories => "\x1b[38;2;80;155;235m",

        _ => &palette.names.directory,
    };

    println!("{}{}{}", color, root_label, palette.reset);
}

fn print_tree_directory(
    directory: &Path,
    depth: usize,
    ancestor_has_more: &[bool],
    options: &TreeOptions,
    counts: &mut TreeCounts,
    palette: &AnsiPalette,
    filesystem_colors: FilesystemColorsMode,
) {
    if let Some(max_depth) = options.max_depth {
        if depth > max_depth {
            return;
        }
    }

    let entries = read_tree_entries(directory, options);

    let entry_count = entries.len();

    for (index, entry) in entries.iter().enumerate() {
        let is_last = index + 1 == entry_count;

        print_tree_entry(
            entry,
            ancestor_has_more,
            is_last,
            palette,
            filesystem_colors,
            options.show_icons,
        );

        count_tree_entry(entry, counts);

        if !entry.metadata.is_dir() {
            continue;
        }

        if let Some(max_depth) = options.max_depth {
            if depth >= max_depth {
                continue;
            }
        }

        let mut child_ancestor_state = ancestor_has_more.to_vec();

        child_ancestor_state.push(!is_last);

        print_tree_directory(
            &entry.path,
            depth + 1,
            &child_ancestor_state,
            options,
            counts,
            palette,
            filesystem_colors,
        );
    }
}

fn read_tree_entries(directory: &Path, options: &TreeOptions) -> Vec<TreeEntry> {
    let read_dir = match fs::read_dir(directory) {
        Ok(read_dir) => read_dir,

        Err(_) => {
            return Vec::new();
        }
    };

    let mut entries = Vec::new();

    for result in read_dir {
        let Ok(dir_entry) = result else {
            continue;
        };

        let name = dir_entry.file_name().to_string_lossy().to_string();

        if !options.show_hidden && name.starts_with('.') {
            continue;
        }

        let path = dir_entry.path();

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,

            Err(_) => {
                continue;
            }
        };

        if options.dirs_only && !is_directory_entry(&path, &metadata) {
            continue;
        }

        entries.push(TreeEntry {
            path,
            name,
            metadata,
        });
    }

    sort_tree_entries(&mut entries, options.sort_mode);

    entries
}

fn is_directory_entry(path: &Path, metadata: &fs::Metadata) -> bool {
    if metadata.is_dir() {
        return true;
    }

    if !metadata.file_type().is_symlink() {
        return false;
    }

    fs::metadata(path)
        .map(|target_metadata| target_metadata.is_dir())
        .unwrap_or(false)
}

fn sort_tree_entries(entries: &mut [TreeEntry], sort_mode: SortMode) {
    match sort_mode {
        SortMode::Name => {
            entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        }

        SortMode::Size => {
            entries.sort_by(|left, right| {
                right
                    .metadata
                    .len()
                    .cmp(&left.metadata.len())
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        SortMode::Age => {
            entries.sort_by(|left, right| {
                let left_modified = modified_time(&left.metadata);

                let right_modified = modified_time(&right.metadata);

                right_modified
                    .cmp(&left_modified)
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        SortMode::Date => {
            entries.sort_by(|left, right| {
                let left_modified = modified_time(&left.metadata);

                let right_modified = modified_time(&right.metadata);

                right_modified
                    .cmp(&left_modified)
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        SortMode::Extension => {
            entries.sort_by(|left, right| {
                extension_for_name(&left.name)
                    .cmp(&extension_for_name(&right.name))
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            });
        }

        SortMode::Unsorted => {}
    }
}

fn modified_time(metadata: &fs::Metadata) -> Option<SystemTime> {
    metadata.modified().ok()
}

fn extension_for_name(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn print_tree_entry(
    entry: &TreeEntry,
    ancestor_has_more: &[bool],
    is_last: bool,
    palette: &AnsiPalette,
    filesystem_colors: FilesystemColorsMode,
    show_icons: bool,
) {
    print!("{}", palette.permissions.missing,);

    for has_more in ancestor_has_more {
        if *has_more {
            print!("{}", VERTICAL,);
        } else {
            print!("{}", EMPTY_PREFIX,);
        }
    }

    if is_last {
        print!("{}", LAST_BRANCH,);
    } else {
        print!("{}", BRANCH,);
    }

    print!("{}", palette.reset,);

    println!(
        "{}",
        render_tree_name(entry, palette, filesystem_colors, show_icons),
    );
}

fn render_tree_name(
    entry: &TreeEntry,
    palette: &AnsiPalette,
    filesystem_colors: FilesystemColorsMode,
    show_icons: bool,
) -> String {
    let file_type = entry.metadata.file_type();

    let is_symlink = file_type.is_symlink();

    let is_executable =
        entry.metadata.is_file() && entry.metadata.permissions().mode() & 0o111 != 0;

    let suffix = if entry.metadata.is_dir() {
        "/"
    } else if is_symlink {
        "@"
    } else if is_executable {
        "*"
    } else {
        ""
    };

    let color_class = classify_entry(&entry.path, &entry.name, &entry.metadata).color_class();

    let color = match filesystem_colors {
        FilesystemColorsMode::Theme => palette.classification_color(color_class),

        FilesystemColorsMode::Standard => standard_filesystem_color(color_class),

        FilesystemColorsMode::Directories => {
            if matches!(color_class, FilesystemColorClass::Directory) {
                "\x1b[38;2;80;155;235m"
            } else {
                ""
            }
        }
    };

    let class = classify_entry(&entry.path, &entry.name, &entry.metadata);

    let icon = if show_icons && !palette.ansi16 {
        format!("{} ", icon_for_class(class))
    } else {
        String::new()
    };

    format!("{}{}{}{}{}", color, icon, entry.name, suffix, palette.reset,)
}

fn count_tree_entry(entry: &TreeEntry, counts: &mut TreeCounts) {
    let file_type = entry.metadata.file_type();

    if file_type.is_symlink() {
        counts.links += 1;
    } else if entry.metadata.is_dir() {
        counts.directories += 1;
    } else if entry.metadata.is_file() {
        counts.files += 1;
    } else {
        counts.special += 1;
    }
}

fn print_tree_summary(counts: &TreeCounts, palette: &AnsiPalette) {
    let total = counts.files + counts.directories + counts.links + counts.special;

    println!(
        "{}{}{} entries: {}{}{} files, {}{}{} dirs, {}{}{} links, {}{}{} special",
        palette.summary.entries,
        total,
        palette.reset,
        palette.summary.files,
        counts.files,
        palette.reset,
        palette.summary.directories,
        counts.directories,
        palette.reset,
        palette.summary.symlinks,
        counts.links,
        palette.reset,
        palette.summary.special,
        counts.special,
        palette.reset,
    );
}
