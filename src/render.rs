// SPDX-License-Identifier: BSD-3-Clause

use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::time::SystemTime;

use crate::args::TimeField;
use crate::classify::{EntryClass, standard_filesystem_color};
use crate::config::FilesystemColorsMode;
use crate::entry::EntryInfo;
use crate::format::{human_age, human_date, human_size, terminal_width, user_name};
use crate::sizes::{
    FilesystemClass, SizeInfo, has_significant_virtual_apparent_size,
    significant_virtual_filesystem_types, total_entry_sizes,
};
use crate::themes::AnsiPalette;

#[derive(Debug)]
pub struct ColumnWidths {
    pub user: usize,
    pub size: usize,
    pub time: usize,
    pub age: usize,
    pub state: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub use_colors: bool,
    pub filesystem_colors: FilesystemColorsMode,
    pub show_icons: bool,
    pub show_permissions: bool,
    pub show_age: bool,
    pub show_user: bool,
    pub show_date: bool,
    pub show_state: bool,
    pub show_state_labels: bool,
    pub grid_padding: usize,
    pub long_gap: &'static str,
    pub output_width: Option<usize>,
}

fn header_for_time_field(time_field: TimeField) -> &'static str {
    match time_field {
        TimeField::Modified => "MODIFIED",
        TimeField::Accessed => "ACCESSED",
        TimeField::Created => "CREATED",
    }
}

fn time_for_entry(entry: &EntryInfo, time_field: TimeField) -> Option<SystemTime> {
    match time_field {
        TimeField::Modified => entry.metadata.modified().ok(),

        TimeField::Accessed => entry.metadata.accessed().ok(),

        TimeField::Created => entry.metadata.created().ok(),
    }
}

pub fn calculate_column_widths(entries: &[EntryInfo], time_field: TimeField) -> ColumnWidths {
    let mut widths = ColumnWidths {
        user: "USER".len(),
        size: "SIZE".len(),
        time: header_for_time_field(time_field).len(),
        age: "AGE".len(),
        state: "STATE".len(),
    };

    for entry in entries {
        let user = user_name(entry.metadata.uid());

        let size_info = SizeInfo::from_metadata(&entry.metadata);

        let size = human_size(size_info.allocated_bytes);

        let time = human_date(time_for_entry(entry, time_field));

        let age = human_age(time_for_entry(entry, time_field));

        widths.user = widths.user.max(user.len());

        widths.size = widths.size.max(size.len());

        widths.time = widths.time.max(time.len());

        widths.age = widths.age.max(age.len());

        widths.state = widths.state.max(entry.state.len());
    }

    widths
}

pub fn print_grid(entries: &[EntryInfo], render_options: &RenderOptions, palette: &AnsiPalette) {
    if entries.is_empty() {
        return;
    }

    let terminal_width = render_options.output_width.unwrap_or_else(terminal_width);

    let names: Vec<String> = entries
        .iter()
        .map(|entry| render_entry_label(entry, render_options, palette))
        .collect();

    let plain_widths: Vec<usize> = entries
        .iter()
        .map(|entry| plain_rendered_label(entry, render_options).len())
        .collect();

    let entry_count = entries.len();

    let mut best_columns = 1usize;

    let mut best_rows = entry_count;

    let mut best_column_widths = vec![plain_widths.iter().copied().max().unwrap_or(0)];

    for columns in (1..=entry_count).rev() {
        let rows = (entry_count + columns - 1) / columns;

        let mut column_widths = vec![0usize; columns];

        for column in 0..columns {
            for row in 0..rows {
                let index = row + column * rows;

                if index >= entry_count {
                    continue;
                }

                column_widths[column] = column_widths[column].max(plain_widths[index]);
            }
        }

        let total_width = column_widths.iter().sum::<usize>()
            + render_options
                .grid_padding
                .saturating_mul(columns.saturating_sub(1));

        if total_width <= terminal_width {
            best_columns = columns;

            best_rows = rows;

            best_column_widths = column_widths;

            break;
        }
    }

    for row in 0..best_rows {
        for column in 0..best_columns {
            let index = row + column * best_rows;

            if index >= entry_count {
                continue;
            }

            print!("{}", names[index]);

            if column + 1 < best_columns {
                let padding = best_column_widths[column].saturating_sub(plain_widths[index])
                    + render_options.grid_padding;

                print!("{}", " ".repeat(padding,));
            }
        }

        println!();
    }
}

