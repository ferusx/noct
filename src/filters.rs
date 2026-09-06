use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::args::{Filter, FilterType};

const BIG_FILE_BYTES: u64 = 10 * 1024 * 1024;
const TINY_FILE_BYTES: u64 = 1024;

pub fn entry_matches_filters(
    path: &Path,
    metadata: &fs::Metadata,
    state: &str,
    filters: &[Filter],
) -> bool {
    filters
        .iter()
        .all(|filter| entry_matches_filter(path, metadata, state, filter))
}

fn entry_matches_filter(
    path: &Path,
    metadata: &fs::Metadata,
    state: &str,
    filter: &Filter,
) -> bool {
    match filter {
        Filter::Extension(extension) => {
            if !metadata.is_file() {
                return false;
            }

            let Some(file_extension) = path.extension() else {
                return false;
            };

            file_extension.to_string_lossy().to_lowercase() == *extension
        }

        Filter::Type(filter_type) => {
            let file_type = metadata.file_type();

            match filter_type {
                FilterType::File => metadata.is_file(),

                FilterType::Directory => metadata.is_dir(),

                FilterType::Symlink => file_type.is_symlink(),

                FilterType::Special => {
                    file_type.is_socket()
                        || file_type.is_fifo()
                        || file_type.is_block_device()
                        || file_type.is_char_device()
                }
            }
        }

        Filter::State(wanted_state) => state == wanted_state,

        Filter::Danger => state != "normal",

        Filter::Orphan => state == "broken-link" || state == "no-owner" || state == "no-group",

        Filter::Empty => metadata.is_file() && metadata.len() == 0,

        Filter::Big => metadata.is_file() && metadata.len() >= BIG_FILE_BYTES,

        Filter::Tiny => metadata.is_file() && metadata.len() <= TINY_FILE_BYTES,

        Filter::GreaterThan(size) => metadata.is_file() && metadata.len() >= *size,

        Filter::LessThan(size) => metadata.is_file() && metadata.len() <= *size,

        Filter::Executable => metadata.is_file() && metadata.permissions().mode() & 0o111 != 0,

        Filter::Writable => metadata.permissions().mode() & 0o222 != 0,

        Filter::WorldWritable => metadata.permissions().mode() & 0o002 != 0,

        Filter::Suid => metadata.permissions().mode() & 0o4000 != 0,

        Filter::Sgid => metadata.permissions().mode() & 0o2000 != 0,

        Filter::ExecutableWithDirs => metadata.permissions().mode() & 0o111 != 0,

        Filter::Permissions(wanted_mode) => metadata.permissions().mode() & 0o777 == *wanted_mode,

        Filter::Fresh => {
            is_younger_than(metadata.modified().ok(), Duration::from_secs(60 * 60 * 24))
        }

        Filter::NewerThan(duration) => is_younger_than(metadata.modified().ok(), *duration),

        Filter::OlderThan(duration) => is_older_than(metadata.modified().ok(), *duration),

        Filter::Stale => is_older_than(
            metadata.modified().ok(),
            Duration::from_secs(60 * 60 * 24 * 30),
        ),
    }
}

fn is_younger_than(time: Option<SystemTime>, limit: Duration) -> bool {
    let Some(time) = time else {
        return false;
    };

    SystemTime::now()
        .duration_since(time)
        .map(|age| age <= limit)
        .unwrap_or(false)
}

fn is_older_than(time: Option<SystemTime>, limit: Duration) -> bool {
    let Some(time) = time else {
        return false;
    };

    SystemTime::now()
        .duration_since(time)
        .map(|age| age >= limit)
        .unwrap_or(false)
}
