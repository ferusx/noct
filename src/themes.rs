// SPDX-License-Identifier: BSD-3-Clause

use std::env;
use std::ffi::CStr;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use crate::classify::FilesystemColorClass;
use crate::config::{system_themes_directories, user_themes_directory};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub red: u8,

    pub green: u8,

    pub blue: u8,
}

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /*
     * Produce a standard ANSI 24-bit foreground-color sequence.
     */
    pub fn foreground_ansi(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.red, self.green, self.blue,)
    }

    /*
     * Produce a standard ANSI 24-bit background-color sequence.
     */
    pub fn background_ansi(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.red, self.green, self.blue,)
    }

    /*
     * Return the color in familiar hexadecimal notation.
     *
     * This will also be useful later for --print-theme and diagnostics.
     */
    #[cfg(test)]
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue,)
    }

    pub fn from_hex(value: &str) -> Result<Self, String> {
        let hexadecimal = value
            .trim()
            .strip_prefix('#')
            .unwrap_or_else(|| value.trim());

        if hexadecimal.len() != 6 {
            return Err(format!("expected #RRGGBB, found '{}'", value,));
        }

        let red = u8::from_str_radix(&hexadecimal[0..2], 16)
            .map_err(|_| format!("invalid red component in '{}'", value,))?;

        let green = u8::from_str_radix(&hexadecimal[2..4], 16)
            .map_err(|_| format!("invalid green component in '{}'", value,))?;

        let blue = u8::from_str_radix(&hexadecimal[4..6], 16)
            .map_err(|_| format!("invalid blue component in '{}'", value,))?;

        Ok(Self::new(red, green, blue))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCapability {
    TrueColor,
    Ansi16,
}

impl ColorCapability {
    pub fn detect() -> Self {
        let Some(tty_path) = stdout_tty_path() else {
            return Self::TrueColor;
        };

        if is_physical_console_tty(&tty_path) {
            Self::Ansi16
        } else {
            Self::TrueColor
        }
    }
}

fn stdout_tty_path() -> Option<String> {
    if !io::stdout().is_terminal() {
        return None;
    }

    let tty_name = unsafe { libc::ttyname(libc::STDOUT_FILENO) };

    if tty_name.is_null() {
        return None;
    }

    Some(
        unsafe { CStr::from_ptr(tty_name) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn is_physical_console_tty(path: &str) -> bool {
    match env::consts::OS {
        /*
         * FreeBSD and DragonFly BSD virtual consoles use /dev/ttyv*.
         */
        "freebsd" | "dragonfly" => path.starts_with("/dev/ttyv") || path == "/dev/console",

        /*
         * NetBSD wscons virtual terminals normally use /dev/ttyE*.
         */
        "netbsd" => {
            path.starts_with("/dev/ttyE")
                || path == "/dev/console"
                || path == "/dev/constty"
        },

        /*
         * OpenBSD wscons virtual terminals use /dev/ttyC*.
         */
        "openbsd" => path.starts_with("/dev/ttyC") || path == "/dev/console",

        /*
         * Linux virtual consoles use /dev/tty followed by a number.
         * Do not mistake serial terminals such as /dev/ttyS0 for consoles.
         */
        "linux" => {
            if path == "/dev/console" {
                return true;
            }

            path.strip_prefix("/dev/tty")
                .map(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
                })
                .unwrap_or(false)
        }

        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /*
     * Emit colors only when standard output is an interactive terminal.
     */
    Auto,

    /*
     * Emit colors regardless of whether output is redirected or piped.
     */
    #[allow(dead_code)]
    Always,

    /*
     * Never emit ANSI styling.
     */
    Never,
}

impl ColorMode {
    /*
     * Resolve the configured policy into one final yes/no decision.
     *
     * In Auto mode:
     *
     * - direct terminal output receives color;
     * - pipes and redirection do not;
     * - the NO_COLOR environment variable disables styling.
     */
    pub fn colors_enabled(self) -> bool {
        match self {
            Self::Always => true,

            Self::Never => false,

            Self::Auto => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
        }
    }

    /*
     * This variant is useful for deterministic tests because it does not
     * inspect the actual process environment.
     */
    #[cfg(test)]
    pub fn colors_enabled_for(self, stdout_is_terminal: bool, no_color_is_set: bool) -> bool {
        match self {
            Self::Always => true,

            Self::Never => false,

            Self::Auto => stdout_is_terminal && !no_color_is_set,
        }
    }
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    pub file_type: FileTypeColors,

    pub permissions: PermissionColors,

    pub columns: ColumnColors,

    pub names: NameColors,

    pub classification: ClassificationColors,

    pub tags: TagColors,

    pub report: ReportColors,

    pub header: HeaderStyle,

    pub summary: SummaryColors,
}

#[derive(Debug, Clone)]
pub struct FileTypeColors {
    pub file: RgbColor,

    pub directory: RgbColor,

    pub symlink: RgbColor,

    pub special: RgbColor,
}

#[derive(Debug, Clone)]
pub struct PermissionColors {
    pub read: RgbColor,

    pub write: RgbColor,

    pub execute: RgbColor,

    pub missing: RgbColor,
}

#[derive(Debug, Clone)]
pub struct ColumnColors {
    pub user: RgbColor,

    pub size: RgbColor,

    pub time: RgbColor,

    pub age: RgbColor,

    pub state_normal: RgbColor,

    pub state_attention: RgbColor,
}

#[derive(Debug, Clone)]
pub struct NameColors {
    pub file: RgbColor,

    pub directory: RgbColor,

    pub symlink: RgbColor,

    pub broken_symlink: RgbColor,

    pub executable: RgbColor,

    pub special: RgbColor,
}

#[derive(Debug, Clone)]
pub struct ClassificationColors {
    pub broken_symlink: RgbColor,

    pub directory: RgbColor,

    pub symlink: RgbColor,

    pub special: RgbColor,

    pub executable: RgbColor,

    pub source_code: RgbColor,

    pub shell: RgbColor,

    pub web: RgbColor,

    pub build_config: RgbColor,

    pub structured_data: RgbColor,

    pub log: RgbColor,

    pub archive_package: RgbColor,

    pub document: RgbColor,

    pub spreadsheet: RgbColor,

    pub presentation: RgbColor,

    pub image: RgbColor,

    pub audio: RgbColor,

    pub video: RgbColor,

    pub font: RgbColor,

    pub database: RgbColor,

    pub backup: RgbColor,

    pub certificate: RgbColor,

    pub disk_image: RgbColor,

    pub torrent: RgbColor,

    pub desktop_plugin: RgbColor,

    pub binary: RgbColor,

    pub file: RgbColor,
}

#[derive(Debug, Clone)]
pub struct TagColors {
    pub filesystem: RgbColor,

    pub virtual_tag: RgbColor,

    pub memory: RgbColor,

    pub note: RgbColor,
}

#[derive(Debug, Clone)]
pub struct ReportColors {
    pub heading: RgbColor,

    pub label: RgbColor,

    pub value: RgbColor,

    pub name: RgbColor,

    pub path: RgbColor,

    pub command: RgbColor,

    pub package: RgbColor,

    pub version: RgbColor,

    pub size: RgbColor,

    pub count: RgbColor,

    pub total: RgbColor,

    pub muted: RgbColor,

    pub attention: RgbColor,
}

#[derive(Debug, Clone)]
pub struct HeaderStyle {
    pub foreground: RgbColor,

    pub background: RgbColor,

    pub bold: bool,
}

#[derive(Debug, Clone)]
pub struct SummaryColors {
    pub entries: RgbColor,

    pub total_size: RgbColor,

    pub entries_bold: bool,

    pub files: RgbColor,

    pub directories: RgbColor,

    pub symlinks: RgbColor,

    pub special: RgbColor,

    pub executables: RgbColor,

    pub empty: RgbColor,

    pub danger: RgbColor,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16ThemeFile {
    file_type: Ansi16FileTypeThemeFile,
    permissions: Ansi16PermissionThemeFile,
    columns: Ansi16ColumnThemeFile,
    names: Ansi16NameThemeFile,
    classification: Ansi16ClassificationThemeFile,
    tags: Ansi16TagThemeFile,
    report: Ansi16ReportThemeFile,
    header: Ansi16HeaderThemeFile,
    summary: Ansi16SummaryThemeFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16NameThemeFile {
    directory: Option<String>,
    symlink: Option<String>,
    broken_symlink: Option<String>,
    executable: Option<String>,
    special: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16FileTypeThemeFile {
    file: Option<String>,
    directory: Option<String>,
    symlink: Option<String>,
    special: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16PermissionThemeFile {
    read: Option<String>,
    write: Option<String>,
    execute: Option<String>,
    missing: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16ColumnThemeFile {
    user: Option<String>,
    size_bytes: Option<String>,
    size_units: Option<String>,
    time: Option<String>,
    age: Option<String>,
    state_normal: Option<String>,
    state_attention: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16ClassificationThemeFile {
    broken_symlink: Option<String>,
    directory: Option<String>,
    symlink: Option<String>,
    special: Option<String>,
    executable: Option<String>,
    source_code: Option<String>,
    shell: Option<String>,
    web: Option<String>,
    build_config: Option<String>,
    structured_data: Option<String>,
    log: Option<String>,
    archive_package: Option<String>,
    document: Option<String>,
    spreadsheet: Option<String>,
    presentation: Option<String>,
    image: Option<String>,
    audio: Option<String>,
    video: Option<String>,
    font: Option<String>,
    database: Option<String>,
    backup: Option<String>,
    certificate: Option<String>,
    disk_image: Option<String>,
    torrent: Option<String>,
    desktop_plugin: Option<String>,
    binary: Option<String>,
    file: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16TagThemeFile {
    filesystem: Option<String>,
    virtual_tag: Option<String>,
    memory: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16ReportThemeFile {
    heading: Option<String>,
    label: Option<String>,
    value: Option<String>,
    name: Option<String>,
    path: Option<String>,
    command: Option<String>,
    package: Option<String>,
    version: Option<String>,
    size: Option<String>,
    count: Option<String>,
    total: Option<String>,
    muted: Option<String>,
    attention: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16HeaderThemeFile {
    foreground: Option<String>,
    background: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ansi16SummaryThemeFile {
    entries: Option<String>,
    total_size: Option<String>,
    files: Option<String>,
    directories: Option<String>,
    symlinks: Option<String>,
    special: Option<String>,
    executables: Option<String>,
    empty: Option<String>,
    danger: Option<String>,
}

fn ansi16_foreground(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "black" => Some("\x1b[30m"),
        "red" => Some("\x1b[31m"),
        "green" => Some("\x1b[32m"),
        "yellow" => Some("\x1b[33m"),
        "blue" => Some("\x1b[34m"),
        "magenta" => Some("\x1b[35m"),
        "cyan" => Some("\x1b[36m"),

        /*
         * ANSI 37 is traditionally named "white", but on the BSD consoles
         * we target here it behaves as the useful light-gray slot.
         */
        "light_gray" | "light-grey" | "lightgray" | "lightgrey" => Some("\x1b[37m"),

        /*
         * ANSI 90 is traditionally "bright black"; visually it is the
         * dark-gray slot we want exposed to console-theme authors.
         */
        "dark_gray" | "dark-grey" | "darkgray" | "darkgrey" => Some("\x1b[90m"),

        "bright_red" => Some("\x1b[91m"),
        "bright_green" => Some("\x1b[92m"),
        "bright_yellow" => Some("\x1b[93m"),
        "bright_blue" => Some("\x1b[94m"),
        "bright_magenta" => Some("\x1b[95m"),
        "bright_cyan" => Some("\x1b[96m"),

        /*
         * ANSI 97 is the true bright-white slot.
         */
        "white" | "bright_white" => Some("\x1b[97m"),

        _ => None,
    }
}

fn ansi16_background(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "black" => Some("\x1b[40m"),
        "red" => Some("\x1b[41m"),
        "green" => Some("\x1b[42m"),
        "yellow" => Some("\x1b[43m"),
        "blue" => Some("\x1b[44m"),
        "magenta" => Some("\x1b[45m"),
        "cyan" => Some("\x1b[46m"),
        "light_gray" | "light-grey" | "lightgray" | "lightgrey" => Some("\x1b[47m"),

        "dark_gray" | "dark-grey" | "darkgray" | "darkgrey" => Some("\x1b[100m"),
        "bright_red" => Some("\x1b[101m"),
        "bright_green" => Some("\x1b[102m"),
        "bright_yellow" => Some("\x1b[103m"),
        "bright_blue" => Some("\x1b[104m"),
        "bright_magenta" => Some("\x1b[105m"),
        "bright_cyan" => Some("\x1b[106m"),
        "white" | "bright_white" => Some("\x1b[107m"),

        _ => None,
    }
}

fn apply_ansi16_foreground(target: &mut String, value: Option<String>, field: &str) {
    let Some(value) = value else {
        return;
    };

    match ansi16_foreground(&value) {
        Some(color) => {
            *target = color.to_string();
        }

        None => {
            eprintln!(
                "noct: invalid ANSI16 color '{}' for {}; keeping fallback color",
                value, field,
            );
        }
    }
}

fn resolved_ansi16_background(
    value: Option<String>,
    fallback: &'static str,
    field: &str,
) -> String {
    let Some(value) = value else {
        return fallback.to_string();
    };

    match ansi16_background(&value) {
        Some(color) => color.to_string(),

        None => {
            eprintln!(
                "noct: invalid ANSI16 color '{}' for {}; keeping fallback color",
                value, field,
            );

            fallback.to_string()
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ThemeFile {
    name: Option<String>,

    file_type: FileTypeThemeFile,

    permissions: PermissionThemeFile,

    columns: ColumnThemeFile,

    names: NameThemeFile,

    classification: ClassificationThemeFile,

    tags: TagThemeFile,

    report: ReportThemeFile,

    header: HeaderThemeFile,

    summary: SummaryThemeFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileTypeThemeFile {
    file: Option<String>,

    directory: Option<String>,

    symlink: Option<String>,

    special: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PermissionThemeFile {
    read: Option<String>,

    write: Option<String>,

    execute: Option<String>,

    missing: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ColumnThemeFile {
    user: Option<String>,

    size: Option<String>,

    time: Option<String>,

    age: Option<String>,

    state_normal: Option<String>,

    state_attention: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct NameThemeFile {
    file: Option<String>,

    directory: Option<String>,

    symlink: Option<String>,

    broken_symlink: Option<String>,

    executable: Option<String>,

    special: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ClassificationThemeFile {
    broken_symlink: Option<String>,

    directory: Option<String>,

    symlink: Option<String>,

    special: Option<String>,

    executable: Option<String>,

    source_code: Option<String>,

    shell: Option<String>,

    web: Option<String>,

    build_config: Option<String>,

    structured_data: Option<String>,

    log: Option<String>,

    archive_package: Option<String>,

    document: Option<String>,

    spreadsheet: Option<String>,

    presentation: Option<String>,

    image: Option<String>,

    audio: Option<String>,

    video: Option<String>,

    font: Option<String>,

    database: Option<String>,

    backup: Option<String>,

    certificate: Option<String>,

    disk_image: Option<String>,

    torrent: Option<String>,

    desktop_plugin: Option<String>,

    binary: Option<String>,

    file: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TagThemeFile {
    filesystem: Option<String>,

    virtual_tag: Option<String>,

    memory: Option<String>,

    note: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ReportThemeFile {
    heading: Option<String>,

    label: Option<String>,

    value: Option<String>,

    name: Option<String>,

    path: Option<String>,

    command: Option<String>,

    package: Option<String>,

    version: Option<String>,

    size: Option<String>,

    count: Option<String>,

    total: Option<String>,

    muted: Option<String>,

    attention: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct HeaderThemeFile {
    foreground: Option<String>,

    background: Option<String>,

    bold: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SummaryThemeFile {
    entries: Option<String>,

    total_size: Option<String>,

    entries_bold: Option<bool>,

    files: Option<String>,

    directories: Option<String>,

    symlinks: Option<String>,

    special: Option<String>,

    executables: Option<String>,

    empty: Option<String>,

    danger: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnsiPalette {
    pub enabled: bool,

    pub ansi16: bool,

    pub reset: &'static str,

    pub file_type: AnsiFileTypeColors,

    pub permissions: AnsiPermissionColors,

    pub columns: AnsiColumnColors,

    pub names: AnsiNameColors,

    pub classification: AnsiClassificationColors,

    pub tags: AnsiTagColors,

    pub report: AnsiReportColors,

    pub header: String,

    pub summary: AnsiSummaryColors,
}

#[derive(Debug, Clone)]
pub struct AnsiFileTypeColors {
    pub file: String,

    pub directory: String,

    pub symlink: String,

    pub special: String,
}

#[derive(Debug, Clone)]
pub struct AnsiPermissionColors {
    pub read: String,

    pub write: String,

    pub execute: String,

    pub missing: String,
}

#[derive(Debug, Clone)]
pub struct AnsiColumnColors {
    pub user: String,

    pub size_bytes: String,

    pub size_units: String,

    pub time: String,

    pub age: String,

    pub state_normal: String,

    pub state_attention: String,
}

#[derive(Debug, Clone)]
pub struct AnsiNameColors {
    pub directory: String,

    pub symlink: String,

    pub broken_symlink: String,

    pub executable: String,

    pub special: String,
}

#[derive(Debug, Clone)]
pub struct AnsiClassificationColors {
    pub broken_symlink: String,

    pub directory: String,

    pub symlink: String,

    pub special: String,

    pub executable: String,

    pub source_code: String,

    pub shell: String,

    pub web: String,

    pub build_config: String,

    pub structured_data: String,

    pub log: String,

    pub archive_package: String,

    pub document: String,

    pub spreadsheet: String,

    pub presentation: String,

    pub image: String,

    pub audio: String,

    pub video: String,

    pub font: String,

    pub database: String,

    pub backup: String,

    pub certificate: String,

    pub disk_image: String,

    pub torrent: String,

    pub desktop_plugin: String,

    pub binary: String,

    pub file: String,
}

#[derive(Debug, Clone)]
pub struct AnsiTagColors {
    pub filesystem: String,

    pub virtual_tag: String,

    pub memory: String,

    pub note: String,
}

#[derive(Debug, Clone)]
pub struct AnsiReportColors {
    pub heading: String,

    pub label: String,

    pub value: String,

    pub name: String,

    pub path: String,

    pub command: String,

    pub package: String,

    pub version: String,

    pub size: String,

    pub count: String,

    pub total: String,

    pub muted: String,

    pub attention: String,
}

#[derive(Debug, Clone)]
pub struct AnsiSummaryColors {
    pub entries: String,

    pub total_size: String,

    pub files: String,

    pub directories: String,

    pub symlinks: String,

    pub special: String,

    pub executables: String,

    pub empty: String,

    pub danger: String,
}

impl Theme {
    /*
     * Noct's built-in default palette.
     *
     * Verdant Mocha is always available internally and also serves as the
     * fallback whenever:
     *
     * - no external theme is selected;
     * - a theme file is incomplete;
     * - a theme cannot be read;
     * - a theme contains an invalid color.
     */
    pub fn builtin() -> Self {
        Self {
            name: "verdant_mocha".to_string(),

            file_type: FileTypeColors {
                file: RgbColor::new(216, 210, 196),
                directory: RgbColor::new(79, 168, 143),
                symlink: RgbColor::new(104, 183, 174),
                special: RgbColor::new(155, 122, 98),
            },

            permissions: PermissionColors {
                read: RgbColor::new(127, 133, 125),
                write: RgbColor::new(95, 159, 154),
                execute: RgbColor::new(131, 182, 127),
                missing: RgbColor::new(73, 68, 62),
            },

            columns: ColumnColors {
                user: RgbColor::new(133, 125, 115),
                size: RgbColor::new(143, 189, 139),
                time: RgbColor::new(109, 159, 158),
                age: RgbColor::new(162, 135, 111),
                state_normal: RgbColor::new(113, 107, 99),
                state_attention: RgbColor::new(201, 111, 98),
            },

            names: NameColors {
                file: RgbColor::new(216, 210, 196),
                directory: RgbColor::new(79, 168, 143),
                symlink: RgbColor::new(104, 183, 174),
                broken_symlink: RgbColor::new(201, 111, 98),
                executable: RgbColor::new(131, 182, 127),
                special: RgbColor::new(155, 122, 98),
            },

            classification: ClassificationColors {
                broken_symlink: RgbColor::new(201, 111, 98),
                directory: RgbColor::new(79, 168, 143),
                symlink: RgbColor::new(104, 183, 174),
                special: RgbColor::new(155, 122, 98),

                executable: RgbColor::new(131, 182, 127),
                source_code: RgbColor::new(169, 201, 143),
                shell: RgbColor::new(131, 182, 127),
                web: RgbColor::new(111, 169, 156),

                build_config: RgbColor::new(109, 159, 158),
                structured_data: RgbColor::new(114, 179, 166),
                log: RgbColor::new(127, 133, 125),

                archive_package: RgbColor::new(164, 111, 79),
                document: RgbColor::new(216, 210, 196),
                spreadsheet: RgbColor::new(143, 189, 139),
                presentation: RgbColor::new(170, 133, 98),

                image: RgbColor::new(120, 166, 154),
                audio: RgbColor::new(104, 183, 174),
                video: RgbColor::new(112, 145, 139),
                font: RgbColor::new(168, 156, 141),

                database: RgbColor::new(79, 144, 136),
                backup: RgbColor::new(142, 117, 105),
                certificate: RgbColor::new(184, 149, 102),
                disk_image: RgbColor::new(117, 141, 137),
                torrent: RgbColor::new(112, 170, 120),
                desktop_plugin: RgbColor::new(146, 122, 104),

                binary: RgbColor::new(113, 107, 99),
                file: RgbColor::new(216, 210, 196),
            },

            tags: TagColors {
                filesystem: RgbColor::new(164, 111, 79),
                virtual_tag: RgbColor::new(104, 183, 174),
                memory: RgbColor::new(142, 117, 105),
                note: RgbColor::new(184, 149, 102),
            },

            report: ReportColors {
                heading: RgbColor::new(109, 181, 157),
                label: RgbColor::new(104, 183, 174),
                value: RgbColor::new(216, 210, 196),
                name: RgbColor::new(216, 210, 196),
                path: RgbColor::new(104, 183, 174),
                command: RgbColor::new(131, 182, 127),
                package: RgbColor::new(155, 122, 98),
                version: RgbColor::new(164, 111, 79),
                size: RgbColor::new(143, 189, 139),
                count: RgbColor::new(143, 135, 125),
                total: RgbColor::new(238, 232, 220),
                muted: RgbColor::new(113, 107, 99),
                attention: RgbColor::new(201, 111, 98),
            },

            header: HeaderStyle {
                foreground: RgbColor::new(23, 26, 23),
                background: RgbColor::new(109, 181, 157),
                bold: true,
            },

            summary: SummaryColors {
                entries: RgbColor::new(238, 232, 220),
                entries_bold: true,
                files: RgbColor::new(216, 210, 196),
                directories: RgbColor::new(79, 168, 143),
                symlinks: RgbColor::new(104, 183, 174),
                special: RgbColor::new(155, 122, 98),
                executables: RgbColor::new(131, 182, 127),
                empty: RgbColor::new(113, 107, 99),
                danger: RgbColor::new(201, 111, 98),
                total_size: RgbColor::new(184, 149, 102),
            },
        }
    }

    fn apply_theme_file(&mut self, theme_file: ThemeFile) {
        if let Some(name) = theme_file.name {
            self.name = name;
        }

        apply_color(
            &mut self.file_type.file,
            theme_file.file_type.file,
            "file_type.file",
        );

        apply_color(
            &mut self.file_type.directory,
            theme_file.file_type.directory,
            "file_type.directory",
        );

        apply_color(
            &mut self.file_type.symlink,
            theme_file.file_type.symlink,
            "file_type.symlink",
        );

        apply_color(
            &mut self.file_type.special,
            theme_file.file_type.special,
            "file_type.special",
        );

        apply_color(
            &mut self.permissions.read,
            theme_file.permissions.read,
            "permissions.read",
        );

        apply_color(
            &mut self.permissions.write,
            theme_file.permissions.write,
            "permissions.write",
        );

        apply_color(
            &mut self.permissions.execute,
            theme_file.permissions.execute,
            "permissions.execute",
        );

        apply_color(
            &mut self.permissions.missing,
            theme_file.permissions.missing,
            "permissions.missing",
        );

        apply_color(
            &mut self.columns.user,
            theme_file.columns.user,
            "columns.user",
        );

        apply_color(
            &mut self.columns.size,
            theme_file.columns.size,
            "columns.size",
        );

        apply_color(
            &mut self.columns.time,
            theme_file.columns.time,
            "columns.time",
        );

        apply_color(&mut self.columns.age, theme_file.columns.age, "columns.age");

        apply_color(
            &mut self.columns.state_normal,
            theme_file.columns.state_normal,
            "columns.state_normal",
        );

        apply_color(
            &mut self.columns.state_attention,
            theme_file.columns.state_attention,
            "columns.state_attention",
        );

        apply_color(&mut self.names.file, theme_file.names.file, "names.file");

        apply_color(
            &mut self.names.directory,
            theme_file.names.directory,
            "names.directory",
        );

        apply_color(
            &mut self.names.symlink,
            theme_file.names.symlink,
            "names.symlink",
        );

        apply_color(
            &mut self.names.broken_symlink,
            theme_file.names.broken_symlink,
            "names.broken_symlink",
        );

        apply_color(
            &mut self.names.executable,
            theme_file.names.executable,
            "names.executable",
        );

        apply_color(
            &mut self.names.special,
            theme_file.names.special,
            "names.special",
        );

        apply_color(
            &mut self.classification.broken_symlink,
            theme_file.classification.broken_symlink,
            "classification.broken_symlink",
        );

        apply_color(
            &mut self.classification.directory,
            theme_file.classification.directory,
            "classification.directory",
        );

        apply_color(
            &mut self.classification.symlink,
            theme_file.classification.symlink,
            "classification.symlink",
        );

        apply_color(
            &mut self.classification.special,
            theme_file.classification.special,
            "classification.special",
        );

        apply_color(
            &mut self.classification.executable,
            theme_file.classification.executable,
            "classification.executable",
        );

        apply_color(
            &mut self.classification.source_code,
            theme_file.classification.source_code,
            "classification.source_code",
        );

        apply_color(
            &mut self.classification.shell,
            theme_file.classification.shell,
            "classification.shell",
        );

        apply_color(
            &mut self.classification.web,
            theme_file.classification.web,
            "classification.web",
        );

        apply_color(
            &mut self.classification.build_config,
            theme_file.classification.build_config,
            "classification.build_config",
        );

        apply_color(
            &mut self.classification.structured_data,
            theme_file.classification.structured_data,
            "classification.structured_data",
        );

        apply_color(
            &mut self.classification.log,
            theme_file.classification.log,
            "classification.log",
        );

        apply_color(
            &mut self.classification.archive_package,
            theme_file.classification.archive_package,
            "classification.archive_package",
        );

        apply_color(
            &mut self.classification.document,
            theme_file.classification.document,
            "classification.document",
        );

        apply_color(
            &mut self.classification.spreadsheet,
            theme_file.classification.spreadsheet,
            "classification.spreadsheet",
        );

        apply_color(
            &mut self.classification.presentation,
            theme_file.classification.presentation,
            "classification.presentation",
        );

        apply_color(
            &mut self.classification.image,
            theme_file.classification.image,
            "classification.image",
        );

        apply_color(
            &mut self.classification.audio,
            theme_file.classification.audio,
            "classification.audio",
        );

        apply_color(
            &mut self.classification.video,
            theme_file.classification.video,
            "classification.video",
        );

        apply_color(
            &mut self.classification.font,
            theme_file.classification.font,
            "classification.font",
        );

        apply_color(
            &mut self.classification.database,
            theme_file.classification.database,
            "classification.database",
        );

        apply_color(
            &mut self.classification.backup,
            theme_file.classification.backup,
            "classification.backup",
        );

        apply_color(
            &mut self.classification.certificate,
            theme_file.classification.certificate,
            "classification.certificate",
        );

        apply_color(
            &mut self.classification.disk_image,
            theme_file.classification.disk_image,
            "classification.disk_image",
        );

        apply_color(
            &mut self.classification.torrent,
            theme_file.classification.torrent,
            "classification.torrent",
        );

        apply_color(
            &mut self.classification.desktop_plugin,
            theme_file.classification.desktop_plugin,
            "classification.desktop_plugin",
        );

        apply_color(
            &mut self.classification.binary,
            theme_file.classification.binary,
            "classification.binary",
        );

        apply_color(
            &mut self.classification.file,
            theme_file.classification.file,
            "classification.file",
        );

        apply_color(
            &mut self.tags.filesystem,
            theme_file.tags.filesystem,
            "tags.filesystem",
        );

        apply_color(
            &mut self.tags.virtual_tag,
            theme_file.tags.virtual_tag,
            "tags.virtual_tag",
        );

        apply_color(&mut self.tags.memory, theme_file.tags.memory, "tags.memory");

        apply_color(&mut self.tags.note, theme_file.tags.note, "tags.note");

        apply_color(
            &mut self.report.heading,
            theme_file.report.heading,
            "report.heading",
        );

        apply_color(
            &mut self.report.label,
            theme_file.report.label,
            "report.label",
        );

        apply_color(
            &mut self.report.value,
            theme_file.report.value,
            "report.value",
        );

        apply_color(&mut self.report.name, theme_file.report.name, "report.name");

        apply_color(&mut self.report.path, theme_file.report.path, "report.path");

        apply_color(
            &mut self.report.command,
            theme_file.report.command,
            "report.command",
        );

        apply_color(
            &mut self.report.package,
            theme_file.report.package,
            "report.package",
        );

        apply_color(
            &mut self.report.version,
            theme_file.report.version,
            "report.version",
        );

        apply_color(&mut self.report.size, theme_file.report.size, "report.size");

        apply_color(
            &mut self.report.count,
            theme_file.report.count,
            "report.count",
        );

        apply_color(
            &mut self.report.total,
            theme_file.report.total,
            "report.total",
        );

        apply_color(
            &mut self.report.muted,
            theme_file.report.muted,
            "report.muted",
        );

        apply_color(
            &mut self.report.attention,
            theme_file.report.attention,
            "report.attention",
        );

        apply_color(
            &mut self.header.foreground,
            theme_file.header.foreground,
            "header.foreground",
        );

        apply_color(
            &mut self.header.background,
            theme_file.header.background,
            "header.background",
        );

        if let Some(bold) = theme_file.header.bold {
            self.header.bold = bold;
        }

        apply_color(
            &mut self.summary.entries,
            theme_file.summary.entries,
            "summary.entries",
        );

        apply_color(
            &mut self.summary.total_size,
            theme_file.summary.total_size,
            "summary.total_size",
        );

        if let Some(bold) = theme_file.summary.entries_bold {
            self.summary.entries_bold = bold;
        }

        apply_color(
            &mut self.summary.files,
            theme_file.summary.files,
            "summary.files",
        );

        apply_color(
            &mut self.summary.directories,
            theme_file.summary.directories,
            "summary.directories",
        );

        apply_color(
            &mut self.summary.symlinks,
            theme_file.summary.symlinks,
            "summary.symlinks",
        );

        apply_color(
            &mut self.summary.special,
            theme_file.summary.special,
            "summary.special",
        );

        apply_color(
            &mut self.summary.executables,
            theme_file.summary.executables,
            "summary.executables",
        );

        apply_color(
            &mut self.summary.empty,
            theme_file.summary.empty,
            "summary.empty",
        );

        apply_color(
            &mut self.summary.danger,
            theme_file.summary.danger,
            "summary.danger",
        );
    }

    pub fn load_named(
        theme_name: &str,
        user_themes_directory: Option<&Path>,
        system_themes_directories: &[PathBuf],
    ) -> Self {
        let mut theme = Self::builtin();

        let normalized_name = theme_name.trim();

        if normalized_name.is_empty() {
            return theme;
        }

        if !normalized_name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        }) {
            eprintln!(
                "noct: invalid theme name '{}'; using built-in verdant_mocha theme\n",
                normalized_name,
            );

            return theme;
        }

        let filename = format!("{}.toml", normalized_name);

        let mut candidate_paths = Vec::new();

        if let Some(user_themes_directory) = user_themes_directory {
            candidate_paths.push(user_themes_directory.join(&filename));
        }

        for system_themes_directory in system_themes_directories {
            candidate_paths.push(system_themes_directory.join(&filename));
        }

        let path = candidate_paths.into_iter().find(|path| path.is_file());

        let Some(path) = path else {
            /*
             * Verdant Mocha is built into Noct, so a missing
             * verdant_mocha.toml does not require an external theme file.
             */
            if normalized_name != "verdant_mocha" {
                eprintln!(
                    "noct: unable to find theme '{}'; using built-in verdant_mocha theme\n",
                    normalized_name,
                );
            }

            return theme;
        };

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,

            Err(error) => {
                eprintln!(
                    "noct: unable to load theme '{}' from {}: {}",
                    normalized_name,
                    path.display(),
                    error,
                );

                eprintln!("noct: using built-in verdant_mocha theme\n");

                return theme;
            }
        };

        let theme_file = match toml::from_str::<ThemeFile>(&contents) {
            Ok(theme_file) => theme_file,

            Err(error) => {
                eprintln!("noct: invalid theme {}: {}", path.display(), error);

                eprintln!("noct: using built-in verdant_mocha theme\n");

                return theme;
            }
        };

        theme.apply_theme_file(theme_file);

        theme
    }

    /*
     * Build the complete ANSI style used by the column header.
     */
    pub fn header_ansi(&self) -> String {
        let mut style = String::new();

        if self.header.bold {
            style.push_str("\x1b[1m");
        }

        style.push_str(&self.header.foreground.foreground_ansi());

        style.push_str(&self.header.background.background_ansi());

        style
    }

    /*
     * Build the complete ANSI style for the summary entry count.
     */
    pub fn summary_entries_ansi(&self) -> String {
        let mut style = String::new();

        if self.summary.entries_bold {
            style.push_str("\x1b[1m");
        }

        style.push_str(&self.summary.entries.foreground_ansi());

        style
    }
}

impl AnsiPalette {
    pub fn new(theme: &Theme, color_mode: ColorMode, capability: ColorCapability) -> Self {
        let enabled = color_mode.colors_enabled();

        match capability {
            ColorCapability::TrueColor => Self::from_theme(theme, enabled),
            ColorCapability::Ansi16 => Self::console_ansi16(enabled),
        }
    }

    pub fn from_theme(theme: &Theme, enabled: bool) -> Self {
        if !enabled {
            return Self::disabled();
        }

        Self {
            enabled: true,

            ansi16: false,

            reset: "\x1b[0m",

            file_type: AnsiFileTypeColors {
                file: theme.file_type.file.foreground_ansi(),

                directory: theme.file_type.directory.foreground_ansi(),

                symlink: theme.file_type.symlink.foreground_ansi(),

                special: theme.file_type.special.foreground_ansi(),
            },

            permissions: AnsiPermissionColors {
                read: theme.permissions.read.foreground_ansi(),

                write: theme.permissions.write.foreground_ansi(),

                execute: theme.permissions.execute.foreground_ansi(),

                missing: theme.permissions.missing.foreground_ansi(),
            },

            columns: AnsiColumnColors {
                user: theme.columns.user.foreground_ansi(),

                size_bytes: theme.columns.size.foreground_ansi(),

                size_units: theme.columns.size.foreground_ansi(),

                time: theme.columns.time.foreground_ansi(),

                age: theme.columns.age.foreground_ansi(),

                state_normal: theme.columns.state_normal.foreground_ansi(),

                state_attention: theme.columns.state_attention.foreground_ansi(),
            },

            names: AnsiNameColors {
                directory: theme.names.directory.foreground_ansi(),

                symlink: theme.names.symlink.foreground_ansi(),

                broken_symlink: theme.names.broken_symlink.foreground_ansi(),

                executable: theme.names.executable.foreground_ansi(),

                special: theme.names.special.foreground_ansi(),
            },

            classification: AnsiClassificationColors {
                broken_symlink: theme.classification.broken_symlink.foreground_ansi(),

                directory: theme.classification.directory.foreground_ansi(),

                symlink: theme.classification.symlink.foreground_ansi(),

                special: theme.classification.special.foreground_ansi(),

                executable: theme.classification.executable.foreground_ansi(),

                source_code: theme.classification.source_code.foreground_ansi(),

                shell: theme.classification.shell.foreground_ansi(),

                web: theme.classification.web.foreground_ansi(),

                build_config: theme.classification.build_config.foreground_ansi(),

                structured_data: theme.classification.structured_data.foreground_ansi(),

                log: theme.classification.log.foreground_ansi(),

                archive_package: theme.classification.archive_package.foreground_ansi(),

                document: theme.classification.document.foreground_ansi(),

                spreadsheet: theme.classification.spreadsheet.foreground_ansi(),

                presentation: theme.classification.presentation.foreground_ansi(),

                image: theme.classification.image.foreground_ansi(),

                audio: theme.classification.audio.foreground_ansi(),

                video: theme.classification.video.foreground_ansi(),

                font: theme.classification.font.foreground_ansi(),

                database: theme.classification.database.foreground_ansi(),

                backup: theme.classification.backup.foreground_ansi(),

                certificate: theme.classification.certificate.foreground_ansi(),

                disk_image: theme.classification.disk_image.foreground_ansi(),

                torrent: theme.classification.torrent.foreground_ansi(),

                desktop_plugin: theme.classification.desktop_plugin.foreground_ansi(),

                binary: theme.classification.binary.foreground_ansi(),

                file: theme.classification.file.foreground_ansi(),
            },

            tags: AnsiTagColors {
                filesystem: theme.tags.filesystem.foreground_ansi(),

                virtual_tag: theme.tags.virtual_tag.foreground_ansi(),

                memory: theme.tags.memory.foreground_ansi(),

                note: theme.tags.note.foreground_ansi(),
            },

            report: AnsiReportColors {
                heading: format!("\x1b[1m{}", theme.report.heading.foreground_ansi(),),

                label: theme.report.label.foreground_ansi(),

                value: theme.report.value.foreground_ansi(),

                name: theme.report.name.foreground_ansi(),

                path: theme.report.path.foreground_ansi(),

                command: format!("\x1b[1m{}", theme.report.command.foreground_ansi(),),

                package: theme.report.package.foreground_ansi(),

                version: theme.report.version.foreground_ansi(),

                size: theme.report.size.foreground_ansi(),

                count: theme.report.count.foreground_ansi(),

                total: theme.report.total.foreground_ansi(),

                muted: theme.report.muted.foreground_ansi(),

                attention: theme.report.attention.foreground_ansi(),
            },

            header: theme.header_ansi(),

            summary: AnsiSummaryColors {
                entries: theme.summary_entries_ansi(),

                total_size: theme.summary.total_size.foreground_ansi(),

                files: theme.summary.files.foreground_ansi(),

                directories: theme.summary.directories.foreground_ansi(),

                symlinks: theme.summary.symlinks.foreground_ansi(),

                special: theme.summary.special.foreground_ansi(),

                executables: theme.summary.executables.foreground_ansi(),

                empty: theme.summary.empty.foreground_ansi(),

                danger: theme.summary.danger.foreground_ansi(),
            },
        }
    }

    fn console_ansi16(enabled: bool) -> Self {
        if !enabled {
            return Self::disabled();
        }

        const BLACK: &str = "\x1b[30m";
        const WHITE: &str = "\x1b[97m";
        const RED: &str = "\x1b[31m";
        const GREEN: &str = "\x1b[32m";
        const YELLOW: &str = "\x1b[33m";
        const BLUE: &str = "\x1b[34m";
        const MAGENTA: &str = "\x1b[35m";
        const CYAN: &str = "\x1b[36m";
        const LIGHT_GRAY: &str = "\x1b[37m";

        const DARK_GRAY: &str = "\x1b[90m";
        const BRIGHT_RED: &str = "\x1b[91m";
        const BRIGHT_GREEN: &str = "\x1b[92m";
        const BRIGHT_YELLOW: &str = "\x1b[93m";
        const BRIGHT_BLUE: &str = "\x1b[94m";
        const BRIGHT_MAGENTA: &str = "\x1b[95m";
        const BRIGHT_CYAN: &str = "\x1b[96m";

        let mut palette = Self {
            enabled: true,

            ansi16: true,

            reset: "\x1b[0m",

            file_type: AnsiFileTypeColors {
                file: LIGHT_GRAY.to_string(),
                directory: BRIGHT_BLUE.to_string(),
                symlink: BRIGHT_CYAN.to_string(),
                special: YELLOW.to_string(),
            },

            permissions: AnsiPermissionColors {
                read: BLUE.to_string(),
                write: CYAN.to_string(),
                execute: LIGHT_GRAY.to_string(),
                missing: DARK_GRAY.to_string(),
            },

            columns: AnsiColumnColors {
                user: BLUE.to_string(),
                size_bytes: CYAN.to_string(),
                size_units: BRIGHT_CYAN.to_string(),
                time: BRIGHT_BLUE.to_string(),
                age: MAGENTA.to_string(),
                state_normal: DARK_GRAY.to_string(),
                state_attention: BRIGHT_RED.to_string(),
            },

            names: AnsiNameColors {
                directory: BRIGHT_BLUE.to_string(),
                symlink: BRIGHT_CYAN.to_string(),
                broken_symlink: BRIGHT_RED.to_string(),
                executable: BRIGHT_GREEN.to_string(),
                special: YELLOW.to_string(),
            },

            classification: AnsiClassificationColors {
                broken_symlink: BRIGHT_RED.to_string(),
                directory: BRIGHT_BLUE.to_string(),
                symlink: BRIGHT_CYAN.to_string(),
                special: YELLOW.to_string(),

                executable: BRIGHT_GREEN.to_string(),
                source_code: BRIGHT_YELLOW.to_string(),
                shell: GREEN.to_string(),
                web: BRIGHT_MAGENTA.to_string(),

                build_config: MAGENTA.to_string(),
                structured_data: CYAN.to_string(),
                log: LIGHT_GRAY.to_string(),

                archive_package: YELLOW.to_string(),

                document: LIGHT_GRAY.to_string(),
                spreadsheet: BRIGHT_GREEN.to_string(),
                presentation: BRIGHT_MAGENTA.to_string(),

                image: MAGENTA.to_string(),
                audio: CYAN.to_string(),
                video: BRIGHT_MAGENTA.to_string(),
                font: LIGHT_GRAY.to_string(),

                database: CYAN.to_string(),
                backup: MAGENTA.to_string(),
                certificate: BRIGHT_YELLOW.to_string(),
                disk_image: BLUE.to_string(),
                torrent: GREEN.to_string(),
                desktop_plugin: MAGENTA.to_string(),

                binary: RED.to_string(),
                file: LIGHT_GRAY.to_string(),
            },

            tags: AnsiTagColors {
                filesystem: YELLOW.to_string(),
                virtual_tag: BRIGHT_YELLOW.to_string(),
                memory: BRIGHT_MAGENTA.to_string(),
                note: BRIGHT_YELLOW.to_string(),
            },

            report: AnsiReportColors {
                heading: WHITE.to_string(),
                label: BRIGHT_CYAN.to_string(),
                value: WHITE.to_string(),
                name: LIGHT_GRAY.to_string(),
                path: BRIGHT_BLUE.to_string(),
                command: BRIGHT_GREEN.to_string(),
                package: BRIGHT_CYAN.to_string(),
                version: BRIGHT_MAGENTA.to_string(),
                size: BRIGHT_YELLOW.to_string(),
                count: WHITE.to_string(),
                total: WHITE.to_string(),
                muted: DARK_GRAY.to_string(),
                attention: BRIGHT_RED.to_string(),
            },

            header: format!("{}{}", BLACK, "\x1b[44m"),

            summary: AnsiSummaryColors {
                entries: LIGHT_GRAY.to_string(),
                total_size: LIGHT_GRAY.to_string(),
                files: LIGHT_GRAY.to_string(),
                directories: BRIGHT_BLUE.to_string(),
                symlinks: BRIGHT_CYAN.to_string(),
                special: YELLOW.to_string(),
                executables: BRIGHT_GREEN.to_string(),
                empty: DARK_GRAY.to_string(),
                danger: BRIGHT_RED.to_string(),
            },
        };

        let path = ansi16_theme_paths().into_iter().find(|path| path.is_file());

        let Some(path) = path else {
            return palette;
        };

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,

            Err(error) => {
                eprintln!(
                    "noct: unable to read ANSI16 console theme {}: {}",
                    path.display(),
                    error,
                );

                return palette;
            }
        };

        let theme_file = match toml::from_str::<Ansi16ThemeFile>(&contents) {
            Ok(theme_file) => theme_file,

            Err(error) => {
                eprintln!(
                    "noct: invalid ANSI16 console theme {}: {}",
                    path.display(),
                    error,
                );

                return palette;
            }
        };

        apply_ansi16_foreground(
            &mut palette.file_type.file,
            theme_file.file_type.file,
            "file_type.file",
        );
        apply_ansi16_foreground(
            &mut palette.file_type.directory,
            theme_file.file_type.directory,
            "file_type.directory",
        );
        apply_ansi16_foreground(
            &mut palette.file_type.symlink,
            theme_file.file_type.symlink,
            "file_type.symlink",
        );
        apply_ansi16_foreground(
            &mut palette.file_type.special,
            theme_file.file_type.special,
            "file_type.special",
        );

        apply_ansi16_foreground(
            &mut palette.permissions.read,
            theme_file.permissions.read,
            "permissions.read",
        );
        apply_ansi16_foreground(
            &mut palette.permissions.write,
            theme_file.permissions.write,
            "permissions.write",
        );
        apply_ansi16_foreground(
            &mut palette.permissions.execute,
            theme_file.permissions.execute,
            "permissions.execute",
        );
        apply_ansi16_foreground(
            &mut palette.permissions.missing,
            theme_file.permissions.missing,
            "permissions.missing",
        );

        apply_ansi16_foreground(
            &mut palette.columns.user,
            theme_file.columns.user,
            "columns.user",
        );
        apply_ansi16_foreground(
            &mut palette.columns.size_bytes,
            theme_file.columns.size_bytes,
            "columns.size_bytes",
        );
        apply_ansi16_foreground(
            &mut palette.columns.size_units,
            theme_file.columns.size_units,
            "columns.size_units",
        );
        apply_ansi16_foreground(
            &mut palette.columns.time,
            theme_file.columns.time,
            "columns.time",
        );
        apply_ansi16_foreground(
            &mut palette.columns.age,
            theme_file.columns.age,
            "columns.age",
        );
        apply_ansi16_foreground(
            &mut palette.columns.state_normal,
            theme_file.columns.state_normal,
            "columns.state_normal",
        );
        apply_ansi16_foreground(
            &mut palette.columns.state_attention,
            theme_file.columns.state_attention,
            "columns.state_attention",
        );

        apply_ansi16_foreground(
            &mut palette.names.directory,
            theme_file.names.directory,
            "names.directory",
        );
        apply_ansi16_foreground(
            &mut palette.names.symlink,
            theme_file.names.symlink,
            "names.symlink",
        );
        apply_ansi16_foreground(
            &mut palette.names.broken_symlink,
            theme_file.names.broken_symlink,
            "names.broken_symlink",
        );
        apply_ansi16_foreground(
            &mut palette.names.executable,
            theme_file.names.executable,
            "names.executable",
        );
        apply_ansi16_foreground(
            &mut palette.names.special,
            theme_file.names.special,
            "names.special",
        );

        macro_rules! ansi16_classification {
            ($field:ident) => {
                apply_ansi16_foreground(
                    &mut palette.classification.$field,
                    theme_file.classification.$field,
                    concat!("classification.", stringify!($field)),
                );
            };
        }

        ansi16_classification!(broken_symlink);
        ansi16_classification!(directory);
        ansi16_classification!(symlink);
        ansi16_classification!(special);
        ansi16_classification!(executable);
        ansi16_classification!(source_code);
        ansi16_classification!(shell);
        ansi16_classification!(web);
        ansi16_classification!(build_config);
        ansi16_classification!(structured_data);
        ansi16_classification!(log);
        ansi16_classification!(archive_package);
        ansi16_classification!(document);
        ansi16_classification!(spreadsheet);
        ansi16_classification!(presentation);
        ansi16_classification!(image);
        ansi16_classification!(audio);
        ansi16_classification!(video);
        ansi16_classification!(font);
        ansi16_classification!(database);
        ansi16_classification!(backup);
        ansi16_classification!(certificate);
        ansi16_classification!(disk_image);
        ansi16_classification!(torrent);
        ansi16_classification!(desktop_plugin);
        ansi16_classification!(binary);
        ansi16_classification!(file);

        macro_rules! ansi16_tag {
            ($field:ident) => {
                apply_ansi16_foreground(
                    &mut palette.tags.$field,
                    theme_file.tags.$field,
                    concat!("tags.", stringify!($field)),
                );
            };
        }

        ansi16_tag!(filesystem);
        ansi16_tag!(virtual_tag);
        ansi16_tag!(memory);
        ansi16_tag!(note);

        macro_rules! ansi16_report {
            ($field:ident) => {
                apply_ansi16_foreground(
                    &mut palette.report.$field,
                    theme_file.report.$field,
                    concat!("report.", stringify!($field)),
                );
            };
        }

        ansi16_report!(heading);
        ansi16_report!(label);
        ansi16_report!(value);
        ansi16_report!(name);
        ansi16_report!(path);
        ansi16_report!(command);
        ansi16_report!(package);
        ansi16_report!(version);
        ansi16_report!(size);
        ansi16_report!(count);
        ansi16_report!(total);
        ansi16_report!(muted);
        ansi16_report!(attention);

        macro_rules! ansi16_summary {
            ($field:ident) => {
                apply_ansi16_foreground(
                    &mut palette.summary.$field,
                    theme_file.summary.$field,
                    concat!("summary.", stringify!($field)),
                );
            };
        }

        ansi16_summary!(entries);
        ansi16_summary!(total_size);
        ansi16_summary!(files);
        ansi16_summary!(directories);
        ansi16_summary!(symlinks);
        ansi16_summary!(special);
        ansi16_summary!(executables);
        ansi16_summary!(empty);
        ansi16_summary!(danger);

        let header_foreground = match theme_file.header.foreground {
            Some(value) => match ansi16_foreground(&value) {
                Some(color) => color.to_string(),

                None => {
                    eprintln!(
                        "noct: invalid ANSI16 color '{}' for header.foreground; using black",
                        value,
                    );

                    "\x1b[30m".to_string()
                }
            },

            None => "\x1b[30m".to_string(),
        };

        let header_background = resolved_ansi16_background(
            theme_file.header.background,
            "\x1b[43m",
            "header.background",
        );

        palette.header = format!("{}{}", header_foreground, header_background);

        palette
    }

    pub fn classification_color(&self, class: FilesystemColorClass) -> &str {
        match class {
            FilesystemColorClass::BrokenSymlink => &self.classification.broken_symlink,

            FilesystemColorClass::Directory => &self.classification.directory,

            FilesystemColorClass::Symlink => &self.classification.symlink,

            FilesystemColorClass::Special => &self.classification.special,

            FilesystemColorClass::Executable => &self.classification.executable,

            FilesystemColorClass::SourceCode => &self.classification.source_code,

            FilesystemColorClass::Shell => &self.classification.shell,

            FilesystemColorClass::Web => &self.classification.web,

            FilesystemColorClass::BuildConfig => &self.classification.build_config,

            FilesystemColorClass::StructuredData => &self.classification.structured_data,

            FilesystemColorClass::Log => &self.classification.log,

            FilesystemColorClass::ArchivePackage => &self.classification.archive_package,

            FilesystemColorClass::Document => &self.classification.document,

            FilesystemColorClass::Spreadsheet => &self.classification.spreadsheet,

            FilesystemColorClass::Presentation => &self.classification.presentation,

            FilesystemColorClass::Image => &self.classification.image,

            FilesystemColorClass::Audio => &self.classification.audio,

            FilesystemColorClass::Video => &self.classification.video,

            FilesystemColorClass::Font => &self.classification.font,

            FilesystemColorClass::Database => &self.classification.database,

            FilesystemColorClass::Backup => &self.classification.backup,

            FilesystemColorClass::Certificate => &self.classification.certificate,

            FilesystemColorClass::DiskImage => &self.classification.disk_image,

            FilesystemColorClass::Torrent => &self.classification.torrent,

            FilesystemColorClass::DesktopPlugin => &self.classification.desktop_plugin,

            FilesystemColorClass::Binary => &self.classification.binary,

            FilesystemColorClass::File => &self.classification.file,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,

            ansi16: true,

            /*
             * Reset must also be empty. Otherwise, plain piped output would
             * still contain stray escape sequences.
             */
            reset: "",

            file_type: AnsiFileTypeColors {
                file: String::new(),
                directory: String::new(),
                symlink: String::new(),
                special: String::new(),
            },

            permissions: AnsiPermissionColors {
                read: String::new(),
                write: String::new(),
                execute: String::new(),
                missing: String::new(),
            },

            columns: AnsiColumnColors {
                user: String::new(),
                size_bytes: String::new(),
                size_units: String::new(),
                time: String::new(),
                age: String::new(),
                state_normal: String::new(),
                state_attention: String::new(),
            },

            names: AnsiNameColors {
                directory: String::new(),
                symlink: String::new(),
                broken_symlink: String::new(),
                executable: String::new(),
                special: String::new(),
            },

            classification: AnsiClassificationColors {
                broken_symlink: String::new(),
                directory: String::new(),
                symlink: String::new(),
                special: String::new(),
                executable: String::new(),
                source_code: String::new(),
                shell: String::new(),
                web: String::new(),
                build_config: String::new(),
                structured_data: String::new(),
                log: String::new(),
                archive_package: String::new(),
                document: String::new(),
                spreadsheet: String::new(),
                presentation: String::new(),
                image: String::new(),
                audio: String::new(),
                video: String::new(),
                font: String::new(),
                database: String::new(),
                backup: String::new(),
                certificate: String::new(),
                disk_image: String::new(),
                torrent: String::new(),
                desktop_plugin: String::new(),
                binary: String::new(),
                file: String::new(),
            },

            tags: AnsiTagColors {
                filesystem: String::new(),
                virtual_tag: String::new(),
                memory: String::new(),
                note: String::new(),
            },

            report: AnsiReportColors {
                heading: String::new(),
                label: String::new(),
                value: String::new(),
                name: String::new(),
                path: String::new(),
                command: String::new(),
                package: String::new(),
                version: String::new(),
                size: String::new(),
                count: String::new(),
                total: String::new(),
                muted: String::new(),
                attention: String::new(),
            },

            header: String::new(),

            summary: AnsiSummaryColors {
                entries: String::new(),
                total_size: String::new(),
                files: String::new(),
                directories: String::new(),
                symlinks: String::new(),
                special: String::new(),
                executables: String::new(),
                empty: String::new(),
                danger: String::new(),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::builtin()
    }
}

fn ansi16_theme_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let platform_filename = format!("console_ansi16_{}.toml", env::consts::OS);

    if let Some(user_themes_directory) = user_themes_directory() {
        let console_directory = user_themes_directory.join("console");

        paths.push(console_directory.join(&platform_filename));
        paths.push(console_directory.join("console_ansi16.toml"));
    }

    for system_themes_directory in system_themes_directories() {
        let console_directory = system_themes_directory.join("console");

        paths.push(console_directory.join(&platform_filename));
        paths.push(console_directory.join("console_ansi16.toml"));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_color_produces_foreground_ansi() {
        let color = RgbColor::new(89, 183, 255);

        assert_eq!(color.foreground_ansi(), "\x1b[38;2;89;183;255m",);
    }

    #[test]
    fn rgb_color_produces_background_ansi() {
        let color = RgbColor::new(236, 234, 83);

        assert_eq!(color.background_ansi(), "\x1b[48;2;236;234;83m",);
    }

    #[test]
    fn rgb_color_produces_hex() {
        let color = RgbColor::new(121, 255, 170);

        assert_eq!(color.to_hex(), "#79ffaa",);
    }

    #[test]
    fn builtin_header_matches_verdant_mocha_style() {
        let theme = Theme::builtin();

        assert_eq!(
            theme.header_ansi(),
            "\x1b[1m\x1b[38;2;23;26;23m\x1b[48;2;109;181;157m",
        );
    }

    #[test]
    fn builtin_summary_entries_are_bold() {
        let theme = Theme::builtin();

        assert_eq!(
            theme.summary_entries_ansi(),
            "\x1b[1m\x1b[38;2;238;232;220m",
        );
    }

    #[test]
    fn auto_color_requires_terminal_output() {
        assert!(ColorMode::Auto.colors_enabled_for(true, false,),);

        assert!(!ColorMode::Auto.colors_enabled_for(false, false,),);
    }

    #[test]
    fn auto_color_respects_no_color() {
        assert!(!ColorMode::Auto.colors_enabled_for(true, true,),);
    }

    #[test]
    fn always_color_ignores_terminal_detection() {
        assert!(ColorMode::Always.colors_enabled_for(false, true,),);
    }

    #[test]
    fn never_color_is_always_disabled() {
        assert!(!ColorMode::Never.colors_enabled_for(true, false,),);
    }

    #[test]
    fn enabled_palette_matches_builtin_read_color() {
        let theme = Theme::builtin();

        let palette = AnsiPalette::from_theme(&theme, true);

        assert_eq!(palette.permissions.read, "\x1b[38;2;127;133;125m",);

        assert_eq!(palette.reset, "\x1b[0m",);
    }

    #[test]
    fn disabled_palette_contains_no_ansi_sequences() {
        let theme = Theme::builtin();

        let palette = AnsiPalette::from_theme(&theme, false);

        assert!(!palette.enabled);
        assert!(palette.permissions.read.is_empty());
        assert!(palette.header.is_empty());
        assert!(palette.summary.entries.is_empty());
        assert_eq!(palette.reset, "");
    }

    #[test]
    fn builtin_report_palette_matches_verdant_mocha_colors() {
        let theme = Theme::builtin();

        assert_eq!(theme.report.heading.to_hex(), "#6db59d",);

        assert_eq!(theme.report.path.to_hex(), "#68b7ae",);

        assert_eq!(theme.report.command.to_hex(), "#83b67f",);

        assert_eq!(theme.report.attention.to_hex(), "#c96f62",);
    }

    #[test]
    fn builtin_theme_is_verdant_mocha() {
        let theme = Theme::builtin();

        assert_eq!(theme.name, "verdant_mocha");
    }
}

fn apply_color(target: &mut RgbColor, value: Option<String>, field_name: &str) {
    let Some(value) = value else {
        return;
    };

    match RgbColor::from_hex(&value) {
        Ok(color) => {
            *target = color;
        }

        Err(error) => {
            eprintln!("noct: invalid theme color {}: {}", field_name, error,);
        }
    }
}
