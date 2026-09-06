use std::time::Duration;

use crate::VERSION;
use crate::config::NoctConfig;

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    File,
    Directory,
    Symlink,
    Special,
}

#[derive(Debug)]
pub enum Filter {
    Extension(String),
    Type(FilterType),
    State(String),
    Danger,
    Orphan,
    Empty,
    Big,
    Tiny,
    GreaterThan(u64),
    LessThan(u64),
    Executable,
    ExecutableWithDirs,
    Writable,
    WorldWritable,
    Suid,
    Sgid,
    Permissions(u32),
    Fresh,
    NewerThan(Duration),
    OlderThan(Duration),
    Stale,
}

#[derive(Debug, Clone, Copy)]
pub enum SortMode {
    Name,
    Size,
    Age,
    Date,
    Extension,
    Unsorted,
}

#[derive(Debug, Clone, Copy)]
pub enum TimeField {
    Modified,
    Accessed,
    Created,
}

#[derive(Debug, Clone, Copy)]
pub enum DisplayMode {
    Grid,
    Long,
    Oneline,
}

#[derive(Debug, Clone, Copy)]
pub enum HiddenMode {
    VisibleOnly,
    AlmostAll,
    All,
}

#[derive(Debug)]
pub struct CliOptions {
    pub paths: Vec<String>,
    pub filters: Vec<Filter>,
    pub sort_mode: SortMode,
    pub show_header: bool,
    pub time_field: TimeField,
    pub display_mode: DisplayMode,
    pub show_icons: bool,
    pub show_age: bool,
    pub show_date: bool,
    pub show_user: bool,
    pub show_permissions: bool,
    pub show_state: bool,
    pub show_state_labels: bool,
    pub grid_padding: usize,
    pub long_gap: &'static str,
    pub hidden_mode: HiddenMode,
    pub show_summary: bool,
    pub tail: Option<usize>,
    pub limit: Option<usize>,
    pub list_directory_itself: bool,
    pub reverse_sort: bool,
    pub output_width: Option<usize>,
}

fn normalize_extension(extension: &str) -> String {
    extension.trim_start_matches('.').to_lowercase()
}

pub fn parse_size_argument(value: &str) -> Option<u64> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    let mut number_part = String::new();

    let mut unit_part = String::new();

    for character in trimmed.chars() {
        if character.is_ascii_digit() {
            if !unit_part.is_empty() {
                return None;
            }

            number_part.push(character);
        } else {
            unit_part.push(character);
        }
    }

    if number_part.is_empty() {
        return None;
    }

    let number = number_part.parse::<u64>().ok()?;

    let multiplier = match unit_part.to_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024_u64 * 1024 * 1024 * 1024,

        _ => {
            return None;
        }
    };

    number.checked_mul(multiplier)
}

fn parse_width_argument(value: &str) -> Option<usize> {
    let width = value.trim().parse::<usize>().ok()?;

    if width == 0 {
        return None;
    }

    Some(width)
}

fn parse_limit_argument(value: &str) -> Option<usize> {
    let limit = value.trim().parse::<usize>().ok()?;

    if limit == 0 {
        return None;
    }

    Some(limit)
}

fn parse_duration_argument(value: &str) -> Option<Duration> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    let mut number_part = String::new();

    let mut unit_part = String::new();

    for character in trimmed.chars() {
        if character.is_ascii_digit() {
            if !unit_part.is_empty() {
                return None;
            }

            number_part.push(character);
        } else {
            unit_part.push(character);
        }
    }

    if number_part.is_empty() || unit_part.is_empty() {
        return None;
    }

    let number = number_part.parse::<u64>().ok()?;

    let seconds_per_unit = match unit_part.to_lowercase().as_str() {
        "m" | "min" | "mins" | "minute" | "minutes" => 60,

        "h" | "hr" | "hrs" | "hour" | "hours" => 60 * 60,

        "d" | "day" | "days" => 60 * 60 * 24,

        "w" | "week" | "weeks" => 60 * 60 * 24 * 7,

        "mo" | "mon" | "month" | "months" => 60 * 60 * 24 * 30,

        "y" | "yr" | "yrs" | "year" | "years" => 60 * 60 * 24 * 365,

        _ => {
            return None;
        }
    };

    number
        .checked_mul(seconds_per_unit)
        .map(Duration::from_secs)
}

fn parse_permission_mode(mode: &str) -> Option<u32> {
    let trimmed = mode.trim_start_matches("0o");

    let trimmed = trimmed.trim_start_matches('0');

    if trimmed.is_empty() || trimmed.len() > 3 {
        return None;
    }

    u32::from_str_radix(trimmed, 8)
        .ok()
        .filter(|value| *value <= 0o777)
}

