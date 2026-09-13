// SPDX-License-Identifier: BSD-3-Clause

use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;

/*
 * Detailed filesystem classification.
 *
 * This layer preserves useful semantic information even when several classes
 * later share one visual filesystem color.
 *
 * Ordinary listing classification is deliberately cheap: metadata, filename,
 * and extension only. No file contents are opened or inspected here.
 */
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryClass {
    BrokenSymlink,
    Directory,
    Symlink,

    Socket,
    Fifo,
    BlockDevice,
    CharDevice,

    Executable,
    ShellScript,

    Rust,
    Python,
    C,
    Cpp,
    Java,
    Kotlin,
    JavaScript,
    TypeScript,
    Assembly,
    Lua,
    Ruby,
    Perl,
    Php,
    Go,
    Swift,
    Dart,
    CSharp,
    Scala,
    Groovy,
    R,
    Awk,
    Elixir,
    Erlang,
    FSharp,
    VisualBasic,
    Clojure,
    Zig,
    Nim,
    Crystal,
    Haskell,
    Ocaml,
    Pascal,
    Solidity,
    Vala,
    Web,
    SourceCode,

    Build,
    Config,
    StructuredData,
    Log,

    Archive,
    Package,

    Document,
    Spreadsheet,
    Presentation,

    Image,
    VectorImage,
    Audio,
    Video,
    Font,

    Database,
    Torrent,
    DesktopEntry,
    Backup,
    Certificate,
    DiskImage,
    Plugin,

    Text,
    Binary,

    File,
}

/*
 * Visual filesystem color groups.
 *
 * Detailed EntryClass values remain available for icon selection and future
 * filesystem-aware commands, while this smaller layer keeps theme authoring
 * practical.
 *
 * A filename and its icon will eventually use the same resolved color from
 * this classification.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemColorClass {
    BrokenSymlink,
    Directory,
    Symlink,
    Special,

    Executable,
    SourceCode,
    Shell,
    Web,

    BuildConfig,
    StructuredData,
    Log,

    ArchivePackage,
    Document,
    Spreadsheet,
    Presentation,

    Image,
    Audio,
    Video,
    Font,

    Database,
    Backup,
    Certificate,
    DiskImage,
    Torrent,
    DesktopPlugin,

    Binary,
    File,
}

impl EntryClass {
    /*
     * Collapse the detailed semantic classifier into the smaller set of visual
     * filesystem colors.
     *
     * Icon identity remains free to use the complete EntryClass instead.
     */
    pub fn color_class(self) -> FilesystemColorClass {
        match self {
            Self::BrokenSymlink => FilesystemColorClass::BrokenSymlink,

            Self::Directory => FilesystemColorClass::Directory,

            Self::Symlink => FilesystemColorClass::Symlink,

            Self::Socket | Self::Fifo | Self::BlockDevice | Self::CharDevice => {
                FilesystemColorClass::Special
            }

            Self::Executable => FilesystemColorClass::Executable,

            Self::ShellScript | Self::Awk => FilesystemColorClass::Shell,

            Self::Rust
            | Self::Python
            | Self::C
            | Self::Cpp
            | Self::Java
            | Self::Kotlin
            | Self::JavaScript
            | Self::TypeScript
            | Self::Assembly
            | Self::Lua
            | Self::Ruby
            | Self::Perl
            | Self::Php
            | Self::Go
            | Self::Swift
            | Self::Dart
            | Self::CSharp
            | Self::Scala
            | Self::Groovy
            | Self::R
            | Self::Elixir
            | Self::Erlang
            | Self::FSharp
            | Self::VisualBasic
            | Self::Clojure
            | Self::Zig
            | Self::Nim
            | Self::Crystal
            | Self::Haskell
            | Self::Ocaml
            | Self::Pascal
            | Self::Solidity
            | Self::Vala
            | Self::SourceCode => FilesystemColorClass::SourceCode,

            Self::Web => FilesystemColorClass::Web,

            Self::Build | Self::Config => FilesystemColorClass::BuildConfig,

            Self::StructuredData => FilesystemColorClass::StructuredData,

            Self::Log => FilesystemColorClass::Log,

            Self::Archive | Self::Package => FilesystemColorClass::ArchivePackage,

            Self::Document | Self::Text => FilesystemColorClass::Document,

            Self::Spreadsheet => FilesystemColorClass::Spreadsheet,

            Self::Presentation => FilesystemColorClass::Presentation,

            Self::Image | Self::VectorImage => FilesystemColorClass::Image,

            Self::Audio => FilesystemColorClass::Audio,

            Self::Video => FilesystemColorClass::Video,

            Self::Font => FilesystemColorClass::Font,

            Self::Database => FilesystemColorClass::Database,

            Self::Backup => FilesystemColorClass::Backup,

            Self::Certificate => FilesystemColorClass::Certificate,

            Self::DiskImage => FilesystemColorClass::DiskImage,

            Self::Torrent => FilesystemColorClass::Torrent,

            Self::DesktopEntry | Self::Plugin => FilesystemColorClass::DesktopPlugin,

            Self::Binary => FilesystemColorClass::Binary,

            Self::File => FilesystemColorClass::File,
        }
    }
}

