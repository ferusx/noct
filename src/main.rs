// SPDX-License-Identifier: BSD-3-Clause

mod args;
mod classify;
mod config;
mod entry;
mod filters;
mod format;
mod manual;
mod options;
mod render;
mod sizes;
mod themes;
pub mod tree;
mod validate;

use std::env;
use std::io::IsTerminal;
use std::path::Path;

use args::{DisplayMode, expand_short_flags, parse_arguments};
use config::{
    config_path, generate_config_copy, load_or_create_config, system_themes_directories,
    user_themes_directory,
};
use entry::{read_entries, read_path_entries, read_path_entry};
use options::{
    handle_audit_request, handle_cold_request, handle_du_request, handle_duplicates_request,
    handle_empty_dirs_request, handle_exts_request, handle_graveyard_request, handle_hot_request,
    handle_inspect_request, handle_leaves_request, handle_links_to_request, handle_map_request,
    handle_mounts_request, handle_names_request, handle_owners_request, handle_owns_request,
    handle_package_bins_request, handle_perms_request, handle_portable_request,
    handle_profile_request, handle_scripts_request, handle_timeline_request, handle_tree_request,
};
use validate::validate_arguments;

use render::{
    RenderOptions, calculate_column_widths, print_entry, print_grid, print_header, print_summary,
    print_total_listed_size, render_entry_label,
};

use manual::{print_help, print_manual, print_theme_help};
use themes::{AnsiPalette, ColorCapability, ColorMode, Theme};

// Constants
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let raw_arguments: Vec<String> = env::args().skip(1).collect();

    /*
     * Configuration generation must happen before load_or_create_config().
     *
     * Normal loading may create the live noct.toml when it is missing, whereas
     * --generate-config must create only the inert .generated copy.
     */
    if raw_arguments
        .iter()
        .any(|argument| argument == "--generate-config")
    {
        let generated_path = match generate_config_copy() {
            Ok(path) => path,

            Err(error) => {
                eprintln!("noct: unable to generate configuration: {}", error);

                std::process::exit(1);
            }
        };

        let Some(live_path) = config_path() else {
            eprintln!(
                "noct: generated {}, but unable to determine the live configuration path",
                generated_path.display(),
            );

            std::process::exit(1);
        };

        println!(
            "Generated configuration template: {}",
            generated_path.display(),
        );

        println!(
            "Rename it to {} after reviewing and editing it.",
            live_path.display(),
        );

        return;
    }

    let config = load_or_create_config();

    let _configured_tree_mode = config.display.show_tree;

    let arguments = expand_short_flags(&raw_arguments);

    validate_arguments(&arguments);

    let use_colors =
        std::io::stdout().is_terminal()
            && std::env::var_os("NO_COLOR").is_none();

    let color_mode = if use_colors {
        ColorMode::Auto
    } else {
        ColorMode::Never
    };

    let user_themes_directory = user_themes_directory();

    let system_themes_directories = system_themes_directories();

    let theme = Theme::load_named(
        &config.theme,
        user_themes_directory.as_deref(),
        &system_themes_directories,
    );

    let color_capability = ColorCapability::detect();

    let palette = AnsiPalette::new(&theme, color_mode, color_capability);

    if arguments.iter().any(|argument| argument == "--manual") {
        print_manual(use_colors);

        return;
    }

    if arguments.iter().any(|argument| argument == "--theme-help") {
        print_theme_help(use_colors);

        return;
    }

    if arguments.iter().any(|argument| argument == "--help") {
        print_help(use_colors);

        return;
    }

    if handle_package_bins_request(&arguments, &palette) {
        return;
    }

    if handle_owns_request(&arguments, &palette) {
        return;
    }

    if handle_inspect_request(&arguments, &palette) {
        return;
    }

    if handle_links_to_request(&arguments, &palette) {
        return;
    }

    if handle_duplicates_request(&arguments, &palette) {
        return;
    }

    if handle_du_request(&arguments, &palette) {
        return;
    }

    if handle_map_request(&arguments, &palette) {
        return;
    }

    if handle_timeline_request(&arguments, &palette) {
        return;
    }

    if handle_exts_request(&arguments, &palette) {
        return;
    }

    if handle_perms_request(&arguments, &palette) {
        return;
    }

    if handle_audit_request(&arguments, &palette) {
        return;
    }

    if handle_scripts_request(&arguments, &palette) {
        return;
    }

    if handle_names_request(&arguments, &palette) {
        return;
    }

    if handle_owners_request(&arguments, &palette) {
        return;
    }

    if handle_profile_request(&arguments, &palette) {
        return;
    }

    if handle_portable_request(&arguments, &palette) {
        return;
    }

    if handle_graveyard_request(&arguments, &palette) {
        return;
    }

    if handle_leaves_request(&arguments, &palette) {
        return;
    }

    if handle_mounts_request(&arguments, &palette) {
        return;
    }

    if handle_empty_dirs_request(&arguments, &palette) {
        return;
    }

    if handle_hot_request(&arguments, &palette) {
        return;
    }

    if handle_cold_request(&arguments, &palette) {
        return;
    }

    if handle_tree_request(&arguments, &palette, config.filesystem_colors) {
        return;
    }

    let options = parse_arguments(&arguments, &config);

    let path = Path::new(&options.paths[0]);

    let render_options = RenderOptions {
        use_colors: palette.enabled,
        filesystem_colors: config.filesystem_colors,
        show_icons: options.show_icons && !palette.ansi16,
        show_permissions: options.show_permissions,
        show_state: options.show_state,
        show_state_labels: options.show_state_labels,
        show_age: options.show_age,
        show_user: options.show_user,
        show_date: options.show_date,
        grid_padding: options.grid_padding,
        long_gap: options.long_gap,
        output_width: options.output_width,
    };

    let entries = if options.paths.len() > 1 {
        read_path_entries(
            &options.paths,
            &options.filters,
            options.sort_mode,
            options.time_field,
            options.reverse_sort,
        )
    } else if options.list_directory_itself || !path.is_dir() {
        read_path_entry(path, &options.filters)
    } else {
        read_entries(
            path,
            &options.filters,
            options.sort_mode,
            options.time_field,
            options.reverse_sort,
            options.hidden_mode,
        )
    };

    let mut entries = match entries {
        Ok(entries) => entries,

        Err(error) => {
            eprintln!("noct: {}", error);

            std::process::exit(1);
        }
    };

    if let Some(limit) = options.limit {
        entries.truncate(limit);
    } else if let Some(tail) = options.tail {
        let entry_count = entries.len();

        if entry_count > tail {
            entries.drain(..entry_count - tail);
        }
    }

    match options.display_mode {
        DisplayMode::Grid => {
            print_grid(&entries, &render_options, &palette);
        }

        DisplayMode::Long => {
            let column_widths = calculate_column_widths(&entries, options.time_field);

            if options.show_header {
                print_header(
                    &column_widths,
                    options.time_field,
                    &render_options,
                    &palette,
                );
            }

            for entry in &entries {
                print_entry(
                    entry,
                    &column_widths,
                    options.time_field,
                    &render_options,
                    &palette,
                );
            }

            print_total_listed_size(&entries, &render_options, &palette);
        }

        DisplayMode::Oneline => {
            for entry in &entries {
                println!("{}", render_entry_label(entry, &render_options, &palette,),);
            }
        }
    }

    if options.show_summary {
        print_summary(&entries, &render_options, &palette);

        if !matches!(options.display_mode, DisplayMode::Long) {
            print_total_listed_size(&entries, &render_options, &palette);
        }
    }
}