pub fn print_header(
    widths: &ColumnWidths,
    time_field: TimeField,
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) {
    let mut columns = Vec::new();

    if render_options.show_permissions {
        columns.push(format!("{:<10}", "MODE",));
    }

    if render_options.show_user {
        columns.push(format!("{:<width$}", "USER", width = widths.user,));
    }

    columns.push(format!("{:>width$}", "SIZE", width = widths.size,));

    if render_options.show_date {
        columns.push(format!(
            "{:>width$}",
            header_for_time_field(time_field,),
            width = widths.time,
        ));
    }

    if render_options.show_age {
        columns.push(format!("{:>width$}", "AGE", width = widths.age,));
    }

    if render_options.show_state {
        columns.push(format!("{:<width$}", "STATE", width = widths.state,));
    }

    columns.push("NAME".to_string());

    let header = columns.join(render_options.long_gap);

    if render_options.use_colors {
        println!("{}{}{}", palette.header, header, palette.reset,);
    } else {
        println!("{}", header,);
    }
}

pub fn print_entry(
    entry: &EntryInfo,
    widths: &ColumnWidths,
    time_field: TimeField,
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) {
    let permissions = render_permissions(
        entry.file_type_char,
        entry.metadata.permissions().mode(),
        render_options,
        palette,
    );

    let user = user_name(entry.metadata.uid());

    let size_info = SizeInfo::from_metadata(&entry.metadata);

    let size = human_size(size_info.allocated_bytes);

    let size_has_unit = size_info.allocated_bytes >= 1024;

    let selected_time = time_for_entry(entry, time_field);

    let time = human_date(selected_time);

    let age = human_age(selected_time);

    let name = render_name(entry, render_options, palette);

    let state_color = if entry.state == "normal" {
        &palette.columns.state_normal
    } else {
        &palette.columns.state_attention
    };

    let mut columns = Vec::new();

    if render_options.show_permissions {
        columns.push(permissions);
    }

    if render_options.show_user {
        if render_options.use_colors {
            columns.push(format!(
                "{}{:<width$}{}",
                palette.columns.user,
                user,
                palette.reset,
                width = widths.user,
            ));
        } else {
            columns.push(format!("{:<width$}", user, width = widths.user,));
        }
    }

    if render_options.use_colors {
        let size_style = if size_has_unit && !palette.ansi16 {
            "\x1b[1m"
        } else {
            ""
        };

        let size_color = if size_has_unit {
            &palette.columns.size_units
        } else {
            &palette.columns.size_bytes
        };

        columns.push(format!(
            "{}{}{:>width$}{}",
            size_style,
            size_color,
            size,
            palette.reset,
            width = widths.size,
        ));

        if render_options.show_date {
            columns.push(format!(
                "{}{:>width$}{}",
                palette.columns.time,
                time,
                palette.reset,
                width = widths.time,
            ));
        }

        if render_options.show_age {
            columns.push(format!(
                "{}{:>width$}{}",
                palette.columns.age,
                age,
                palette.reset,
                width = widths.age,
            ));
        }

        if render_options.show_state {
            columns.push(format!(
                "{}{:<width$}{}",
                state_color,
                entry.state,
                palette.reset,
                width = widths.state,
            ));
        }
    } else {
        columns.push(format!("{:>width$}", size, width = widths.size,));

        if render_options.show_date {
            columns.push(format!("{:>width$}", time, width = widths.time,));
        }

        if render_options.show_age {
            columns.push(format!("{:>width$}", age, width = widths.age,));
        }

        if render_options.show_state {
            columns.push(format!("{:<width$}", entry.state, width = widths.state,));
        }
    }

    columns.push(name);

    println!("{}", columns.join(render_options.long_gap,),);
}

pub fn print_total_listed_size(
    entries: &[EntryInfo],
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) {
    let totals = total_entry_sizes(entries);

    let line = totals.formatted_total_line();

    if render_options.use_colors {
        println!("{}{}{}", palette.summary.total_size, line, palette.reset,);
    } else {
        println!("{}", line,);
    }

    let filesystem_types = significant_virtual_filesystem_types(entries);

    if filesystem_types.is_empty() {
        return;
    }

    let types = filesystem_types.join(", ");

    let note = if filesystem_types.len() == 1 {
        format!(
            "Note: apparent size includes virtual filesystem data from {}.",
            types,
        )
    } else {
        format!(
            "Note: apparent size includes virtual filesystem data from {}.",
            types,
        )
    };

    if render_options.use_colors {
        println!("{}{}{}", palette.tags.note, note, palette.reset,);
    } else {
        println!("{}", note,);
    }
}