pub fn classify_entry(path: &Path, name: &str, metadata: &fs::Metadata) -> EntryClass {
    let file_type = metadata.file_type();

    /*
     * Structural filesystem identity is authoritative.
     *
     * A broken symbolic link must be distinguished before any attempt is made
     * to classify its pathname.
     */
    if file_type.is_symlink() && fs::metadata(path).is_err() {
        return EntryClass::BrokenSymlink;
    }

    if metadata.is_dir() {
        return EntryClass::Directory;
    }

    if file_type.is_symlink() {
        return EntryClass::Symlink;
    }

    /*
     * Unix special files must never acquire a semantic class merely because
     * their pathname resembles an ordinary filename.
     */
    if file_type.is_socket() {
        return EntryClass::Socket;
    }

    if file_type.is_fifo() {
        return EntryClass::Fifo;
    }

    if file_type.is_block_device() {
        return EntryClass::BlockDevice;
    }

    if file_type.is_char_device() {
        return EntryClass::CharDevice;
    }

    let lowercase_filename = name.to_lowercase();

    /*
     * Specific filename knowledge outranks generic extension and executable
     * status.
     */
    if let Some(class) = classify_special_filename(&lowercase_filename) {
        return class;
    }

    /*
     * Recognize meaningful multipart suffixes before Path::extension() reduces
     * them to only their final component.
     */
    if let Some(class) = classify_compound_extension(&lowercase_filename) {
        return class;
    }

    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if let Some(class) = classify_extension(&extension) {
        return class;
    }

    /*
     * Executability is intentionally checked after semantic filename and
     * extension classification.
     *
     * An executable main.rs remains Rust; an otherwise unknown executable
     * becomes Executable.
     */
    if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
        return EntryClass::Executable;
    }

    if looks_like_backup_name(&lowercase_filename) {
        return EntryClass::Backup;
    }

    EntryClass::File
}

fn classify_special_filename(filename: &str) -> Option<EntryClass> {
    let class = match filename {
        /*
         * Rust ecosystem.
         */
        "cargo.toml" | "cargo.lock" | "rust-toolchain" | "rust-toolchain.toml" => EntryClass::Rust,

        /*
         * Build systems and project machinery.
         */
        "makefile"
        | "gnumakefile"
        | "bsdmakefile"
        | "cmakelists.txt"
        | "meson.build"
        | "meson_options.txt"
        | "build.gradle"
        | "build.gradle.kts"
        | "settings.gradle"
        | "settings.gradle.kts"
        | "gradlew"
        | "gradlew.bat"
        | "justfile"
        | "rakefile"
        | "dockerfile"
        | "containerfile"
        | "vagrantfile"
        | "procfile" => EntryClass::Build,

        /*
         * Shell and application configuration.
         */
        ".bashrc" | ".bash_profile" | ".bash_login" | ".bash_logout" | ".zshrc" | ".zprofile"
        | ".zlogin" | ".zlogout" | ".zshenv" | ".profile" | ".kshrc" | ".cshrc" | ".tcshrc"
        | "config.fish" | "nanorc" | ".nanorc" | ".vimrc" | ".gvimrc" | ".editorconfig"
        | ".gitconfig" | ".gitignore" | ".gitattributes" | ".gitmodules" | ".npmrc" | ".yarnrc"
        | ".yarnrc.yml" | ".clang-format" | ".clang-tidy" | ".dockerignore" => EntryClass::Config,

        ".bash_history" | ".zsh_history" | ".history" | ".sh_history" => EntryClass::Log,

        /*
         * Familiar documentation files without useful extensions.
         */
        "readme" | "readme.txt" | "readme.md" | "license" | "license.txt" | "licence"
        | "copying" | "authors" | "contributors" | "changelog" | "changes" | "news" | "todo" => {
            EntryClass::Document
        }

        _ => {
            return None;
        }
    };

    Some(class)
}

