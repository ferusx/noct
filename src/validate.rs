use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandMode {
    PackageBins,
    Owns,
    Inspect,
    LinksTo,
    Duplicates,
    Du,
    Map,
    Timeline,
    Extensions,
    Permissions,
    Audit,
    Scripts,
    Names,
    Owners,
    Profile,
    Portable,
    Graveyard,
    Leaves,
    Mounts,
    EmptyDirs,
    Hot,
    Cold,
    Tree,
}

impl CommandMode {
    fn label(self) -> &'static str {
        match self {
            CommandMode::PackageBins => "--package-bins",
            CommandMode::Owns => "--owns",
            CommandMode::Inspect => "--inspect",
            CommandMode::LinksTo => "--link-to",
            CommandMode::Duplicates => "--duplicates",
            CommandMode::Du => "--du",
            CommandMode::Map => "--map",
            CommandMode::Timeline => "--timeline",
            CommandMode::Extensions => "--exts",
            CommandMode::Permissions => "--perms",
            CommandMode::Audit => "--audit",
            CommandMode::Scripts => "--scripts",
            CommandMode::Names => "--names",
            CommandMode::Owners => "--owners",
            CommandMode::Profile => "--profile",
            CommandMode::Portable => "--portable",
            CommandMode::Graveyard => "--graveyard",
            CommandMode::Leaves => "--leaves",
            CommandMode::Mounts => "--mounts",
            CommandMode::EmptyDirs => "--empty-dirs",
            CommandMode::Hot => "--hot",
            CommandMode::Cold => "--cold",
            CommandMode::Tree => "--tree",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SelectedMode {
    mode: CommandMode,
    index: usize,
}

pub fn validate_arguments(arguments: &[String]) {
    validate_repeated_regular_options(arguments);

    let Some(selected_mode) = detect_selected_mode(arguments) else {
        return;
    };

    validate_regular_listing_only_options(arguments, selected_mode.mode);

    match selected_mode.mode {
        CommandMode::PackageBins | CommandMode::Owns | CommandMode::Inspect => {
            validate_solo_target_mode(arguments, selected_mode);
        }

        CommandMode::LinksTo => {
            validate_solo_target_mode(arguments, selected_mode);
        }

        CommandMode::Duplicates => {
            validate_duplicates(arguments, selected_mode);
        }

        CommandMode::Du => {
            validate_du(arguments, selected_mode);
        }

        CommandMode::Map => {
            validate_map(arguments, selected_mode);
        }

        CommandMode::Extensions => {
            validate_extensions(arguments, selected_mode);
        }

        CommandMode::Permissions => {
            validate_sortable_directory_mode(arguments, selected_mode);
        }

        CommandMode::Scripts => {
            validate_scripts(arguments, selected_mode);
        }

        CommandMode::Audit => {
            validate_optional_path_mode(arguments, selected_mode);
        }

        CommandMode::Names => {
            validate_names(arguments, selected_mode);
        }

        CommandMode::Owners => {
            validate_sortable_directory_mode(arguments, selected_mode);
        }

        CommandMode::Profile | CommandMode::Portable => {
            validate_optional_path_mode(arguments, selected_mode);
        }

        CommandMode::Graveyard => {
            validate_graveyard(arguments, selected_mode);
        }

        CommandMode::Leaves => {
            validate_leaves(arguments, selected_mode);
        }

        CommandMode::Mounts => {
            validate_mounts(arguments, selected_mode);
        }

        CommandMode::EmptyDirs => {
            validate_empty_dirs(arguments, selected_mode);
        }

        CommandMode::Hot => {
            validate_hot(arguments, selected_mode);
        }

        CommandMode::Cold => {
            validate_cold(arguments, selected_mode);
        }

        CommandMode::Tree => {
            validate_tree(arguments, selected_mode);
        }

        _ => {
            // Conflict detection is already active for these modes.
            // Their detailed allowlists will be added after inspection.
        }
    }
}

fn validate_repeated_regular_options(arguments: &[String]) {
    let mut seen_options = HashSet::new();

    for argument in arguments {
        let Some(option_name) = canonical_regular_option(argument) else {
            continue;
        };

        if !seen_options.insert(option_name) {
            fail(format!("{} may only be specified once", option_name,));
        }
    }
}

fn validate_scripts(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_sort = false;

    let mut saw_limit = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            "-s" | "--sort" => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn canonical_regular_option(argument: &str) -> Option<&'static str> {
    match argument {
        "-h" | "--header" => Some("--header"),

        "-l" | "--long" => Some("--long"),

        "-1" | "--oneline" => Some("--oneline"),

        "-d" | "--directory" => Some("--directory"),

        "-r" | "--reverse" => Some("--reverse"),

        "-a" | "--all" => Some("--all"),

        "-A" | "--almost-all" => Some("--almost-all"),

        "--summary" => Some("--summary"),

        "--no-color" | "--no-colour" | "--no-colors" | "--no-colours" => Some("--no-color"),

        "--no-user" => Some("--no-user"),

        "--no-date" => Some("--no-date"),

        "--no-age" => Some("--no-age"),

        "--no-perms" => Some("--no-perms"),

        "--no-state" => Some("--no-state"),

        "--state" => Some("--state"),

        "--only-state" => Some("--only-state"),

        "--danger" => Some("--danger"),

        "--orphans" => Some("--orphans"),

        "--recent" => Some("--recent"),

        "--newer-than" => Some("--newer-than"),

        "--older-than" => Some("--older-than"),

        "--fresh" => Some("--fresh"),

        "--stale" => Some("--stale"),

        "--empty" | "--zero" => Some("--empty"),

        "--big" => Some("--big"),

        "--tiny" => Some("--tiny"),

        "--greater-than" => Some("--greater-than"),

        "--less-than" => Some("--less-than"),

        "--executables" => Some("--executables"),

        "--executables-with-dirs" => Some("--executables-with-dirs"),

        "--writable" => Some("--writable"),

        "--world-writable" => Some("--world-writable"),

        "--suid" => Some("--suid"),

        "--sgid" => Some("--sgid"),

        "--permissions" => Some("--permissions"),

        "--tail" => Some("--tail"),

        argument if argument.starts_with("--tail=") => Some("--tail"),

        "--limit" => Some("--limit"),

        argument if argument.starts_with("--limit=") => Some("--limit"),

        "-w" | "--wide" => Some("--wide"),

        "-W" | "--width" => Some("--width"),

        argument if argument.starts_with("--width=") => Some("--width"),

        "-f" | "--filter" => Some("--filter"),

        "-S" | "-s" | "--sort" => Some("--sort"),

        argument if argument.starts_with("--sort=") => Some("--sort"),

        "-U" | "--unsorted" => Some("--unsorted"),

        "-t" | "--time" => Some("--time"),

        _ => None,
    }
}

fn detect_selected_mode(arguments: &[String]) -> Option<SelectedMode> {
    let mut selected: Option<SelectedMode> = None;

    for (index, argument) in arguments.iter().enumerate() {
        let Some(mode) = command_mode_for_argument(argument) else {
            continue;
        };

        if let Some(previous) = selected {
            if previous.mode == mode {
                fail(format!("{} may only be specified once", mode.label(),));
            }

            fail(format!(
                "{} cannot be combined with {}",
                previous.mode.label(),
                mode.label(),
            ));
        }

        selected = Some(SelectedMode { mode, index });
    }

    selected
}

fn validate_regular_listing_only_options(arguments: &[String], mode: CommandMode) {
    for argument in arguments {
        match argument.as_str() {
            "-d" | "--directory" => {
                fail(format!(
                    "{} is not supported with {}",
                    argument,
                    mode.label(),
                ));
            }

            "-r" | "--reverse" if mode != CommandMode::Cold => {
                fail(format!(
                    "{} is not supported with {}",
                    argument,
                    mode.label(),
                ));
            }

            _ => {}
        }
    }
}

fn command_mode_for_argument(argument: &str) -> Option<CommandMode> {
    match argument {
        "--package-bins" => Some(CommandMode::PackageBins),

        "--owns" => Some(CommandMode::Owns),

        "--inspect" => Some(CommandMode::Inspect),

        "--duplicates" => Some(CommandMode::Duplicates),

        "--du" => Some(CommandMode::Du),

        "--map" => Some(CommandMode::Map),

        "--timeline" => Some(CommandMode::Timeline),

        "--exts" => Some(CommandMode::Extensions),

        "--perms" => Some(CommandMode::Permissions),

        "--audit" => Some(CommandMode::Audit),

        "--scripts" => Some(CommandMode::Scripts),

        "--names" => Some(CommandMode::Names),

        "--owners" => Some(CommandMode::Owners),

        "--profile" => Some(CommandMode::Profile),

        "--portable" => Some(CommandMode::Portable),

        "--graveyard" => Some(CommandMode::Graveyard),

        "--leaves" => Some(CommandMode::Leaves),

        "--mounts" => Some(CommandMode::Mounts),

        "--empty-dirs" => Some(CommandMode::EmptyDirs),

        "--link-to" => Some(CommandMode::LinksTo),

        "--hot" => Some(CommandMode::Hot),

        "--cold" => Some(CommandMode::Cold),

        "-T" | "--tree" => Some(CommandMode::Tree),

        _ => None,
    }
}

fn validate_solo_target_mode(arguments: &[String], selected: SelectedMode) {
    let target_index = required_target_index(arguments, selected);

    for (index, argument) in arguments.iter().enumerate() {
        if index == selected.index || index == target_index || is_global_color_option(argument) {
            continue;
        }

        reject_argument(argument, selected.mode);
    }
}

fn validate_duplicates(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_recursive = false;

    let mut saw_min_size = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "-R" | "--recursive" => {
                if saw_recursive {
                    fail("--recursive may only be specified once with --duplicates");
                }

                saw_recursive = true;

                index += 1;
            }

            "--min-size" => {
                if saw_min_size {
                    fail("--min-size may only be specified once with --duplicates");
                }

                require_option_value(arguments, index, "--min-size");

                saw_min_size = true;

                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                if saw_min_size {
                    fail("--min-size may only be specified once with --duplicates");
                }

                let value = argument.trim_start_matches("--min-size=");

                if value.is_empty() {
                    fail("--min-size requires a size");
                }

                saw_min_size = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                if path_count > 1 {
                    fail(format!(
                        "unexpected argument '{}' with --duplicates",
                        argument,
                    ));
                }

                index += 1;
            }
        }
    }
}

