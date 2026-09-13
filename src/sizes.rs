// SPDX-License-Identifier: BSD-3-Clause

use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
use std::process::Command;

use crate::entry::EntryInfo;
use crate::format::human_size;

const ALLOCATED_BLOCK_BYTES: u64 = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemClass {
    Ordinary,
    VirtualMetadata,
    MemoryBacked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemInfo {
    pub filesystem_type: String,
    pub mount_point: PathBuf,
    pub class: FilesystemClass,
}

impl FilesystemInfo {
    pub fn unknown() -> Self {
        Self {
            filesystem_type: "unknown".to_string(),

            mount_point: PathBuf::new(),

            class: FilesystemClass::Unknown,
        }
    }

    pub fn display_type(&self) -> &str {
        match self.filesystem_type.as_str() {
            "proc" => "procfs",

            filesystem_type => filesystem_type,
        }
    }

    pub fn is_virtual_metadata(&self) -> bool {
        matches!(self.class, FilesystemClass::VirtualMetadata)
    }

    pub fn is_memory_backed(&self) -> bool {
        matches!(self.class, FilesystemClass::MemoryBacked)
    }
}

#[derive(Debug, Clone)]
struct MountRecord {
    mount_point: PathBuf,
    filesystem_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct MountTable {
    records: Vec<MountRecord>,
}

impl MountTable {
    pub fn load() -> Self {
        let mut records = platform_mount_records();

        records.sort_by(|left, right| {
            right
                .mount_point
                .components()
                .count()
                .cmp(&left.mount_point.components().count())
        });

        Self { records }
    }

    pub fn filesystem_for_path(&self, path: &Path) -> FilesystemInfo {
        let absolute_path = absolute_lexical_path(path);

        for record in &self.records {
            if path_is_within_mount(&absolute_path, &record.mount_point) {
                return FilesystemInfo {
                    filesystem_type: record.filesystem_type.clone(),

                    mount_point: record.mount_point.clone(),

                    class: classify_filesystem(&record.filesystem_type),
                };
            }
        }

        FilesystemInfo::unknown()
    }
}

fn path_is_within_mount(path: &Path, mount_point: &Path) -> bool {
    mount_point == Path::new("/") || path == mount_point || path.starts_with(mount_point)
}

fn absolute_lexical_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };

    normalize_lexical_path(&absolute)
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::RootDir => {
                normalized.push("/");
            }

            Component::CurDir => {}

            Component::ParentDir => {
                normalized.pop();
            }

            Component::Normal(part) => {
                normalized.push(part);
            }

            Component::Prefix(prefix) => {
                normalized.push(prefix.as_os_str());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        normalized
    }
}

fn classify_filesystem(filesystem_type: &str) -> FilesystemClass {
    match filesystem_type {
        "proc" | "procfs" | "kernfs" | "ptyfs" | "sysfs" | "cgroup" | "cgroup2" | "debugfs"
        | "tracefs" | "securityfs" | "configfs" | "pstore" | "fusectl" | "mqueue" | "devpts"
        | "binfmt_misc" => FilesystemClass::VirtualMetadata,

        "tmpfs" | "devtmpfs" | "ramfs" => FilesystemClass::MemoryBacked,

        "" | "unknown" => FilesystemClass::Unknown,

        _ => FilesystemClass::Ordinary,
    }
}

#[cfg(target_os = "linux")]
fn platform_mount_records() -> Vec<MountRecord> {
    let Ok(contents) = fs::read_to_string("/proc/self/mountinfo") else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(parse_linux_mountinfo_line)
        .collect()
}

#[cfg(target_os = "linux")]
fn parse_linux_mountinfo_line(line: &str) -> Option<MountRecord> {
    let (mount_fields, filesystem_fields) = line.split_once(" - ")?;

    let mount_point = mount_fields.split_whitespace().nth(4)?;

    let filesystem_type = filesystem_fields.split_whitespace().next()?;

    Some(MountRecord {
        mount_point: PathBuf::from(decode_mount_field(mount_point)),

        filesystem_type: filesystem_type.to_string(),
    })
}

#[cfg(target_os = "freebsd")]
fn platform_mount_records() -> Vec<MountRecord> {
    let Ok(output) = Command::new("mount").arg("-p").output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_freebsd_mount_line)
        .collect()
}

