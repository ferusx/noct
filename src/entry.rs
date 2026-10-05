// SPDX-License-Identifier: BSD-3-Clause

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::args::{HiddenMode, SortMode, TimeField};
use crate::classify::{EntryClass, classify_entry};
use crate::filters::entry_matches_filters;
use crate::format::{gid_exists, uid_exists};
use crate::sizes::{FilesystemInfo, MountTable};

#[derive(Debug)]
pub struct EntryInfo {
    pub name: String,
    pub path: PathBuf,
    pub tree_prefix: Option<String>,
    pub tree_omitted_prefix: Option<String>,
    pub tree_omitted_count: usize,
    pub metadata: fs::Metadata,
    pub class: EntryClass,
    pub file_type_char: char,
    pub state: String,
    pub filesystem: FilesystemInfo,
}

pub fn file_kind_label(metadata: &fs::Metadata) -> &'static str {
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_socket() {
        "socket"
    } else if file_type.is_fifo() {
        "fifo"
    } else if file_type.is_block_device() {
        "block device"
    } else if file_type.is_char_device() {
        "char device"
    } else if metadata.is_file() {
        "regular file"
    } else {
        "unknown"
    }
}

pub fn file_type_char(metadata: &fs::Metadata) -> char {
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        'd'
    } else if file_type.is_symlink() {
        'l'
    } else if file_type.is_socket() {
        's'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_block_device() {
        'b'
    } else if file_type.is_char_device() {
        'c'
    } else {
        '.'
    }
}

pub fn detect_state(path: &Path, metadata: &fs::Metadata) -> String {
    if metadata.file_type().is_symlink() && fs::metadata(path).is_err() {
        return "broken-link".to_string();
    }

    if !uid_exists(metadata.uid()) {
        return "no-owner".to_string();
    }

    if !gid_exists(metadata.gid()) {
        return "no-group".to_string();
    }

    let mode = metadata.permissions().mode();

    if mode & 0o002 != 0 {
        return "world-writable".to_string();
    }

    if mode & 0o4000 != 0 {
        return "setuid".to_string();
    }

    if mode & 0o2000 != 0 {
        return "setgid".to_string();
    }

    if metadata.is_file() && metadata.len() == 0 {
        return "empty".to_string();
    }

    "normal".to_string()
}