pub fn expand_short_flags(arguments: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();

    for argument in arguments {
        if argument.starts_with("--") {
            expanded.push(argument.clone());

            continue;
        }

        if !argument.starts_with('-') || argument.len() <= 2 {
            expanded.push(argument.clone());

            continue;
        }

        let mut can_expand = true;

        for short_flag in argument[1..].chars() {
            match short_flag {
                'h' | 'l' | '1' | 'v' | '?' | 'a' | 'A' | 'U' | 'S' | 'w' | 'T' | 'd' | 'D'
                | 'r' => {}

                _ => {
                    can_expand = false;
                    break;
                }
            }
        }

        if can_expand {
            for short_flag in argument[1..].chars() {
                expanded.push(format!("-{}", short_flag));
            }
        } else {
            expanded.push(argument.clone());
        }
    }

    expanded
}

pub fn parse_arguments(arguments: &[String], config: &NoctConfig) -> CliOptions {
    let mut paths = Vec::new();

    let mut filters: Vec<Filter> = Vec::new();

    let mut sort_mode = match config.listing.sort.as_str() {
        "name" => SortMode::Name,

        "size" => SortMode::Size,

        "age" => SortMode::Age,

        "date" => SortMode::Date,

        "extension" | "ext" => SortMode::Extension,

        "unsorted" => SortMode::Unsorted,

        invalid_sort => {
            eprintln!(
                "noct: invalid configured sort mode '{}'; using name",
                invalid_sort,
            );

            SortMode::Name
        }
    };

    let mut show_header = config.display.show_header;

    let mut time_field = TimeField::Modified;

    let mut display_mode = DisplayMode::Grid;

    let mut show_icons = config.display.icons;

    let mut show_user = config.long.show_user;

    let mut show_permissions = config.long.show_permissions;

    let mut show_date = config.long.show_date;

    let mut show_state_labels = false;

    let mut show_age = config.long.show_age;

    let mut grid_padding = 2usize;

    let mut long_gap = "  ";

    let mut show_state = config.long.show_state;

    let mut hidden_mode = HiddenMode::VisibleOnly;

    let mut show_summary = config.display.show_summary;

    let mut limit = if config.listing.limit == 0 {
        None
    } else {
        Some(config.listing.limit)
    };

    let mut tail: Option<usize> = None;

    let mut list_directory_itself = false;

    let mut reverse_sort = config.listing.reverse;

    let mut output_width: Option<usize> = None;

    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            // Version option
            "-v" | "--version" => {
                println!(
                    "noct - noctilucent file system inspection tool and viewer\nVer {}\n\nhttp://github.com/ferusx/noct\n",
                    VERSION
                );

                std::process::exit(0);
            }

            // Header option
            "-h" | "--header" => {
                show_header = true;

                index += 1;
            }

            // Long listing option
            "-l" | "--long" => {
                display_mode = DisplayMode::Long;

                index += 1;
            }

            // One entry per line option
            "-1" | "--oneline" => {
                display_mode = DisplayMode::Oneline;

                index += 1;
            }

            // List the supplied path itself instead of directory contents
            "-d" | "--directory" => {
                list_directory_itself = true;

                index += 1;
            }

            // Reverse the selected sort order
            "-r" | "--reverse" => {
                reverse_sort = true;

                index += 1;
            }

            // Show hidden dotfiles plus . and ..
            "-a" | "--all" => {
                hidden_mode = HiddenMode::All;

                index += 1;
            }

            // Show hidden dotfiles, but not . and ..
            "-A" | "--almost-all" => {
                hidden_mode = HiddenMode::AlmostAll;

                index += 1;
            }

            // Print a summary after the listing
            "--summary" => {
                show_summary = true;

                index += 1;
            }

            // Limit the number of listed entries
            "--limit" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --limit requires a positive number");

                    std::process::exit(2);
                }

                let Some(parsed_limit) = parse_limit_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid limit '{}'", arguments[index + 1],);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 2;
            }

            // Keep the last N entries after filtering and sorting
            "--tail" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --tail requires a positive number");

                    std::process::exit(2);
                }

                let Some(parsed_tail) = parse_limit_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid tail '{}'", arguments[index + 1],);

                    std::process::exit(2);
                };

                tail = Some(parsed_tail);

                index += 2;
            }

            // No colors option
            "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => {
                index += 1;
            }

            // Show file-type icons
            "--icons" => {
                show_icons = true;

                index += 1;
            }

            // Hide the user column in long view
            "--no-user" => {
                show_user = false;

                index += 1;
            }

            // Hide the timestamp column in long view
            "--no-date" => {
                show_date = false;

                index += 1;
            }

            // Hide the age column in long view
            "--no-age" => {
                show_age = false;

                index += 1;
            }

            // Hide the permissions column in long view
            "--no-perms" => {
                show_permissions = false;

                index += 1;
            }

            // Hide the state column in long view
            "--no-state" => {
                show_state = false;

                index += 1;
            }

            // Show detected file state in grid and oneline views
            "--state" => {
                show_state_labels = true;

                index += 1;
            }

            // Show only entries with a specific detected state
            "--only-state" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --only-state requires a state");

                    std::process::exit(2);
                }

                filters.push(Filter::State(arguments[index + 1].clone()));

                show_state_labels = true;

                index += 2;
            }

            // Show only attention-worthy entries
            "--danger" => {
                filters.push(Filter::Danger);

                sort_mode = SortMode::Age;

                show_state_labels = true;

                index += 1;
            }

            // Show broken symlinks and dangling filesystem entries
            "--orphans" => {
                filters.push(Filter::Orphan);

                show_state_labels = true;

                index += 1;
            }

            // Show entries changed within a custom age window
            "--recent" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --recent requires an age, for example 12h or 30d");

                    std::process::exit(2);
                }

                let Some(duration) = parse_duration_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid recent age '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                filters.push(Filter::NewerThan(duration));

                sort_mode = SortMode::Age;

                index += 2;
            }

            // Show entries changed within a custom age window
            "--newer-than" | "--since" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --newer-than requires an age, for example 12h or 30d");

                    std::process::exit(2);
                }

                let Some(duration) = parse_duration_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid age '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                filters.push(Filter::NewerThan(duration));

                sort_mode = SortMode::Age;

                index += 2;
            }

            // Show entries older than a custom age window
            "--older-than" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --older-than requires an age, for example 12h or 30d");

                    std::process::exit(2);
                }

                let Some(duration) = parse_duration_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid age '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                filters.push(Filter::OlderThan(duration));

                sort_mode = SortMode::Age;

                index += 2;
            }

            // Show recently changed entries
            "--fresh" => {
                filters.push(Filter::Fresh);

                sort_mode = SortMode::Age;

                index += 1;
            }

            // Show old entries
            "--stale" => {
                filters.push(Filter::Stale);

                index += 1;
            }

            // Show empty / zero-byte regular files
            "--empty" | "--zero" => {
                filters.push(Filter::Empty);

                index += 1;
            }

            // Show large regular files
            "--big" => {
                filters.push(Filter::Big);

                sort_mode = SortMode::Size;

                index += 1;
            }

            // Show genuinely tiny regular files
            "--tiny" => {
                filters.push(Filter::Tiny);

                sort_mode = SortMode::Size;

                index += 1;
            }

            // Show regular files greater than or equal to a size
            "--greater-than" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --greater-than requires a size");

                    std::process::exit(2);
                }

                let Some(size) = parse_size_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid size '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                filters.push(Filter::GreaterThan(size));

                sort_mode = SortMode::Size;

                index += 2;
            }

            // Show regular files less than or equal to a size
            "--less-than" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --less-than requires a size");

                    std::process::exit(2);
                }

                let Some(size) = parse_size_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid size '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                filters.push(Filter::LessThan(size));

                sort_mode = SortMode::Size;

                index += 2;
            }

            // Show executable entries
            "--executables" => {
                filters.push(Filter::Executable);

                index += 1;
            }

            "--executables-with-dirs" => {
                filters.push(Filter::ExecutableWithDirs);

                index += 1;
            }

            // Show writable entries
            "--writable" => {
                filters.push(Filter::Writable);

                index += 1;
            }

            // Show world-writable entries
            "--world-writable" => {
                filters.push(Filter::WorldWritable);

                show_state_labels = true;

                index += 1;
            }

            // Show setuid entries
            "--suid" => {
                filters.push(Filter::Suid);

                show_state_labels = true;

                index += 1;
            }

            // Show setgid entries
            "--sgid" => {
                filters.push(Filter::Sgid);

                show_state_labels = true;

                index += 1;
            }

            // Match exact permission bits, for example 755 or 0644
            "--permissions" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: --permissions requires a mode");

                    std::process::exit(2);
                }

                let Some(mode) = parse_permission_mode(&arguments[index + 1]) else {
                    eprintln!("noct: invalid permission mode '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                filters.push(Filter::Permissions(mode));

                index += 2;
            }

            // More generous grid columns
            "-w" | "--wide" => {
                grid_padding = 6;
                long_gap = "    ";

                index += 1;
            }

            // Set output width for grid formatting
            "-W" | "--width" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: -W, --width requires a column count");

                    std::process::exit(2);
                }

                let Some(width) = parse_width_argument(&arguments[index + 1]) else {
                    eprintln!("noct: invalid width '{}'", arguments[index + 1]);

                    std::process::exit(2);
                };

                output_width = Some(width);

                index += 2;
            }

            // Filter option
            "--filter" | "-f" => {
                if index + 2 >= arguments.len() {
                    eprintln!("noct: -f, --filter requires a kind and value");

                    std::process::exit(2);
                }

                let filter_kind = arguments[index + 1].as_str();

                let filter_value = arguments[index + 2].clone();

                let filter = match filter_kind {
                    "extension" | "ext" => Filter::Extension(normalize_extension(&filter_value)),

                    "type" => {
                        let filter_type = match filter_value.to_lowercase().as_str() {
                            "file" => FilterType::File,

                            "directory" | "dir" => FilterType::Directory,

                            "symlink" | "link" => FilterType::Symlink,

                            "special" => FilterType::Special,

                            _ => {
                                eprintln!(
                                    "noct: unknown type filter value '{}'; expected file, directory, symlink, or special",
                                    filter_value,
                                );

                                std::process::exit(2);
                            }
                        };

                        Filter::Type(filter_type)
                    }

                    _ => {
                        eprintln!("noct: unknown filter kind '{}'", filter_kind);

                        std::process::exit(2);
                    }
                };

                filters.push(filter);

                index += 3;
            }

            // Sort by size, largest first
            "-S" => {
                sort_mode = SortMode::Size;

                index += 1;
            }

            // Sort option
            "--sort" | "-s" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: -s, --sort requires a mode");

                    std::process::exit(2);
                }

                let sort_value = arguments[index + 1].as_str();

                sort_mode = match sort_value {
                    "name" => SortMode::Name,
                    "size" => SortMode::Size,
                    "age" => SortMode::Age,
                    "date" => SortMode::Date,
                    "extension" | "ext" => SortMode::Extension,

                    _ => {
                        eprintln!("noct: unknown sort mode '{}'", sort_value);

                        std::process::exit(2);
                    }
                };

                index += 2;
            }

            // Do not sort; keep filesystem directory order
            "-U" | "--unsorted" => {
                sort_mode = SortMode::Unsorted;

                index += 1;
            }

            // Time opton
            "-t" | "--time" => {
                if index + 1 >= arguments.len() {
                    eprintln!("noct: -t, --time requires a field");

                    std::process::exit(2);
                }

                let time_value = arguments[index + 1].as_str();

                time_field = match time_value {
                    "modified" | "mod" => TimeField::Modified,
                    "accessed" | "access" => TimeField::Accessed,
                    "created" | "create" => TimeField::Created,

                    _ => {
                        eprintln!("noct: unknown time field '{}'", time_value);

                        std::process::exit(2);
                    }
                };

                index += 2;
            }

            argument if argument.starts_with("--width=") => {
                let width_text = argument.trim_start_matches("--width=");

                let Some(width) = parse_width_argument(width_text) else {
                    eprintln!("noct: invalid width '{}'", width_text);

                    std::process::exit(2);
                };

                output_width = Some(width);

                index += 1;
            }

            argument if argument.starts_with("--limit=") => {
                let value = argument.trim_start_matches("--limit=");

                let Some(parsed_limit) = parse_limit_argument(value) else {
                    eprintln!("noct: invalid limit '{}'", value,);

                    std::process::exit(2);
                };

                limit = Some(parsed_limit);

                index += 1;
            }

            argument if argument.starts_with("--tail=") => {
                let value = argument.trim_start_matches("--tail=");

                let Some(parsed_tail) = parse_limit_argument(value) else {
                    eprintln!("noct: invalid tail '{}'", value,);

                    std::process::exit(2);
                };

                tail = Some(parsed_tail);

                index += 1;
            }

            argument if argument.starts_with("--since=") => {
                let value = argument.trim_start_matches("--since=");

                let Some(duration) = parse_duration_argument(value) else {
                    eprintln!("noct: invalid age '{}'", value,);

                    std::process::exit(2);
                };

                filters.push(Filter::NewerThan(duration));

                sort_mode = SortMode::Age;

                index += 1;
            }

            argument => {
                paths.push(argument.to_string());

                index += 1;
            }
        }
    }

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    CliOptions {
        paths,
        filters,
        sort_mode,
        show_header,
        time_field,
        display_mode,
        show_icons,
        show_permissions,
        show_user,
        show_age,
        show_date,
        show_state,
        show_state_labels,
        grid_padding,
        long_gap,
        hidden_mode,
        show_summary,
        limit,
        tail,
        list_directory_itself,
        reverse_sort,
        output_width,
    }
}