fn validate_optional_path_mode(arguments: &[String], selected: SelectedMode) {
    let mut path_count = 0usize;

    for (index, argument) in arguments.iter().enumerate() {
        if index == selected.index || is_global_color_option(argument) {
            continue;
        }

        if argument.starts_with('-') {
            reject_argument(argument, selected.mode);
        }

        path_count += 1;

        if path_count > 1 {
            fail(format!(
                "unexpected argument '{}' with {}",
                argument,
                selected.mode.label(),
            ));
        }
    }
}

fn validate_map(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_du(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_leaves(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    let mut saw_min_files = false;

    let mut saw_min_size = false;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            "--min-files" => {
                reject_repeated_option(saw_min_files, "--min-files", selected.mode);

                require_option_value(arguments, index, "--min-files");

                saw_min_files = true;

                index += 2;
            }

            argument if argument.starts_with("--min-files=") => {
                reject_repeated_option(saw_min_files, "--min-files", selected.mode);

                require_equals_value(argument, "--min-files=", "--min-files requires a number");

                saw_min_files = true;

                index += 1;
            }

            "--min-size" => {
                reject_repeated_option(saw_min_size, "--min-size", selected.mode);

                require_option_value(arguments, index, "--min-size");

                saw_min_size = true;

                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                reject_repeated_option(saw_min_size, "--min-size", selected.mode);

                require_equals_value(argument, "--min-size=", "--min-size requires a size");

                saw_min_size = true;

                index += 1;
            }

            "-s" | "--sort" => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_mounts(arguments: &[String], selected: SelectedMode) {
    let mut path_count = 0usize;

    let mut saw_all = false;

    for (index, argument) in arguments.iter().enumerate() {
        if index == selected.index || is_global_color_option(argument) {
            continue;
        }

        match argument.as_str() {
            "--all" => {
                reject_repeated_option(saw_all, "--all", selected.mode);

                saw_all = true;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);
            }
        }
    }
}

fn validate_empty_dirs(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    let mut saw_min_depth = false;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            "--min-depth" => {
                reject_repeated_option(saw_min_depth, "--min-depth", selected.mode);

                require_option_value(arguments, index, "--min-depth");

                saw_min_depth = true;

                index += 2;
            }

            argument if argument.starts_with("--min-depth=") => {
                reject_repeated_option(saw_min_depth, "--min-depth", selected.mode);

                require_equals_value(argument, "--min-depth=", "--min-depth requires a number");

                saw_min_depth = true;

                index += 1;
            }

            "-s" | "--sort" => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_hot(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    let mut saw_since = false;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            "--since" => {
                reject_repeated_option(saw_since, "--since", selected.mode);

                require_option_value(arguments, index, "--since");

                saw_since = true;

                index += 2;
            }

            argument if argument.starts_with("--since=") => {
                reject_repeated_option(saw_since, "--since", selected.mode);

                require_equals_value(argument, "--since=", "--since requires an age");

                saw_since = true;

                index += 1;
            }

            "-s" | "--sort" => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_cold(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    let mut saw_older_than = false;

    let mut saw_min_size = false;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                reject_repeated_option(saw_limit, "--limit", selected.mode);

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            "--older-than" => {
                reject_repeated_option(saw_older_than, "--older-than", selected.mode);

                require_option_value(arguments, index, "--older-than");

                saw_older_than = true;

                index += 2;
            }

            argument if argument.starts_with("--older-than=") => {
                reject_repeated_option(saw_older_than, "--older-than", selected.mode);

                require_equals_value(argument, "--older-than=", "--older-than requires an age");

                saw_older_than = true;

                index += 1;
            }

            "--min-size" => {
                reject_repeated_option(saw_min_size, "--min-size", selected.mode);

                require_option_value(arguments, index, "--min-size");

                saw_min_size = true;

                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                reject_repeated_option(saw_min_size, "--min-size", selected.mode);

                require_equals_value(argument, "--min-size=", "--min-size requires a size");

                saw_min_size = true;

                index += 1;
            }

            "-s" | "--sort" => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            "-r" | "--reverse" => {
                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_tree(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_level = false;

    let mut saw_all = false;

    let mut saw_almost_all = false;

    let mut saw_dirs_only = false;

    let mut saw_summary = false;

    let mut saw_sort = false;

    let mut saw_unsorted = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "-a" | "--all" => {
                reject_repeated_option(saw_all, "--all", selected.mode);

                if saw_almost_all {
                    fail("--all cannot be combined with --almost-all in --tree mode");
                }

                saw_all = true;

                index += 1;
            }

            "-A" | "--almost-all" => {
                reject_repeated_option(saw_almost_all, "--almost-all", selected.mode);

                if saw_all {
                    fail("--all cannot be combined with --almost-all in --tree mode");
                }

                saw_almost_all = true;

                index += 1;
            }

            "--icons" => {
                index += 1;
            }

            "-D" | "--dirs-only" => {
                reject_repeated_option(saw_dirs_only, "--dirs-only", selected.mode);

                saw_dirs_only = true;

                index += 1;
            }

            "--summary" => {
                reject_repeated_option(saw_summary, "--summary", selected.mode);

                saw_summary = true;

                index += 1;
            }

            "-L" | "--level" => {
                reject_repeated_option(saw_level, "--level", selected.mode);

                require_option_value(arguments, index, "--level");

                saw_level = true;

                index += 2;
            }

            argument if argument.starts_with("--level=") => {
                reject_repeated_option(saw_level, "--level", selected.mode);

                require_equals_value(argument, "--level=", "--level requires a depth");

                saw_level = true;

                index += 1;
            }

            "-s" | "--sort" => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                if saw_unsorted {
                    fail("--sort cannot be combined with --unsorted in --tree mode");
                }

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                reject_repeated_option(saw_sort, "--sort", selected.mode);

                if saw_unsorted {
                    fail("--sort cannot be combined with --unsorted in --tree mode");
                }

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            "-U" | "--unsorted" => {
                reject_repeated_option(saw_unsorted, "--unsorted", selected.mode);

                if saw_sort {
                    fail("--sort cannot be combined with --unsorted in --tree mode");
                }

                saw_unsorted = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                reject_extra_path(path_count, argument, selected.mode);

                index += 1;
            }
        }
    }
}

fn validate_extensions(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "-s" | "--sort" => {
                if saw_sort {
                    fail("--sort may only be specified once with --exts");
                }

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                if saw_sort {
                    fail("--sort may only be specified once with --exts");
                }

                let value = argument.trim_start_matches("--sort=");

                if value.is_empty() {
                    fail("--sort requires a field");
                }

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                if path_count > 1 {
                    fail(format!("unexpected argument '{}' with --exts", argument,));
                }

                index += 1;
            }
        }
    }
}

fn validate_sortable_directory_mode(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "-s" | "--sort" => {
                if saw_sort {
                    fail(format!(
                        "--sort may only be specified once with {}",
                        selected.mode.label(),
                    ));
                }

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                if saw_sort {
                    fail(format!(
                        "--sort may only be specified once with {}",
                        selected.mode.label(),
                    ));
                }

                let value = argument.trim_start_matches("--sort=");

                if value.is_empty() {
                    fail("--sort requires a field");
                }

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                if path_count > 1 {
                    fail(format!(
                        "unexpected argument '{}' with {}",
                        argument,
                        selected.mode.label(),
                    ));
                }

                index += 1;
            }
        }
    }
}