fn extension_for_name(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn time_for_sort(metadata: &fs::Metadata, time_field: TimeField) -> Option<std::time::SystemTime> {
    match time_field {
        TimeField::Modified => metadata.modified().ok(),
        TimeField::Accessed => metadata.accessed().ok(),
        TimeField::Created => metadata.created().ok(),
    }
}

pub fn sort_entries(entries: &mut [EntryInfo], sort_mode: SortMode, time_field: TimeField) {
    match sort_mode {
        SortMode::Name => {
            entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }

        SortMode::Size => {
            entries.sort_by(|a, b| {
                b.metadata
                    .len()
                    .cmp(&a.metadata.len())
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }

        SortMode::Age => {
            entries.sort_by(|a, b| {
                let a_modified = a.metadata.modified().ok();

                let b_modified = b.metadata.modified().ok();

                b_modified
                    .cmp(&a_modified)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }

        SortMode::Date => {
            entries.sort_by(|a, b| {
                let a_time = time_for_sort(&a.metadata, time_field);

                let b_time = time_for_sort(&b.metadata, time_field);

                b_time
                    .cmp(&a_time)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }

        SortMode::Extension => {
            entries.sort_by(|a, b| {
                extension_for_name(&a.name)
                    .cmp(&extension_for_name(&b.name))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }

        SortMode::Unsorted => {}
    }
}

pub fn read_path_entry(
    path: &Path,
    filters: &[crate::args::Filter],
) -> std::io::Result<Vec<EntryInfo>> {
    let mut entries = Vec::new();

    let mount_table = MountTable::load();

    let name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    maybe_push_entry(path, &name, filters, &mount_table, &mut entries)?;

    Ok(entries)
}

pub fn read_path_entries(
    paths: &[String],
    filters: &[crate::args::Filter],
    sort_mode: SortMode,
    time_field: TimeField,
    reverse_sort: bool,
) -> std::io::Result<Vec<EntryInfo>> {
    let mut entries = Vec::new();

    let mount_table = MountTable::load();

    for path_text in paths {
        let path = Path::new(path_text);

        let name = path
            .file_name()
            .map(|file_name| file_name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        if let Err(error) = maybe_push_entry(path, &name, filters, &mount_table, &mut entries) {
            return Err(std::io::Error::new(
                error.kind(),
                format!("unable to read {}: {}", path.display(), error),
            ));
        }
    }

    sort_entries(&mut entries, sort_mode, time_field);

    if reverse_sort && !matches!(sort_mode, SortMode::Unsorted) {
        entries.reverse();
    }

    Ok(entries)
}

pub fn read_entries(
    path: &Path,
    filters: &[crate::args::Filter],
    sort_mode: SortMode,
    time_field: TimeField,
    reverse_sort: bool,
    hidden_mode: HiddenMode,
) -> std::io::Result<Vec<EntryInfo>> {
    let mut entries = Vec::new();

    let mount_table = MountTable::load();

    if matches!(hidden_mode, HiddenMode::All) {
        maybe_push_entry(path, ".", filters, &mount_table, &mut entries)?;

        maybe_push_entry(&path.join(".."), "..", filters, &mount_table, &mut entries)?;
    }

    for result in fs::read_dir(path)? {
        let dir_entry = result?;
        let entry_path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        if matches!(hidden_mode, HiddenMode::VisibleOnly) && name.starts_with('.') {
            continue;
        }

        maybe_push_entry(&entry_path, &name, filters, &mount_table, &mut entries)?;
    }

    sort_entries(&mut entries, sort_mode, time_field);

    if reverse_sort && !matches!(sort_mode, SortMode::Unsorted) {
        entries.reverse();
    }

    Ok(entries)
}

pub fn read_entries_with_names_tree(
    path: &Path,
    filters: &[crate::args::Filter],
    sort_mode: SortMode,
    time_field: TimeField,
    reverse_sort: bool,
    hidden_mode: HiddenMode,
    max_depth: usize,
    per_directory_limit: usize,
) -> std::io::Result<Vec<EntryInfo>> {
    let mount_table = MountTable::load();

    let mut entries = Vec::new();

    read_names_tree_directory(
        path,
        filters,
        sort_mode,
        time_field,
        reverse_sort,
        hidden_mode,
        &mount_table,
        &[],
        true,
        0,
        max_depth,
        per_directory_limit,
        &mut entries,
    )?;

    Ok(entries)
}

fn read_names_tree_directory(
    directory: &Path,
    filters: &[crate::args::Filter],
    sort_mode: SortMode,
    time_field: TimeField,
    reverse_sort: bool,
    hidden_mode: HiddenMode,
    mount_table: &MountTable,
    ancestor_has_more: &[bool],
    top_level: bool,
    depth: usize,
    max_depth: usize,
    per_directory_limit: usize,
    entries: &mut Vec<EntryInfo>,
) -> std::io::Result<()> {
    let mut children = Vec::new();

    for result in fs::read_dir(directory)? {
        let dir_entry = result?;

        let path = dir_entry.path();

        let name = dir_entry.file_name().to_string_lossy().to_string();

        if matches!(hidden_mode, HiddenMode::VisibleOnly) && name.starts_with('.') {
            continue;
        }

        let metadata = fs::symlink_metadata(&path)?;

        let state = detect_state(&path, &metadata);

        let class = classify_entry(&path, &name, &metadata);

        let file_type_char = file_type_char(&metadata);

        let filesystem = mount_table.filesystem_for_path(&path);

        children.push(EntryInfo {
            name,
            path,
            tree_prefix: None,
            tree_omitted_prefix: None,
            tree_omitted_count: 0,
            metadata,
            class,
            file_type_char,
            state,
            filesystem,
        });
    }

    sort_entries(&mut children, sort_mode, time_field);

    if reverse_sort && !matches!(sort_mode, SortMode::Unsorted) {
        children.reverse();
    }

    let omitted_count =
        if !top_level && per_directory_limit > 0 && children.len() > per_directory_limit {
            children.len() - per_directory_limit
        } else {
            0
        };

    if omitted_count > 0 {
        children.truncate(per_directory_limit);
    }

    let child_count = children.len();

    for (index, mut entry) in children.into_iter().enumerate() {
        let is_last_visible = index + 1 == child_count;

        let is_last = is_last_visible && omitted_count == 0;

        if top_level {
            entry.tree_prefix = Some(String::new());
        } else {
            entry.tree_prefix = Some(names_tree_prefix(ancestor_has_more, is_last));
        }

        if is_last_visible && omitted_count > 0 {
            entry.tree_omitted_prefix =
                Some(names_tree_prefix(ancestor_has_more, true));

            entry.tree_omitted_count = omitted_count;
        }
        
        let descend = entry.metadata.is_dir();

        let child_path = entry.path.clone();

        /*
         * Apply the ordinary listing filters to displayed entries.
         *
         * Directories are still traversed even when they do not themselves
         * satisfy a filter, so matching descendants remain discoverable.
         */
        if entry_matches_filters(
            &entry.path,
            &entry.metadata,
            &entry.state,
            filters,
        ) {
            entries.push(entry);
        }

        if descend && depth < max_depth {
            let mut child_ancestor_state = ancestor_has_more.to_vec();

            if !top_level {
                child_ancestor_state.push(!is_last);
            }

            read_names_tree_directory(
                &child_path,
                filters,
                sort_mode,
                time_field,
                reverse_sort,
                hidden_mode,
                mount_table,
                &child_ancestor_state,
                false,
                depth + 1,
                max_depth,
                per_directory_limit,
                entries,
            )?;
        }
    }

    Ok(())
}

fn names_tree_prefix(ancestor_has_more: &[bool], is_last: bool) -> String {
    let mut prefix = String::new();

    for has_more in ancestor_has_more {
        if *has_more {
            prefix.push_str("│   ");
        } else {
            prefix.push_str("    ");
        }
    }

    if is_last {
        prefix.push_str("└── ");
    } else {
        prefix.push_str("├── ");
    }

    prefix
}

fn maybe_push_entry(
    path: &Path,
    name: &str,
    filters: &[crate::args::Filter],
    mount_table: &MountTable,
    entries: &mut Vec<EntryInfo>,
) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;

    let state = detect_state(path, &metadata);

    if !entry_matches_filters(path, &metadata, &state, filters) {
        return Ok(());
    }

    let class = classify_entry(path, name, &metadata);

    let file_type_char = file_type_char(&metadata);

    let filesystem = mount_table.filesystem_for_path(path);

    entries.push(EntryInfo {
        name: name.to_string(),

        path: path.to_path_buf(),

        tree_prefix: None,
        tree_omitted_prefix: None,
        tree_omitted_count: 0,

        metadata,
        class,
        file_type_char,
        state,
        filesystem,
    });

    Ok(())
}
