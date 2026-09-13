// SPDX-License-Identifier: BSD-3-Clause

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_CONFIG: &str = r#"# Noct configuration
#
# Command-line options override settings from this file.
#
# A useful starting point for customization is to copy an existing Noct theme
# and adjust its colors rather than building a complete theme from scratch.

# Theme loaded from ~/.config/noct/themes/.
theme = "verdant_mocha"

# Filesystem filename and icon colors.
#
# "theme" uses [classification] colors from the selected theme. Any omitted
# classification entry falls back to Noct's built-in classification palette;
# those fallback values are the same throughout all themes.
#
# Because the fallback palette is theme-independent, filename and directory
# colors may not harmonize with the rest of a custom theme when filesystem
# classification entries are omitted.
#
# "standard" always uses Noct's built-in classification palette regardless of
# the selected theme.
#
# "directories" disables classification colors for ordinary filenames while
# retaining directory coloring in listings.
filesystem_colors = "theme"

[display]

# Enable colored terminal output.
colors = true

# Show file-type icons. Requires a font containing the used Nerd Font glyphs.
icons = false

# Show the long-view column header.
show_header = false

# Print a summary after ordinary listings.
show_summary = false

# Use Tree output by default.
show_tree = false

[long]

# Columns shown in long listing mode.
show_permissions = true
show_user = true
show_date = true
show_age = true
show_state = true

[listing]

# Default sort field: "name", "size", "age", "date", "extension", or "unsorted".
sort = "name"

# Reverse the selected sort order.
reverse = false

# Maximum number of displayed entries. 0 means unlimited.
limit = 0
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilesystemColorsMode {
    Theme,
    Standard,
    Directories,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NoctConfig {
    pub theme: String,

    pub filesystem_colors: FilesystemColorsMode,

    pub display: DisplayConfig,

    pub long: LongConfig,

    pub listing: ListingConfig,
}

impl Default for NoctConfig {
    fn default() -> Self {
        Self {
            theme: "verdant_mocha".to_string(),

            filesystem_colors: FilesystemColorsMode::Theme,

            display: DisplayConfig::default(),

            long: LongConfig::default(),

            listing: ListingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub colors: bool,
    pub icons: bool,
    pub show_header: bool,
    pub show_summary: bool,
    pub show_tree: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            colors: true,
            icons: false,
            show_header: false,
            show_summary: false,
            show_tree: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LongConfig {
    pub show_permissions: bool,
    pub show_user: bool,
    pub show_date: bool,
    pub show_age: bool,
    pub show_state: bool,
}

impl Default for LongConfig {
    fn default() -> Self {
        Self {
            show_permissions: true,
            show_user: true,
            show_date: true,
            show_age: true,
            show_state: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ListingConfig {
    pub sort: String,
    pub reverse: bool,
    pub limit: usize,
}

impl Default for ListingConfig {
    fn default() -> Self {
        Self {
            sort: "name".to_string(),

            reverse: false,

            limit: 0,
        }
    }
}

pub fn config_directory() -> Option<PathBuf> {
    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config_home).join("noct"));
    }

    let home = env::var_os("HOME")?;

    Some(PathBuf::from(home).join(".config").join("noct"))
}

pub fn user_themes_directory() -> Option<PathBuf> {
    config_directory().map(|directory| directory.join("themes"))
}

pub fn system_themes_directories() -> Vec<PathBuf> {
    let data_directories = env::var_os("XDG_DATA_DIRS")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });

    data_directories
        .into_iter()
        .map(|directory| directory.join("noct").join("themes"))
        .collect()
}
pub fn config_path() -> Option<PathBuf> {
    config_directory().map(|directory| directory.join("noct.toml"))
}

pub fn generated_config_path() -> Option<PathBuf> {
    config_directory().map(|directory| directory.join("noct.toml.generated"))
}

pub fn generate_config_copy() -> io::Result<PathBuf> {
    let path = generated_config_path().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "unable to determine the configuration directory",
        )
    })?;

    let Some(parent) = path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "generated configuration path has no parent directory",
        ));
    };

    fs::create_dir_all(parent)?;

    /*
     * The generated template is disposable and may always be refreshed.
     *
     * This operation never touches the active noct.toml configuration.
     * An existing noct.toml.generated is replaced with the newest built-in
     * template so newly introduced settings appear immediately.
     */
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;

    file.write_all(DEFAULT_CONFIG.as_bytes())?;

    file.flush()?;

    Ok(path)
}

pub fn load_or_create_config() -> NoctConfig {
    let Some(path) = config_path() else {
        eprintln!("noct: unable to determine the configuration directory");

        return NoctConfig::default();
    };

    if let Some(themes_directory) = user_themes_directory() {
        if let Err(error) = fs::create_dir_all(&themes_directory) {
            eprintln!(
                "noct: unable to create themes directory {}: {}",
                themes_directory.display(),
                error,
            );
        }
    }

    if !path.exists() {
        if let Err(error) = write_default_config(&path) {
            eprintln!(
                "noct: unable to create configuration {}: {}",
                path.display(),
                error,
            );

            return NoctConfig::default();
        }
    }

    load_config_from_path(&path)
}

fn load_config_from_path(path: &Path) -> NoctConfig {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,

        Err(error) => {
            eprintln!(
                "noct: unable to read configuration {}: {}",
                path.display(),
                error,
            );

            return NoctConfig::default();
        }
    };

    match toml::from_str::<NoctConfig>(&contents) {
        Ok(config) => config,

        Err(error) => {
            eprintln!("noct: invalid configuration {}: {}", path.display(), error,);

            NoctConfig::default()
        }
    }
}

fn write_default_config(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "configuration path has no parent directory",
        ));
    };

    fs::create_dir_all(parent)?;

    fs::write(path, DEFAULT_CONFIG)
}