fn validate_names(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_limit = false;

    let mut saw_since = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "--limit" => {
                if saw_limit {
                    fail("--limit may only be specified once with --names");
                }

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                if saw_limit {
                    fail("--limit may only be specified once with --names");
                }

                let value = argument.trim_start_matches("--limit=");

                if value.is_empty() {
                    fail("--limit requires a number");
                }

                saw_limit = true;

                index += 1;
            }

            "--since" => {
                if saw_since {
                    fail("--since may only be specified once with --names");
                }

                require_option_value(arguments, index, "--since");

                saw_since = true;

                index += 2;
            }

            argument if argument.starts_with("--since=") => {
                if saw_since {
                    fail("--since may only be specified once with --names");
                }

                let value = argument.trim_start_matches("--since=");

                if value.is_empty() {
                    fail("--since requires an age");
                }

                saw_since = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                if path_count > 1 {
                    fail(format!("unexpected argument '{}' with --names", argument,));
                }

                index += 1;
            }
        }
    }
}

fn validate_graveyard(arguments: &[String], selected: SelectedMode) {
    let mut index = 0;

    let mut path_count = 0usize;

    let mut saw_recursive = false;

    let mut saw_limit = false;

    let mut saw_min_size = false;

    let mut saw_older_than = false;

    let mut saw_kind = false;

    let mut saw_sort = false;

    while index < arguments.len() {
        let argument = arguments[index].as_str();

        if index == selected.index || is_global_color_option(argument) {
            index += 1;

            continue;
        }

        match argument {
            "-R" | "--recursive" => {
                if saw_recursive {
                    fail("--recursive may only be specified once with --graveyard");
                }

                saw_recursive = true;

                index += 1;
            }

            "--limit" => {
                if saw_limit {
                    fail("--limit may only be specified once with --graveyard");
                }

                require_option_value(arguments, index, "--limit");

                saw_limit = true;

                index += 2;
            }

            argument if argument.starts_with("--limit=") => {
                if saw_limit {
                    fail("--limit may only be specified once with --graveyard");
                }

                require_equals_value(argument, "--limit=", "--limit requires a number");

                saw_limit = true;

                index += 1;
            }

            "--min-size" => {
                if saw_min_size {
                    fail("--min-size may only be specified once with --graveyard");
                }

                require_option_value(arguments, index, "--min-size");

                saw_min_size = true;

                index += 2;
            }

            argument if argument.starts_with("--min-size=") => {
                if saw_min_size {
                    fail("--min-size may only be specified once with --graveyard");
                }

                require_equals_value(argument, "--min-size=", "--min-size requires a size");

                saw_min_size = true;

                index += 1;
            }

            "--older-than" => {
                if saw_older_than {
                    fail("--older-than may only be specified once with --graveyard");
                }

                require_option_value(arguments, index, "--older-than");

                saw_older_than = true;

                index += 2;
            }

            argument if argument.starts_with("--older-than=") => {
                if saw_older_than {
                    fail("--older-than may only be specified once with --graveyard");
                }

                require_equals_value(argument, "--older-than=", "--older-than requires an age");

                saw_older_than = true;

                index += 1;
            }

            "--kind" => {
                if saw_kind {
                    fail("--kind may only be specified once with --graveyard");
                }

                require_option_value(arguments, index, "--kind");

                saw_kind = true;

                index += 2;
            }

            argument if argument.starts_with("--kind=") => {
                if saw_kind {
                    fail("--kind may only be specified once with --graveyard");
                }

                require_equals_value(argument, "--kind=", "--kind requires a value");

                saw_kind = true;

                index += 1;
            }

            "-s" | "--sort" => {
                if saw_sort {
                    fail("--sort may only be specified once with --graveyard");
                }

                require_option_value(arguments, index, "--sort");

                saw_sort = true;

                index += 2;
            }

            argument if argument.starts_with("--sort=") => {
                if saw_sort {
                    fail("--sort may only be specified once with --graveyard");
                }

                require_equals_value(argument, "--sort=", "--sort requires a field");

                saw_sort = true;

                index += 1;
            }

            argument if argument.starts_with('-') => {
                reject_argument(argument, selected.mode);
            }

            _ => {
                path_count += 1;

                if path_count > 1 {
                    fail(format!(
                        "unexpected argument '{}' with --graveyard",
                        argument,
                    ));
                }

                index += 1;
            }
        }
    }
}