#[cfg(target_os = "freebsd")]
fn parse_freebsd_mount_line(line: &str) -> Option<MountRecord> {
    let mut fields = line.split_whitespace();

    let _device = fields.next()?;

    let mount_point = fields.next()?;

    let filesystem_type = fields.next()?;

    Some(MountRecord {
        mount_point: PathBuf::from(decode_mount_field(mount_point)),

        filesystem_type: filesystem_type.to_string(),
    })
}

#[cfg(target_os = "netbsd")]
fn platform_mount_records() -> Vec<MountRecord> {
    let Ok(output) = Command::new("mount").output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_netbsd_mount_line)
        .collect()
}

#[cfg(target_os = "netbsd")]
fn parse_netbsd_mount_line(line: &str) -> Option<MountRecord> {
    let (_, mount_and_type) = line.split_once(" on ")?;

    let (mount_point, type_and_options) = mount_and_type.split_once(" type ")?;

    let filesystem_type = type_and_options.split_whitespace().next()?;

    Some(MountRecord {
        mount_point: PathBuf::from(decode_mount_field(mount_point)),

        filesystem_type: filesystem_type.to_string(),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd",)))]
fn platform_mount_records() -> Vec<MountRecord> {
    Vec::new()
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd",))]
fn decode_mount_field(value: &str) -> String {
    let bytes = value.as_bytes();

    let mut decoded = Vec::with_capacity(bytes.len());

    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'\\'
            && index + 3 < bytes.len()
            && bytes[index + 1].is_ascii_digit()
            && bytes[index + 2].is_ascii_digit()
            && bytes[index + 3].is_ascii_digit()
        {
            let first = bytes[index + 1] - b'0';

            let second = bytes[index + 2] - b'0';

            let third = bytes[index + 3] - b'0';

            if first <= 7 && second <= 7 && third <= 7 {
                decoded.push(first * 64 + second * 8 + third);

                index += 4;

                continue;
            }
        }

        decoded.push(bytes[index]);

        index += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SizeInfo {
    pub apparent_bytes: u64,
    pub allocated_bytes: u64,
}

impl SizeInfo {
    pub fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            apparent_bytes: metadata.len(),

            allocated_bytes: metadata.blocks().saturating_mul(ALLOCATED_BLOCK_BYTES),
        }
    }
}

const SIGNIFICANT_VIRTUAL_APPARENT_BYTES: u64 = 1024 * 1024 * 1024;

const SIGNIFICANT_VIRTUAL_RATIO: u64 = 1024;

pub fn has_significant_virtual_apparent_size(entry: &EntryInfo) -> bool {
    if !entry.filesystem.is_virtual_metadata() {
        return false;
    }

    let size = SizeInfo::from_metadata(&entry.metadata);

    if size.apparent_bytes < SIGNIFICANT_VIRTUAL_APPARENT_BYTES {
        return false;
    }

    if size.allocated_bytes == 0 {
        return true;
    }

    size.apparent_bytes
        >= size
            .allocated_bytes
            .saturating_mul(SIGNIFICANT_VIRTUAL_RATIO)
}

pub fn significant_virtual_filesystem_types(entries: &[EntryInfo]) -> Vec<String> {
    let mut filesystem_types = Vec::new();

    for entry in entries {
        if !has_significant_virtual_apparent_size(entry) {
            continue;
        }

        let filesystem_type = entry.filesystem.display_type().to_string();

        if !filesystem_types.contains(&filesystem_type) {
            filesystem_types.push(filesystem_type);
        }
    }

    filesystem_types.sort();

    filesystem_types
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SizeTotals {
    pub apparent_bytes: u64,
    pub allocated_bytes: u64,
}

impl SizeTotals {
    pub fn add_metadata(&mut self, metadata: &fs::Metadata) {
        let size_info = SizeInfo::from_metadata(metadata);

        self.apparent_bytes = self.apparent_bytes.saturating_add(size_info.apparent_bytes);

        self.allocated_bytes = self
            .allocated_bytes
            .saturating_add(size_info.allocated_bytes);
    }

    pub fn formatted_total_line(self) -> String {
        let allocated = human_size(self.allocated_bytes);

        let apparent = human_size(self.apparent_bytes);

        if allocated == apparent {
            format!("Total listed size: {}", allocated,)
        } else {
            format!("Total listed size: {} (apparent: {})", allocated, apparent,)
        }
    }
}

pub fn total_entry_sizes(entries: &[EntryInfo]) -> SizeTotals {
    let mut totals = SizeTotals::default();

    for entry in entries {
        totals.add_metadata(&entry.metadata);
    }

    totals
}