fn classify_compound_extension(filename: &str) -> Option<EntryClass> {
    let class = if filename.ends_with(".tar.gz")
        || filename.ends_with(".tar.bz2")
        || filename.ends_with(".tar.xz")
        || filename.ends_with(".tar.zst")
        || filename.ends_with(".tar.lz")
        || filename.ends_with(".tar.lzma")
        || filename.ends_with(".tar.lzo")
        || filename.ends_with(".tar.lz4")
        || filename.ends_with(".tar.br")
        || filename.ends_with(".tar.z")
        || filename.ends_with(".cpio.gz")
        || filename.ends_with(".cpio.xz")
        || filename.ends_with(".cpio.zst")
    {
        EntryClass::Archive
    } else if filename.ends_with(".user.js") || filename.ends_with(".min.js") {
        EntryClass::JavaScript
    } else if filename.ends_with(".d.ts")
        || filename.ends_with(".spec.ts")
        || filename.ends_with(".test.ts")
        || filename.ends_with(".spec.tsx")
        || filename.ends_with(".test.tsx")
    {
        EntryClass::TypeScript
    } else if filename.ends_with(".spec.js")
        || filename.ends_with(".test.js")
        || filename.ends_with(".spec.jsx")
        || filename.ends_with(".test.jsx")
    {
        EntryClass::JavaScript
    } else if filename.ends_with(".blade.php") {
        EntryClass::Web
    } else if filename.ends_with(".desktop.in") {
        EntryClass::DesktopEntry
    } else if filename.ends_with(".service.in") || filename.ends_with(".conf.in") {
        EntryClass::Config
    } else if filename.ends_with(".bak")
        || filename.ends_with(".backup")
        || filename.ends_with(".old")
        || filename.ends_with(".orig")
        || filename.ends_with(".save")
    {
        EntryClass::Backup
    } else {
        return None;
    };

    Some(class)
}