fn required_target_index(arguments: &[String], selected: SelectedMode) -> usize {
    let target_index = selected.index + 1;

    if target_index >= arguments.len() || arguments[target_index].starts_with('-') {
        fail(format!(
            "{} requires a target argument",
            selected.mode.label(),
        ));
    }

    target_index
}

fn require_option_value(arguments: &[String], option_index: usize, option_name: &str) {
    if option_index + 1 >= arguments.len() || arguments[option_index + 1].starts_with('-') {
        fail(format!("{} requires a value", option_name,));
    }
}

fn require_equals_value(argument: &str, prefix: &str, error_message: &str) {
    let value = argument.trim_start_matches(prefix);

    if value.is_empty() {
        fail(error_message);
    }
}

fn is_global_color_option(argument: &str) -> bool {
    matches!(
        argument,
        "--no-color" | "--no-colour" | "--no-colors" | "--no-colours"
    )
}

fn reject_repeated_option(already_seen: bool, option_name: &str, mode: CommandMode) {
    if already_seen {
        fail(format!(
            "{} may only be specified once with {}",
            option_name,
            mode.label(),
        ));
    }
}

fn reject_extra_path(path_count: usize, argument: &str, mode: CommandMode) {
    if path_count > 1 {
        fail(format!(
            "unexpected argument '{}' with {}",
            argument,
            mode.label(),
        ));
    }
}

fn reject_argument(argument: &str, mode: CommandMode) -> ! {
    if argument.starts_with('-') {
        fail(format!(
            "{} is not supported with {}",
            argument,
            mode.label(),
        ));
    }

    fail(format!(
        "unexpected argument '{}' with {}",
        argument,
        mode.label(),
    ));
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("noct: {}", message,);

    std::process::exit(2);
}