pub fn print_summary(entries: &[EntryInfo], render_options: &RenderOptions, palette: &AnsiPalette) {
    let mut files = 0usize;

    let mut dirs = 0usize;

    let mut links = 0usize;

    let mut special = 0usize;

    let mut executables = 0usize;

    let mut empty = 0usize;

    let mut danger = 0usize;

    for entry in entries {
        let file_type = entry.metadata.file_type();

        if entry.metadata.is_dir() {
            dirs += 1;
        } else if file_type.is_symlink() {
            links += 1;
        } else if file_type.is_socket()
            || file_type.is_fifo()
            || file_type.is_block_device()
            || file_type.is_char_device()
        {
            special += 1;
        } else {
            files += 1;
        }

        if entry.metadata.is_file() && entry.metadata.permissions().mode() & 0o111 != 0 {
            executables += 1;
        }

        if entry.metadata.is_file() && entry.metadata.len() == 0 {
            empty += 1;
        }

        if entry.state != "normal" {
            danger += 1;
        }
    }

    if !render_options.use_colors {
        println!(
            "{} entries: {} files, {} dirs, {} links, {} special, {} executables, {} empty, {} danger",
            entries.len(),
            files,
            dirs,
            links,
            special,
            executables,
            empty,
            danger,
        );

        return;
    }

    print!(
        "{}{} entries:{} ",
        palette.summary.entries,
        entries.len(),
        palette.reset,
    );

    print!(
        "{}{} files{}, ",
        palette.summary.files, files, palette.reset,
    );

    print!(
        "{}{} dirs{}, ",
        palette.summary.directories, dirs, palette.reset,
    );

    print!(
        "{}{} links{}, ",
        palette.summary.symlinks, links, palette.reset,
    );

    print!(
        "{}{} special{}, ",
        palette.summary.special, special, palette.reset,
    );

    print!(
        "{}{} executables{}, ",
        palette.summary.executables, executables, palette.reset,
    );

    print!(
        "{}{} empty{}, ",
        palette.summary.empty, empty, palette.reset,
    );

    println!(
        "{}{} danger{}",
        palette.summary.danger, danger, palette.reset,
    );
}

fn entry_is_mount_point(entry: &EntryInfo) -> bool {
    if entry.filesystem.mount_point.as_os_str().is_empty() {
        return false;
    }

    let entry_path = std::fs::canonicalize(&entry.path).unwrap_or_else(|_| entry.path.clone());

    let mount_point = std::fs::canonicalize(&entry.filesystem.mount_point)
        .unwrap_or_else(|_| entry.filesystem.mount_point.clone());

    entry_path == mount_point
}

fn plain_entry_annotation(entry: &EntryInfo) -> Option<String> {
    if has_significant_virtual_apparent_size(entry) {
        let size_info = SizeInfo::from_metadata(&entry.metadata);

        return Some(format!(
            "[{} apparent, virtual]",
            human_size(size_info.apparent_bytes,),
        ));
    }

    if !entry_is_mount_point(entry) {
        return None;
    }

    match entry.filesystem.class {
        FilesystemClass::VirtualMetadata => Some(format!("[{}]", entry.filesystem.display_type(),)),

        FilesystemClass::MemoryBacked => Some(format!(
            "[{}, memory-backed]",
            entry.filesystem.display_type(),
        )),

        FilesystemClass::Ordinary | FilesystemClass::Unknown => None,
    }
}

fn render_entry_annotation(
    entry: &EntryInfo,
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) -> Option<String> {
    let annotation = plain_entry_annotation(entry)?;

    if !render_options.use_colors {
        return Some(annotation);
    }

    let color = if has_significant_virtual_apparent_size(entry) {
        &palette.tags.virtual_tag
    } else if entry.filesystem.is_memory_backed() {
        &palette.tags.memory
    } else {
        &palette.tags.filesystem
    };

    Some(format!("{}{}{}", color, annotation, palette.reset,))
}