fn classify_extension(extension: &str) -> Option<EntryClass> {
    let class = match extension {
        /*
         * Programming languages.
         */
        "rs" => EntryClass::Rust,

        "py" | "pyw" | "pyi" => EntryClass::Python,

        "c" | "h" => EntryClass::C,

        "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" => EntryClass::Cpp,

        "java" => EntryClass::Java,

        "kt" | "kts" => EntryClass::Kotlin,

        "js" | "jsx" | "mjs" | "cjs" => EntryClass::JavaScript,

        "ts" | "tsx" | "mts" | "cts" => EntryClass::TypeScript,

        "asm" | "s" | "inc" | "nasm" => EntryClass::Assembly,

        "lua" => EntryClass::Lua,

        "rb" | "rake" | "gemspec" => EntryClass::Ruby,

        "pl" | "pm" | "pod" | "t" => EntryClass::Perl,

        "php" | "php3" | "php4" | "php5" | "php7" | "php8" | "phtml" => EntryClass::Php,

        "go" => EntryClass::Go,

        "swift" => EntryClass::Swift,

        "dart" => EntryClass::Dart,

        "cs" | "csx" => EntryClass::CSharp,

        "scala" | "sc" => EntryClass::Scala,

        "groovy" | "gvy" | "gy" | "gsh" => EntryClass::Groovy,

        "r" | "rmd" | "rnw" => EntryClass::R,

        "awk" => EntryClass::Awk,

        "ex" | "exs" => EntryClass::Elixir,

        "erl" | "hrl" => EntryClass::Erlang,

        "fs" | "fsx" | "fsi" | "fsscript" => EntryClass::FSharp,

        "vb" | "vbs" => EntryClass::VisualBasic,

        "clj" | "cljs" | "cljc" | "edn" => EntryClass::Clojure,

        "zig" => EntryClass::Zig,

        "nim" | "nims" | "nimble" => EntryClass::Nim,

        "cr" => EntryClass::Crystal,

        "hs" | "lhs" => EntryClass::Haskell,

        "ml" | "mli" => EntryClass::Ocaml,

        "pas" | "pp" | "p" => EntryClass::Pascal,

        "sol" => EntryClass::Solidity,

        "vala" | "vapi" => EntryClass::Vala,

        /*
         * Shell and command scripts.
         */
        "sh" | "bash" | "zsh" | "ash" | "dash" | "ksh" | "csh" | "tcsh" | "fish" | "command" => {
            EntryClass::ShellScript
        }

        /*
         * Web.
         */
        "html" | "htm" | "xhtml" | "css" | "scss" | "sass" | "less" | "vue" | "svelte"
        | "astro" => EntryClass::Web,

        /*
         * Build and dependency files.
         */
        "mk" | "cmake" | "ninja" | "gradle" | "d" | "dep" | "mak" | "sln" | "vcxproj"
        | "csproj" | "fsproj" | "xcodeproj" | "lo" | "rlib" | "pdb" | "beam" => EntryClass::Build,

        /*
         * Configuration.
         */
        "conf" | "config" | "cfg" | "ini" | "toml" | "properties" | "prefs" | "rc" | "cnf"
        | "service" | "socket" | "timer" | "mount" | "target" | "rules" | "policy" | "env"
        | "editorconfig" => EntryClass::Config,

        /*
         * Structured data.
         */
        "json" | "jsonc" | "json5" | "yaml" | "yml" | "xml" | "xsd" | "xsl" | "xslt" | "csv"
        | "tsv" | "ndjson" | "geojson" | "plist" | "ron" | "msgpack" | "cbor" => {
            EntryClass::StructuredData
        }

        /*
         * Logs and diagnostics.
         */
        "log" | "trace" | "out" | "err" | "dump" | "stacktrace" => EntryClass::Log,

        /*
         * Archives and compression.
         */
        "zip" | "7z" | "rar" | "tar" | "tgz" | "tbz" | "tbz2" | "txz" | "gz" | "bz2" | "xz"
        | "zst" | "lz" | "lz4" | "lzma" | "lzo" | "cab" | "ar" | "cpio" => EntryClass::Archive,

        /*
         * Installable/software packages.
         */
        "deb" | "rpm" | "pkg" | "apk" | "appimage" | "flatpak" | "snap" | "msi" | "exe" | "whl"
        | "gem" | "crate" | "jar" | "war" | "ear" => EntryClass::Package,

        /*
         * Documents.
         */
        "txt" | "md" | "markdown" | "rst" | "adoc" | "asciidoc" | "org" | "tex" | "pdf"
        | "djvu" | "epub" | "mobi" | "azw" | "azw3" | "doc" | "docx" | "odt" | "rtf" | "pages"
        | "man" | "info" => EntryClass::Document,

        /*
         * Spreadsheets.
         */
        "xls" | "xlsx" | "xlsm" | "ods" | "numbers" | "gnumeric" => EntryClass::Spreadsheet,

        /*
         * Presentations.
         */
        "ppt" | "pptx" | "pptm" | "odp" | "key" => EntryClass::Presentation,

        /*
         * Raster images.
         */
        "png" | "jpg" | "jpeg" | "jpe" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "avif"
        | "heif" | "heic" | "ico" | "xcf" | "psd" | "raw" | "cr2" | "cr3" | "nef" | "arw"
        | "dng" => EntryClass::Image,

        /*
         * Vector/design images.
         */
        "svg" | "svgz" | "eps" | "ai" | "cdr" | "wmf" | "emf" => EntryClass::VectorImage,

        /*
         * Audio.
         */
        "mp3" | "ogg" | "oga" | "opus" | "flac" | "wav" | "wave" | "m4a" | "aac" | "wma"
        | "aiff" | "aif" | "mid" | "midi" | "ape" | "alac" | "m3u" | "m3u8" => EntryClass::Audio,

        /*
         * Video.
         */
        "mp4" | "m4v" | "mkv" | "webm" | "avi" | "mov" | "mpg" | "mpeg" | "mpe" | "wmv" | "flv"
        | "ogv" | "3gp" | "m2ts" | "vob" => EntryClass::Video,

        /*
         * Fonts.
         */
        "ttf" | "otf" | "woff" | "woff2" | "eot" | "fon" | "pcf" | "bdf" => EntryClass::Font,

        /*
         * Databases.
         */
        "db" | "sqlite" | "sqlite3" | "mdb" | "accdb" | "sql" | "dbf" | "realm" => {
            EntryClass::Database
        }

        "torrent" => EntryClass::Torrent,

        "desktop" => EntryClass::DesktopEntry,

        /*
         * Backups and temporary copies.
         */
        "bak" | "backup" | "old" | "orig" | "save" | "tmp" | "temp" | "swp" | "swo" => {
            EntryClass::Backup
        }

        /*
         * Certificates, keys, and signatures.
         */
        "pem" | "crt" | "cer" | "der" | "p12" | "pfx" | "csr" | "pub" | "asc" | "sig" | "gpg"
        | "pgp" => EntryClass::Certificate,

        /*
         * Disk, virtual-machine, and optical images.
         */
        "iso" | "img" | "dmg" | "vhd" | "vhdx" | "vdi" | "vmdk" | "qcow" | "qcow2" | "ova"
        | "ovf" | "bin" | "cue" => EntryClass::DiskImage,

        /*
         * Shared libraries, plugins, and generated binary objects.
         */
        "so" | "dylib" | "dll" | "ko" | "a" | "lib" | "o" | "obj" | "class" | "pyc" | "pyo"
        | "wasm" | "plugin" => EntryClass::Plugin,

        /*
         * Plain text whose role is not otherwise known.
         */
        "text" | "nfo" | "dic" | "dict" | "words" => EntryClass::Text,

        _ => {
            return None;
        }
    };

    Some(class)
}