fn entry_color<'a>(
    entry: &EntryInfo,
    render_options: &RenderOptions,
    palette: &'a AnsiPalette,
) -> &'a str {
    match render_options.filesystem_colors {
        FilesystemColorsMode::Standard => standard_filesystem_color(entry.class.color_class()),

        FilesystemColorsMode::Theme => palette.classification_color(entry.class.color_class()),

        FilesystemColorsMode::Directories => {
            if entry.metadata.is_dir() {
                "\x1b[38;2;80;155;235m"
            } else {
                ""
            }
        }
    }
}

pub fn icon_for_class(class: EntryClass) -> &'static str {
    match class {
        EntryClass::BrokenSymlink => "󰌷",

        EntryClass::Directory => "󰉋",

        EntryClass::Symlink => "󰌹",

        EntryClass::Socket => "󰆨",

        EntryClass::Fifo => "󰟥",

        EntryClass::BlockDevice | EntryClass::CharDevice => "󰋊",

        EntryClass::Executable => "󰆍",

        EntryClass::ShellScript | EntryClass::Awk => "",

        EntryClass::Rust => "",

        EntryClass::Python => "",

        EntryClass::C | EntryClass::Cpp => "",

        EntryClass::Java | EntryClass::Kotlin => "",

        EntryClass::JavaScript | EntryClass::TypeScript => "󰌞",

        EntryClass::Assembly => "",

        EntryClass::Lua => "",

        EntryClass::Ruby => "",

        EntryClass::Perl => "",

        EntryClass::Php => "",

        EntryClass::Go => "",

        EntryClass::Swift => "",

        EntryClass::Dart => "",

        EntryClass::CSharp | EntryClass::FSharp => "󰌛",

        EntryClass::Scala => "",

        EntryClass::Groovy => "",

        EntryClass::R => "󰟔",

        EntryClass::Elixir => "",

        EntryClass::Erlang => "",

        EntryClass::VisualBasic => "󰛂",

        EntryClass::Clojure => "",

        EntryClass::Zig => "",

        EntryClass::Nim => "",

        EntryClass::Crystal => "",

        EntryClass::Haskell => "",

        EntryClass::Ocaml => "",

        EntryClass::Pascal => "󰟢",

        EntryClass::Solidity => "",

        EntryClass::Vala => "",

        EntryClass::SourceCode => "󰅩",

        EntryClass::Web => "󰌝",

        EntryClass::Build => "󰣪",

        EntryClass::Config => "",

        EntryClass::StructuredData => "",

        EntryClass::Log => "󰌱",

        EntryClass::Archive | EntryClass::Package => "",

        EntryClass::Document | EntryClass::Text => "󰈙",

        EntryClass::Spreadsheet => "󰈛",

        EntryClass::Presentation => "󰈧",

        EntryClass::Image | EntryClass::VectorImage => "󰈟",

        EntryClass::Audio => "󰎈",

        EntryClass::Video => "󰈫",

        EntryClass::Font => "",

        EntryClass::Database => "",

        EntryClass::Torrent => "󰇚",

        EntryClass::DesktopEntry => "󰍹",

        EntryClass::Backup => "󰁯",

        EntryClass::Certificate => "󰄤",

        EntryClass::DiskImage => "󰋊",

        EntryClass::Plugin => "󰚥",

        EntryClass::Binary => "",

        EntryClass::File => "󰈔",
    }
}

pub fn render_name(
    entry: &EntryInfo,
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) -> String {
    let file_type = entry.metadata.file_type();

    let is_executable = entry.metadata.permissions().mode() & 0o111 != 0;

    let color = entry_color(entry, render_options, palette);

    let suffix = if entry.metadata.is_dir() {
        if entry.name == "/" { "" } else { "/" }
    } else if file_type.is_symlink() {
        "@"
    } else if is_executable {
        "*"
    } else {
        ""
    };

    let icon = if render_options.show_icons {
        Some(icon_for_class(entry.class))
    } else {
        None
    };

    let rendered_name = match (render_options.use_colors, icon) {
        (true, Some(icon)) => {
            format!(
                "{}{} {}{}{}",
                color, icon, entry.name, suffix, palette.reset,
            )
        }

        (true, None) => {
            format!("{}{}{}{}", color, entry.name, suffix, palette.reset,)
        }

        (false, Some(icon)) => {
            format!("{} {}{}", icon, entry.name, suffix,)
        }

        (false, None) => {
            format!("{}{}", entry.name, suffix,)
        }
    };

    match render_entry_annotation(entry, render_options, palette) {
        Some(annotation) => {
            format!("{}  {}", rendered_name, annotation,)
        }

        None => rendered_name,
    }
}