fn looks_like_backup_name(filename: &str) -> bool {
    filename.ends_with('~')
        || filename.starts_with(".#")
        || filename.starts_with('#') && filename.ends_with('#')
}

pub fn standard_filesystem_color(color_class: FilesystemColorClass) -> &'static str {
    match color_class {
        FilesystemColorClass::BrokenSymlink => "\x1b[38;2;255;80;100m",
        FilesystemColorClass::Directory => "\x1b[38;2;80;155;235m",
        FilesystemColorClass::Symlink => "\x1b[38;2;75;195;210m",
        FilesystemColorClass::Special => "\x1b[38;2;210;150;95m",
        FilesystemColorClass::Executable => "\x1b[38;2;125;220;150m",
        FilesystemColorClass::SourceCode => "\x1b[38;2;235;205;105m",
        FilesystemColorClass::Shell => "\x1b[38;2;125;220;150m",
        FilesystemColorClass::Web => "\x1b[38;2;235;145;215m",
        FilesystemColorClass::BuildConfig => "\x1b[38;2;195;170;235m",
        FilesystemColorClass::StructuredData => "\x1b[38;2;120;215;220m",
        FilesystemColorClass::Log => "\x1b[38;2;175;190;210m",
        FilesystemColorClass::ArchivePackage => "\x1b[38;2;235;165;105m",
        FilesystemColorClass::Document => "\x1b[38;2;225;215;180m",
        FilesystemColorClass::Spreadsheet => "\x1b[38;2;125;220;165m",
        FilesystemColorClass::Presentation => "\x1b[38;2;245;165;110m",
        FilesystemColorClass::Image => "\x1b[38;2;235;145;215m",
        FilesystemColorClass::Audio => "\x1b[38;2;120;220;185m",
        FilesystemColorClass::Video => "\x1b[38;2;200;150;235m",
        FilesystemColorClass::Font => "\x1b[38;2;215;175;240m",
        FilesystemColorClass::Database => "\x1b[38;2;110;210;205m",
        FilesystemColorClass::Backup => "\x1b[38;2;205;165;220m",
        FilesystemColorClass::Certificate => "\x1b[38;2;245;210;115m",
        FilesystemColorClass::DiskImage => "\x1b[38;2;155;185;235m",
        FilesystemColorClass::Torrent => "\x1b[38;2;120;215;170m",
        FilesystemColorClass::DesktopPlugin => "\x1b[38;2;185;165;235m",
        FilesystemColorClass::Binary => "\x1b[38;2;225;150;160m",
        FilesystemColorClass::File => "\x1b[38;2;210;215;225m",
    }
}