fn permission_char(mode: u32, bit: u32, letter: char) -> char {
    match bit {
        0o100 => {
            let has_execute = mode & 0o100 != 0;

            let has_setuid = mode & 0o4000 != 0;

            match (has_execute, has_setuid) {
                (true, true) => 's',
                (false, true) => 'S',
                (true, false) => 'x',
                (false, false) => '-',
            }
        }

        0o010 => {
            let has_execute = mode & 0o010 != 0;

            let has_setgid = mode & 0o2000 != 0;

            match (has_execute, has_setgid) {
                (true, true) => 's',
                (false, true) => 'S',
                (true, false) => 'x',
                (false, false) => '-',
            }
        }

        0o001 => {
            let has_execute = mode & 0o001 != 0;

            let has_sticky = mode & 0o1000 != 0;

            match (has_execute, has_sticky) {
                (true, true) => 't',
                (false, true) => 'T',
                (true, false) => 'x',
                (false, false) => '-',
            }
        }

        _ => {
            if mode & bit != 0 {
                letter
            } else {
                '-'
            }
        }
    }
}

pub fn render_permissions(
    file_type_char: char,
    mode: u32,
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) -> String {
    let mut output = String::new();

    if render_options.use_colors {
        output.push_str(color_for_file_type(file_type_char, palette));

        output.push(file_type_char);

        output.push_str(palette.reset);
    } else {
        output.push(file_type_char);
    }

    let permission_bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];

    for (bit, letter) in permission_bits {
        let rendered = permission_char(mode, bit, letter);

        if render_options.use_colors {
            output.push_str(color_for_permission(rendered, palette));

            output.push(rendered);

            output.push_str(palette.reset);
        } else {
            output.push(rendered);
        }
    }

    output
}

fn color_for_file_type<'a>(file_type_char: char, palette: &'a AnsiPalette) -> &'a str {
    match file_type_char {
        'd' => &palette.file_type.directory,

        'l' => &palette.file_type.symlink,

        's' | 'p' | 'b' | 'c' => &palette.file_type.special,

        _ => &palette.file_type.file,
    }
}

fn color_for_permission<'a>(permission: char, palette: &'a AnsiPalette) -> &'a str {
    match permission {
        'r' => &palette.permissions.read,

        'w' => &palette.permissions.write,

        'x' | 's' | 'S' | 't' | 'T' => &palette.permissions.execute,

        _ => &palette.permissions.missing,
    }
}

fn render_state(state: &str, render_options: &RenderOptions, palette: &AnsiPalette) -> String {
    if !render_options.use_colors {
        return state.to_string();
    }

    let color = if state == "normal" {
        &palette.columns.state_normal
    } else {
        &palette.columns.state_attention
    };

    format!("{}{}{}", color, state, palette.reset,)
}

pub fn render_entry_label(
    entry: &EntryInfo,
    render_options: &RenderOptions,
    palette: &AnsiPalette,
) -> String {
    if !render_options.show_state_labels {
        return render_name(entry, render_options, palette);
    }

    format!(
        "[{}] {}",
        render_state(&entry.state, render_options, palette,),
        render_name(entry, render_options, palette,),
    )
}

fn plain_rendered_label(entry: &EntryInfo, render_options: &RenderOptions) -> String {
    if !render_options.show_state_labels {
        return plain_rendered_name(entry, render_options);
    }

    format!(
        "[{}] {}",
        entry.state,
        plain_rendered_name(entry, render_options),
    )
}

fn plain_rendered_name(entry: &EntryInfo, render_options: &RenderOptions) -> String {
    let suffix = if entry.metadata.is_dir() {
        if entry.name == "/" { "" } else { "/" }
    } else if entry.metadata.file_type().is_symlink() {
        "@"
    } else if entry.metadata.permissions().mode() & 0o111 != 0 {
        "*"
    } else {
        ""
    };

    /*
     * Grid measurement must reserve the same two terminal cells used by the
     * rendered icon and its following space.
     *
     * Do not insert the actual Nerd Font glyph here: String::len() measures
     * UTF-8 bytes rather than terminal display cells.
     */
    let icon_space = if render_options.show_icons { "  " } else { "" };

    let name = format!("{}{}{}", icon_space, entry.name, suffix);

    match plain_entry_annotation(entry) {
        Some(annotation) => {
            format!("{}  {}", name, annotation)
        }

        None => name,
    }
}
