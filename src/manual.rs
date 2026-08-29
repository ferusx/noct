use crate::format::{terminal_size, terminal_width};
use crate::themes::AnsiPalette;
use std::sync::atomic::{AtomicBool, Ordering};

use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::Duration;

const MIN_INNER_WIDTH: usize = 40;
const MAX_INNER_WIDTH: usize = 96;

/*
 * The manual deliberately uses its own fixed palette rather than the
 * currently selected Noct theme.
 */
const MANUAL_ACCENT: &str = "\x1b[38;2;185;190;210m"; // The ACCENT (light gray) color
const MANUAL_TEXT: &str = "\x1b[38;2;110;110;128m"; // The TEXT (dark gray) color
const MANUAL_EMPHASIS: &str = "\x1b[38;2;255;255;255m"; // The EMPHASIS (white)
const MANUAL_TITLE: &str = "\x1b[1;38;2;255;255;255m"; // The EMPHASIS (bold+white)

const MANUAL_EXAMPLE: &str = "\x1b[38;2;146;220;72m"; // The EXAMPLE (green) color
const MANUAL_OPTION: &str = "\x1b[1;38;2;146;220;72m"; // The EXAMPLE (bold+green) color

const MANUAL_BORDER: &str = "\x1b[38;2;78;86;98m"; // The BORDER (gray) color
const MANUAL_RESET: &str = "\x1b[0m";
const MANUAL_MIN_WIDTH: usize = 100;
const MANUAL_MIN_HEIGHT: usize = 16;

const MANUAL_RESIZE_GREEN: &str = "\x1b[38;2;90;220;120m";
const MANUAL_RESIZE_WHITE: &str = "\x1b[38;2;235;235;240m";
const MANUAL_RESIZE_RED: &str = "\x1b[38;2;255;85;100m";
const MANUAL_RESIZE_MUTED: &str = "\x1b[38;2;120;128;142m";
static MANUAL_RESIZE_INTERRUPTED: AtomicBool = AtomicBool::new(false);
const MANUAL_LOGO_COLORS: [&str; 6] = [
    "\x1b[38;2;245;245;245m",
    "\x1b[38;2;205;205;205m",
    "\x1b[38;2;165;165;165m",
    "\x1b[38;2;125;125;125m",
    "\x1b[38;2;85;85;85m",
    "\x1b[38;2;50;50;50m",
];

const MANUAL_LOGO: [&str; 6] = [
    "███╗   ██╗ ██████╗  ██████╗████████╗",
    "████╗  ██║██╔═══██╗██╔════╝╚══██╔══╝",
    "██╔██╗ ██║██║   ██║██║        ██║",
    "██║╚██╗██║██║   ██║██║        ██║",
    "██║ ╚████║╚██████╔╝╚██████╗   ██║",
    "╚═╝  ╚═══╝ ╚═════╝  ╚═════╝   ╚═╝",
];

extern "C" fn manual_resize_sigint_handler(_: libc::c_int) {
    MANUAL_RESIZE_INTERRUPTED.store(true, Ordering::SeqCst);
}

struct ManualColors {
    accent: &'static str,
    text: &'static str,
    emphasis: &'static str,
    title: &'static str,
    example: &'static str,
    option: &'static str,
    border: &'static str,
    reset: &'static str,
}

impl ManualColors {
    fn new(enabled: bool) -> Self {
        if !enabled {
            return Self {
                accent: "",
                text: "",
                emphasis: "",
                title: "",
                example: "",
                option: "",
                border: "",
                reset: "",
            };
        }

        Self {
            accent: MANUAL_ACCENT,
            text: MANUAL_TEXT,
            emphasis: MANUAL_EMPHASIS,
            title: MANUAL_TITLE,
            example: MANUAL_EXAMPLE,
            option: MANUAL_OPTION,
            border: MANUAL_BORDER,
            reset: MANUAL_RESET,
        }
    }
}
struct ManualPart<'a> {
    text: &'a str,
    color: &'a str,
}

pub fn print_manual(palette: &AnsiPalette) {
    wait_for_manual_terminal_size(palette.enabled);

    let inner_width = manual_inner_width();

    let colors = ManualColors::new(palette.enabled);

    print_top_border(inner_width, &colors);

    print_line("Noct Manual", inner_width, colors.title, &colors);

    print_line(
        "noctilucent filesystem inspection tool",
        inner_width,
        colors.text,
        &colors,
    );

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_line("About Noct", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Noct",
                color: colors.accent,
            },
            ManualPart {
                text: "is a command-line",
                color: "",
            },
            ManualPart {
                text: "filesystem inspection tool",
                color: colors.accent,
            },
            ManualPart {
                text: "for Unix-like systems. It can examine a directory from a lot of different angles: \
sizes, ownership, permissions, age, extensions, scripts, links, duplicate files, filesystem \
boundaries, cleanup candidates, and plenty more.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Most of Noct's special modes are made to answer one useful filesystem question at a time.",
                color: "",
            },
            ManualPart {
                text: "--inspect",
                color: colors.emphasis,
            },
            ManualPart {
                text: "takes a close look at one entry,",
                color: "",
            },
            ManualPart {
                text: "--duplicates",
                color: colors.emphasis,
            },
            ManualPart {
                text: "finds files that really contain the same data,",
                color: "",
            },
            ManualPart {
                text: "--owners",
                color: colors.emphasis,
            },
            ManualPart {
                text: "shows who owns what, and modes such as",
                color: "",
            },
            ManualPart {
                text: "--hot,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "--cold,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "--graveyard",
                color: colors.emphasis,
            },
            ManualPart {
                text: "help you look at the filesystem in ways that would otherwise take several separate commands.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Noct",
                color: colors.accent,
            },
            ManualPart {
                text: "can also do the ordinary job of",
                color: "",
            },
            ManualPart {
                text: "listing files and directories.",
                color: colors.accent,
            },
            ManualPart {
                text: "Run",
                color: "",
            },
            ManualPart {
                text: "noct",
                color: colors.example,
            },
            ManualPart {
                text: "or its shorter",
                color: "",
            },
            ManualPart {
                text: "nt",
                color: colors.example,
            },
            ManualPart {
                text: "alias and you get a compact everyday directory view with sorting, filtering, metadata, \
colors, long listings, and Tree output when you want it.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "The pages below walk through Noct's special modes one by one, explain what they are good for, \
and point out the extra options that make sense with each of them. For the short version, use",
                color: "",
            },
            ManualPart {
                text: "nt --help.",
                color: colors.example,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    /* =============================================================
                              Meta Options
    ============================================================= */

    print_blank_line(inner_width, &colors);

    print_line("Meta Options", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_line("--manual", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show this explanatory Noct manual. It provides a guided reference to ordinary listing options, \
filters, fields, and Noct's specialized inspection modes.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --manual",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-?, --help", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show Noct's compact command reference. Unlike",
                color: "",
            },
            ManualPart {
                text: "--manual,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "the help view is intended as a quicker lookup for available options, fields, and command forms.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --help",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-v, --version", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Print the installed Noct version together with basic program information.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --version",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Display and Listing", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_line("-a, --all", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Include hidden dotfiles in the listing together with the special",
                color: "",
            },
            ManualPart {
                text: ". and ..",
                color: colors.accent,
            },
            ManualPart {
                text: "directory entries. Use this when you want the most complete view of a directory.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -a ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-A, --almost-all", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Include hidden dotfiles but leave out the special",
                color: "",
            },
            ManualPart {
                text: ". and ..",
                color: colors.accent,
            },
            ManualPart {
                text: "entries. This is usually the cleaner choice when hidden files matter but directory navigation entries do not.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -A ~/.config",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-h, --header", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show column headings above the long table view. The header labels the displayed metadata columns and is especially useful when several long-view fields are enabled.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-l, --long", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Display entries in a detailed table instead of the compact grid. Long view can show permissions, owner, size, timestamp, age, state, and name, making it the",
                color: "",
            },
            ManualPart {
                text: "metadata-rich listing mode",
                color: colors.accent,
            },
            ManualPart {
                text: "for everyday inspection.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Several display options can remove individual columns when a leaner table is preferred; those controls are described below in this section.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh -t created ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-1, --oneline", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Display one entry per line instead of arranging names into the compact grid. This is useful when \
you want a simple vertical listing that is easy to scan, pipe, or process further.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -1 ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-d, --directory", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "List the supplied directory itself instead of listing the entries inside it. This lets a directory \
be treated as the item being inspected, just like an ordinary file.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "It is especially useful together with",
                color: "",
            },
            ManualPart {
                text: "--long",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when you want the directory's own metadata rather than the metadata of its contents.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -ld ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -ld Cargo.toml",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--no-colors", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Disable colored output for the current invocation. The layout and information remain unchanged, \
but Noct emits plain terminal text without ANSI color styling.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "This is useful for",
                color: "",
            },
            ManualPart {
                text: "plain-text output, scripts, logs, and terminals where color is unwanted.",
                color: colors.accent,
            },
            ManualPart {
                text: "Noct also suppresses color automatically when appropriate for non-interactive output.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Projects --no-colors",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--no-age", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Hide the AGE column from long view. Use this when relative file age is not useful and you want a slightly leaner table.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--no-date", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Hide the timestamp column from long view. This removes the selected modified, accessed, or created time from the table.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--no-user", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Hide the USER column from long view. This is useful when ownership is not relevant to the current inspection.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--no-perms", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Hide the MODE column from long view. The file type and permission bits are omitted while the remaining metadata stays unchanged.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--no-state", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Hide the STATE column from long view. Detected file states are still available to filtering and other Noct features; only the displayed column is removed.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --no-user --no-date --no-age",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh /usr/local/bin --no-perms --no-state",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--state", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show the detected file state alongside entries in grid and oneline views. This makes conditions such as empty files, broken links, or attention-worthy permission states visible without switching to long view.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The available state names and their meanings are described later in the",
                color: "",
            },
            ManualPart {
                text: "File States",
                color: colors.accent,
            },
            ManualPart {
                text: "section.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --state ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--limit N", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Limit the listing to at most N entries. Filtering and sorting happen first, so the limit is applied to the final ordered result rather than to arbitrary directory entries.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Downloads --limit=20",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--tail N", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Keep the last N entries after filtering and sorting. This is the counterpart to",
                color: "",
            },
            ManualPart {
                text: "--limit:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "instead of taking entries from the beginning of the ordered result, it keeps entries from the end.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Downloads --sort age --tail 15",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-w, --wide", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Use more generous spacing between displayed columns. In grid view this increases the gap between name columns; in long view it adds more breathing room between metadata fields.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use it when you prefer a",
                color: "",
            },
            ManualPart {
                text: "more spacious listing",
                color: colors.accent,
            },
            ManualPart {
                text: "and have enough terminal width available.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lw ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-W, --width COLS", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Format grid output for a specific column width instead of using the detected terminal width. This is useful when preparing output for a known layout or when terminal width detection is not what you want.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "This option controls",
                color: "",
            },
            ManualPart {
                text: "grid formatting width",
                color: colors.accent,
            },
            ManualPart {
                text: "rather than truncating filenames or changing the actual terminal size.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -W 120 ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Filtering", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_line("--big", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show large regular files. The built-in threshold is",
                color: "",
            },
            ManualPart {
                text: "10 MiB",
                color: colors.accent,
            },
            ManualPart {
                text: "so this is a quick way to narrow an ordinary listing to entries that occupy a more noticeable amount of space.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "For a custom size threshold, use",
                color: "",
            },
            ManualPart {
                text: "--greater-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "instead.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Downloads --big",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--danger", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show only entries whose detected state is not normal. This gathers attention-worthy filesystem conditions into one filtered view instead of requiring you to search for each state separately.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--state",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or long view when you also want the detected state displayed beside each result.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Downloads --danger",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--empty, --zero", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show zero-byte regular files.",
                color: "",
            },
            ManualPart {
                text: "--empty",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "--zero",
                color: colors.emphasis,
            },
            ManualPart {
                text: "are aliases for the same filter.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "This is useful for finding placeholder files, abandoned output files, or other regular files that currently contain no data.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lA ~/Projects --empty",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--executables", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show executable regular files. A file matches when at least one execute permission bit is set.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use this when you want to narrow a directory to",
                color: "",
            },
            ManualPart {
                text: "runnable file entries",
                color: colors.accent,
            },
            ManualPart {
                text: "without including ordinary non-executable files.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l /usr/local/bin --executables",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line(
        "-f, --filter KIND VALUE",
        inner_width,
        colors.option,
        &colors,
    );

    print_indented_paragraph(
        &[ManualPart {
            text: "Apply the general filtering interface to ordinary listings. The filter kind selects what property to match, and the following value supplies the wanted match.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Supported filter kinds are",
                color: "",
            },
            ManualPart {
                text: "extension",
                color: colors.accent,
            },
            ManualPart {
                text: "with",
                color: "",
            },
            ManualPart {
                text: "ext",
                color: colors.accent,
            },
            ManualPart {
                text: "as its shorter alias, and",
                color: "",
            },
            ManualPart {
                text: "type",
                color: colors.accent,
            },
            ManualPart {
                text: "for selecting entries by filesystem type. Filter kinds and their accepted values are summarized later in this section.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh -f ext rs ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh -f type directory ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--fresh", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries modified within the last",
                color: "",
            },
            ManualPart {
                text: "24 hours.",
                color: colors.accent,
            },
            ManualPart {
                text: "This is a convenient fixed-time filter for finding recently changed material without supplying an explicit age.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "For a custom time window, use",
                color: "",
            },
            ManualPart {
                text: "--newer-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "--since.",
                color: colors.emphasis,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --fresh",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--greater-than SIZE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show regular files whose size is at least SIZE. This provides a custom threshold when the fixed",
                color: "",
            },
            ManualPart {
                text: "--big",
                color: colors.emphasis,
            },
            ManualPart {
                text: "filter is too broad or too restrictive.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Sizes may use forms such as",
                color: "",
            },
            ManualPart {
                text: "100K, 10M, 2G,",
                color: colors.accent,
            },
            ManualPart {
                text: "or equivalent binary-style suffixes.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Downloads --greater-than=100M",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--less-than SIZE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show regular files whose size is at most SIZE. This is the counterpart to",
                color: "",
            },
            ManualPart {
                text: "--greater-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and is useful when you want to isolate smaller files below a chosen threshold.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Downloads --less-than 100K",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--newer-than AGE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries modified within the supplied age window. Unlike",
                color: "",
            },
            ManualPart {
                text: "--fresh,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "this lets you choose the time span instead of using the fixed 24-hour window.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Age values may use minutes, hours, days, weeks, months, or years; the accepted format is summarized later under",
                color: "",
            },
            ManualPart {
                text: "Age Format.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --newer-than=3d",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--older-than AGE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries whose modification time is at least the supplied age. This is the opposite of",
                color: "",
            },
            ManualPart {
                text: "--newer-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and is useful for narrowing a listing to older or long-untouched material.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Downloads --older-than 6mo",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--only-state STATE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show only entries whose detected state matches STATE. This turns Noct's filesystem state detection into a direct filter instead of merely displaying the state beside each entry.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The available state names and their meanings are described in the",
                color: "",
            },
            ManualPart {
                text: "File States",
                color: colors.accent,
            },
            ManualPart {
                text: "section.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Downloads --only-state broken-link",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--orphans", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show entries whose filesystem references no longer resolve cleanly. This includes broken symbolic links and entries whose stored user or group IDs no longer correspond to known local accounts.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use this when you want a focused view of",
                color: "",
            },
            ManualPart {
                text: "dangling links and missing ownership references",
                color: colors.accent,
            },
            ManualPart {
                text: "without mixing them with every other attention-worthy state.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --orphans",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--permissions MODE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries whose permission bits match MODE exactly. The mode is written in octal form, such as",
                color: "",
            },
            ManualPart {
                text: "755",
                color: colors.accent,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "0644.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Use this when you want to isolate entries with one specific permission layout rather than broader conditions such as writable or world-writable.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l /usr/local/bin --permissions 755",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--stale", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries modified more than",
                color: "",
            },
            ManualPart {
                text: "30 days ago.",
                color: colors.accent,
            },
            ManualPart {
                text: "This is the fixed-age counterpart to",
                color: "",
            },
            ManualPart {
                text: "--fresh",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and provides a quick way to focus on older material.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "For a custom age threshold, use",
                color: "",
            },
            ManualPart {
                text: "--older-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "instead.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Downloads --stale",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--tiny", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show tiny regular files. The built-in maximum is",
                color: "",
            },
            ManualPart {
                text: "1 KiB,",
                color: colors.accent,
            },
            ManualPart {
                text: "making this a quick way to isolate very small files without supplying a custom threshold.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "For a custom upper size limit, use",
                color: "",
            },
            ManualPart {
                text: "--less-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "instead.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --tiny",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--writable", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show entries with at least one write permission bit set. This is a broad writability filter and includes files writable by the owner, group, or others.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "For the narrower case where the",
                color: "",
            },
            ManualPart {
                text: "world-write bit",
                color: colors.accent,
            },
            ManualPart {
                text: "is set, use",
                color: "",
            },
            ManualPart {
                text: "--world-writable",
                color: colors.emphasis,
            },
            ManualPart {
                text: "instead.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --writable",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--world-writable", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries whose world-write permission bit is set. This is narrower than",
                color: "",
            },
            ManualPart {
                text: "--writable",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and specifically identifies entries that may be modified by users outside the owning user or group.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "World-writable entries can deserve extra attention, especially in shared or system locations where broad write access is unexpected.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l /tmp --world-writable",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--suid", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries with the",
                color: "",
            },
            ManualPart {
                text: "set-user-ID",
                color: colors.accent,
            },
            ManualPart {
                text: "permission bit set. On executable files, this bit can cause the program to run with the effective user identity of the file owner.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "This makes",
                color: "",
            },
            ManualPart {
                text: "--suid",
                color: colors.emphasis,
            },
            ManualPart {
                text: "particularly useful when inspecting system executables and other locations where elevated execution behavior matters.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l /usr/bin --suid",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--sgid", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries with the",
                color: "",
            },
            ManualPart {
                text: "set-group-ID",
                color: colors.accent,
            },
            ManualPart {
                text: "permission bit set. On executable files, this can cause the program to run with the effective group identity associated with the file.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "On directories, the same bit can affect how newly created entries inherit group ownership, so",
                color: "",
            },
            ManualPart {
                text: "--sgid",
                color: colors.emphasis,
            },
            ManualPart {
                text: "can also be useful when examining shared directory structures.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l /usr/bin --sgid",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Time Filtering", inner_width, colors.title, &colors);

    print_paragraph(
        &[ManualPart {
            text: "These options narrow listings by modification age. Some use fixed time windows, while others let you supply a custom age value.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--since AGE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show entries changed within AGE. Use this when you want to look back a chosen amount of time from now and focus on recent activity in a directory.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "This is a flexible recency filter and pairs naturally with",
                color: "",
            },
            ManualPart {
                text: "--long",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or age-based sorting when you want to inspect what has changed most recently.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -l ~/Projects --since 12h",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--newer-than AGE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries newer than AGE. This is another custom recent-entry filter and is useful when you prefer phrasing the condition in terms of",
                color: "",
            },
            ManualPart {
                text: "newer than",
                color: colors.accent,
            },
            ManualPart {
                text: "rather than",
                color: "",
            },
            ManualPart {
                text: "since.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--older-than AGE", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries older than AGE. This is the opposite of",
                color: "",
            },
            ManualPart {
                text: "--since",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "--newer-than,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and is useful when you want to focus on stale or long-untouched material.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Downloads --older-than 6mo",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--fresh", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries modified within the last",
                color: "",
            },
            ManualPart {
                text: "24 hours.",
                color: colors.accent,
            },
            ManualPart {
                text: "This is the fixed-window convenience form of recent time filtering.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--stale", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Show entries modified more than",
                color: "",
            },
            ManualPart {
                text: "30 days ago.",
                color: colors.accent,
            },
            ManualPart {
                text: "This is the fixed-window convenience form of older-entry filtering.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted age syntax such as",
                color: "",
            },
            ManualPart {
                text: "30m, 12h, 7d, 3mo,",
                color: colors.accent,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "1y",
                color: colors.accent,
            },
            ManualPart {
                text: "is described in the next",
                color: "",
            },
            ManualPart {
                text: "Age Format",
                color: colors.accent,
            },
            ManualPart {
                text: "section.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Age Format", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Age values combine a positive number with a time suffix. These forms are used by options such as",
                color: "",
            },
            ManualPart {
                text: "--since, --newer-than,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "--older-than.",
                color: colors.emphasis,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("m", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Minutes. For example:",
                color: "",
            },
            ManualPart {
                text: "30m",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("h", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Hours. For example:",
                color: "",
            },
            ManualPart {
                text: "12h",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("d", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Days. For example:",
                color: "",
            },
            ManualPart {
                text: "7d",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("w", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Weeks. For example:",
                color: "",
            },
            ManualPart {
                text: "4w",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("mo", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Months. For example:",
                color: "",
            },
            ManualPart {
                text: "3mo",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("y", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Years. For example:",
                color: "",
            },
            ManualPart {
                text: "2y",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("File States", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Noct assigns one detected state to each entry when certain filesystem conditions are found. These states can be displayed with",
                color: "",
            },
            ManualPart {
                text: "--state",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or used as filters with",
                color: "",
            },
            ManualPart {
                text: "--only-state.",
                color: colors.emphasis,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[ManualPart {
            text: "An entry receives only one displayed state. When several conditions are true at the same time, Noct reports the first matching state according to its detection order.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("broken-link", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "A symbolic link whose target cannot be resolved.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("empty", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "A regular file whose size is zero bytes.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("no-group", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "An entry whose stored group ID has no corresponding local group.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("no-owner", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "An entry whose stored user ID has no corresponding local user.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("normal", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "No attention-worthy filesystem state was detected for the entry.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("setgid", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "The set-group-ID permission bit is set.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("setuid", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "The set-user-ID permission bit is set.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("world-writable", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "The world-write permission bit is set, allowing users outside the owning user or group to modify the entry.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Sorting", inner_width, colors.title, &colors);

    print_paragraph(
        &[ManualPart {
            text: "Ordinary listings can be ordered by several entry properties. Choose a sort field explicitly, use the size shortcut, reverse the selected order, or disable sorting entirely.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-r, --reverse", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Reverse the selected sort order. This applies after Noct has sorted the entries, letting the same field be viewed from the opposite direction.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Downloads --sort size --reverse",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-s, --sort FIELD", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Sort entries by FIELD. The ordinary listing fields are",
                color: "",
            },
            ManualPart {
                text: "name, size, age,",
                color: colors.accent,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "extension.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh ~/Projects --sort=age",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-S", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Sort by size with the largest entries first. This is the short-form convenience equivalent of selecting the",
                color: "",
            },
            ManualPart {
                text: "size",
                color: colors.accent,
            },
            ManualPart {
                text: "sort field.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lhS ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-U, --unsorted", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Do not sort the listing. Entries remain in the order returned by the filesystem instead of being rearranged by Noct.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "This is useful when you want to inspect",
                color: "",
            },
            ManualPart {
                text: "filesystem directory order",
                color: colors.accent,
            },
            ManualPart {
                text: "rather than one of Noct's named sort orders.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lU ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Sort Fields", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_line("name", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Sort alphabetically by entry name. This is Noct's ordinary default sort.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("size", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Sort by file size with the largest entries first.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("age", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Sort by modification age with the newest entries first.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("date", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Sort by the timestamp selected with -t, --time, with the newest entries first.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("extension, ext", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Sort by filename extension.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line(
        "Long View and Time Fields",
        inner_width,
        colors.title,
        &colors,
    );

    print_paragraph(
        &[
            ManualPart {
                text: "Long view can display one of several filesystem timestamps for each entry. Use",
                color: "",
            },
            ManualPart {
                text: "-t, --time",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to choose which timestamp is shown.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The long-view columns can also be trimmed with options such as",
                color: "",
            },
            ManualPart {
                text: "--no-date, --no-age, --no-user, --no-perms,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "--no-state.",
                color: colors.emphasis,
            },
            ManualPart {
                text: "Their full descriptions appear earlier under Display and Listing.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("-t, --time FIELD", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Select which filesystem timestamp is displayed in long view and used by",
                color: "",
            },
            ManualPart {
                text: "--sort DATE.",
                color: colors.emphasis,
            },
            ManualPart {
                text: "FIELD",
                color: colors.accent,
            },
            ManualPart {
                text: "may be",
                color: "",
            },
            ManualPart {
                text: "modified, accessed,",
                color: colors.accent,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "created.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh -t created -s date ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt -lh -t created ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("modified, mod", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Use the file modification time. This is Noct's default timestamp field.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("accessed, access", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Use the file access time.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("created, create", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Use the file creation time when the filesystem and operating system make that timestamp available.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line(
        "Mode-specific Sort Fields",
        inner_width,
        colors.title,
        &colors,
    );

    print_paragraph(
        &[ManualPart {
            text: "Some Noct inspection modes define their own sort fields in addition to the ordinary listing fields. These fields only apply to the mode named below.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--cold", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "age, path, size.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--empty-dirs", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "age, depth, path.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--exts", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "count, name, size.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--graveyard", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "age, name, score, size.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--hot", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "age, path.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--leaves", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "files, path, size.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--owners", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "entries, owner, size.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--perms", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "attention, count, modes.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--scripts", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Accepted sort fields:",
                color: "",
            },
            ManualPart {
                text: "count, interpreter, name.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Filter Kinds", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "The",
                color: "",
            },
            ManualPart {
                text: "-f, --filter",
                color: colors.emphasis,
            },
            ManualPart {
                text: "option accepts one filter kind followed by a value. The kinds are:",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("extension, ext", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Match regular files by filename extension.",
                color: "",
            },
            ManualPart {
                text: "ext",
                color: colors.emphasis,
            },
            ManualPart {
                text: "is the shorter alias.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("type", inner_width, colors.title, &colors);

    print_paragraph(
        &[ManualPart {
            text: "Match entries by filesystem type. This kind takes a value. Accepted values are:",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("  file", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "        Match only regular file entries.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("  directory (dir)", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "        Match only directory entries.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("  symlink (link)", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "        Match only symbolic link entries.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("  special", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "        Match special entries (sockets, FIFOs, block devices, and character devices)",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Graveyard Kinds", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "The",
                color: "",
            },
            ManualPart {
                text: "--graveyard",
                color: colors.emphasis,
            },
            ManualPart {
                text: "mode classifies cleanup candidates into recognizable kinds. Use",
                color: "",
            },
            ManualPart {
                text: "--kind",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to restrict the report to candidates carrying one of these classifications.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "A candidate may match more than one kind. Noct keeps those matches as tags while selecting one primary kind for the displayed report.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("cache", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Names that look cache-related, including names containing",
                color: "",
            },
            ManualPart {
                text: "cache.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("trash", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Names suggesting trash, recycling, or graveyard-style storage.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("backup", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Backup-like names and suffixes such as",
                color: "",
            },
            ManualPart {
                text: "~, .bak, .backup, .old, .orig,",
                color: colors.accent,
            },
            ManualPart {
                text: "and similar forms.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("temp", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Temporary or incomplete-download names, including patterns such as",
                color: "",
            },
            ManualPart {
                text: "tmp, temp, .swp, .part,",
                color: colors.accent,
            },
            ManualPart {
                text: "and download remnants.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("save", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Save, autosave, recovery, or recovered-file names.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("log", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Log-like filenames, including common",
                color: "",
            },
            ManualPart {
                text: ".log",
                color: colors.accent,
            },
            ManualPart {
                text: "and rotated-log forms.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("archive", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Common archive and compressed-file formats such as",
                color: "",
            },
            ManualPart {
                text: "zip, tar, 7z, rar, gz, xz,",
                color: colors.accent,
            },
            ManualPart {
                text: "and related forms.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("empty", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Entries whose calculated size is zero bytes.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("large", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Entries whose calculated size is at least",
                color: "",
            },
            ManualPart {
                text: "100 MiB.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("old", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Entries whose modification age is at least",
                color: "",
            },
            ManualPart {
                text: "180 days.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    /* =========================================================================
                               Noct Specific Options
    ========================================================================= */

    print_blank_line(inner_width, &colors);

    print_line("Noct-Specific Options", inner_width, colors.title, &colors);

    print_paragraph(
        &[ManualPart {
            text: "Noct's inspection modes are focused tools for answering specific filesystem questions. Instead of shaping an ordinary listing, each mode performs a dedicated analysis of files, directories, ownership, age, permissions, packages, links, structure, or cleanup candidates.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--audit", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Audit a directory for filesystem entries that deserve attention. This gives you a focused \
inspection of unusual or potentially troublesome entries instead of making you hunt for them through an ordinary listing.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use it when you want a quick",
                color: "",
            },
            ManualPart {
                text: "filesystem health check",
                color: colors.accent,
            },
            ManualPart {
                text: "of a directory before investigating individual findings more closely.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --audit /usr/local/bin",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--cold", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Search recursively for old or stale filesystem entries below a directory. By default, --cold \
looks for entries older than 180 days and returns at most 50 results, making it useful for finding material that may have \
been sitting untouched for a long time.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The age threshold can be changed with",
                color: "",
            },
            ManualPart {
                text: "--older-than",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and results can be restricted by",
                color: "",
            },
            ManualPart {
                text: "--min-size",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "--limit.",
                color: colors.emphasis,
            },
            ManualPart {
                text: "Sorting is available by age, size, or path, and the selected order can be reversed.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --cold ~/ --older-than=1y --min-size 100M",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --cold ~/ --sort size --limit=20",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--du", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Walk a directory and report the sizes of the directories below it. This gives you a compact way \
to see where disk space is concentrated without turning the ordinary directory listing into a recursive size report.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when you only want the largest part of the report instead of every directory found.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --du ~/Projects --limit=20",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --du",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--duplicates", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Find regular files that really contain the same data. Noct first narrows candidates by size, \
then hashes them, and finally verifies matching candidates by content before reporting them as duplicates.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Without",
                color: "",
            },
            ManualPart {
                text: "--recursive",
                color: colors.emphasis,
            },
            ManualPart {
                text: "the search stays in the selected directory. Add",
                color: "",
            },
            ManualPart {
                text: "--recursive",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "-R",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to descend into subdirectories, and use",
                color: "",
            },
            ManualPart {
                text: "--min-size",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to ignore files below a chosen size.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --duplicates ~/Downloads --recursive --min-size=10M",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --duplicates",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--empty-dirs", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Find empty directories recursively below a starting directory. This is useful for locating abandoned \
directory structures, leftover application folders, and empty branches that are easy to miss in an ordinary listing.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--min-depth",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to ignore empty directories close to the starting point, and",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to restrict the number of results. Entries can be sorted by age, depth, or path.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --empty-dirs ~/Projects --min-depth=3 --limit 30",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --empty-dirs ~/ --sort depth",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--exts", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show an extension profile for the regular files in a directory. Noct groups files by extension and \
reports how many files belong to each group together with the space represented by that group.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The report is sorted by total size by default. Use",
                color: "",
            },
            ManualPart {
                text: "--sort",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to order extension groups by size, file count, or name instead.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --exts ~/Downloads --sort=count",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --exts ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--graveyard", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Find entries that look like cleanup candidates. Noct recognizes patterns such as caches, trash, \
backups, temporary files, save files, logs, archives, empty entries, unusually large entries, and old material, then \
assigns candidates a score based on the reasons that matched.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--kind",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to concentrate on one candidate type, or refine the report with",
                color: "",
            },
            ManualPart {
                text: "--min-size, --older-than, --limit,",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and",
                color: "",
            },
            ManualPart {
                text: "--recursive.",
                color: colors.emphasis,
            },
            ManualPart {
                text: "Results can be sorted by score, size, age, or name; score is the default.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --graveyard ~/ --kind=backup --older-than 180d",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --graveyard ~/ --recursive --min-size=100M",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--hot", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Search recursively for recently modified entries below a directory. By default, --hot looks back \
24 hours, sorts the newest findings first, and shows at most 50 results.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Change the time window with",
                color: "",
            },
            ManualPart {
                text: "--since",
                color: colors.emphasis,
            },
            ManualPart {
                text: "and the number of displayed findings with",
                color: "",
            },
            ManualPart {
                text: "--limit.",
                color: colors.emphasis,
            },
            ManualPart {
                text: "The report can be sorted by age or path.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --hot ~/Projects --since=6h --limit 30",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --hot ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--inspect", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Inspect one filesystem entry in detail. Give --inspect a path, or supply a command name and noct \
will also look for that command in PATH before examining the resolved entry.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The report brings together",
                color: "",
            },
            ManualPart {
                text: "identity, location, type, permissions, ownership, timestamps, state, and package information",
                color: colors.accent,
            },
            ManualPart {
                text: "and",
                color: colors.text,
            },
            ManualPart {
                text: "package information",
                color: colors.accent,
            },
            ManualPart {
                text: "in one place. Symlinks also show their target, while installed commands can show their owning package \
and package version when noct can determine them.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --inspect rg",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --inspect ./Cargo.toml",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--leaves", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Find leaf directories: directories that contain regular files but no subdirectories. This is useful \
for locating the endpoints of directory trees where the actual file collections live.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--min-files",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to require a minimum number of files,",
                color: "",
            },
            ManualPart {
                text: "--min-size",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to ignore small collections, and",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to restrict the report. Results can be sorted by size, file count, or path.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --leaves ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --leaves ~/Projects --min-files=3 --sort files",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --leaves ~/Projects --min-size 10M --limit=20",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--link-to", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Finds the target to a specified symbolic link. By default, --link-to examines the specified \
symlink and reports the matching target.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --link-to path/to/symlink",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--map", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show a visual size map of a directory. The map turns directory contents into a compact overview \
that makes larger entries easier to spot than they would be in an ordinary name-oriented listing.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "An optional path selects the directory to examine. Use",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when you want to restrict how many entries are included in the map.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --map ~/Downloads --limit=25",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--mounts", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show filesystem boundaries below a directory. This reveals mounted filesystems encountered beneath \
the selected path and helps explain where a directory tree crosses from one filesystem into another.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "By default, noct keeps the report focused on relevant boundaries below the selected path. Add",
                color: "",
            },
            ManualPart {
                text: "--all",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when you want the broader mount view instead.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --mounts /var",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --mounts / --all",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--names", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Analyze filename patterns in a directory. Rather than concentrating on file contents or sizes, \
--names looks at the names themselves and turns recurring naming characteristics into a compact report.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--since",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to restrict the analysis to recently modified entries and",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to reduce the number of reported findings.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --names",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --names ~/Downloads --limit=10",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --names ~/ --since 4h --limit=8",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--owners", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Build a recursive ownership profile for a directory. Noct groups the files it finds by owner so you \
can see who owns the contents, how many entries belong to each owner, and how much space those entries represent.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The report can be sorted by",
                color: "",
            },
            ManualPart {
                text: "owner, entries,",
                color: colors.accent,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "size,",
                color: colors.accent,
            },
            ManualPart {
                text: "depending on whether names, file counts, or disk usage are most useful for the inspection.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --owners /var",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --owners /var --sort=entries",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--owns", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show which installed package owns a filesystem path or command. If a command name is supplied, \
noct resolves it through PATH before asking the system package database about the resulting executable.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "This is useful when you have found an unfamiliar file or executable and want to know",
                color: "",
            },
            ManualPart {
                text: "which installed package put it there.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --owns rg",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --owns /usr/local/bin/rg",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--package-bins", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Show the executable commands installed by a package. Noct queries the system package database, \
filters the package contents down to executable command paths, and presents the commands together with package information.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use it when you know the package name but want to discover",
                color: "",
            },
            ManualPart {
                text: "which commands that package actually provides.",
                color: colors.accent,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --package-bins ripgrep",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--perms", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Build a permission profile for the entries in a directory. Instead of listing every entry \
individually, --perms groups matching permission modes and shows how frequently each mode occurs.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "The report can be sorted by",
                color: "",
            },
            ManualPart {
                text: "mode, count,",
                color: colors.accent,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "attention.",
                color: colors.accent,
            },
            ManualPart {
                text: "The attention view is useful when unusual or security-sensitive permission patterns matter more \
than simple frequency.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --perms",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --perms ~/Projects --sort=count",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --perms /usr/local/bin --sort attention",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--portable", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Find filenames that may be awkward to use across shells, tools, or filesystems. This helps reveal \
names whose characters or structure can make copying, scripting, quoting, or moving files between systems less predictable.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--portable",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when you want a",
                color: "",
            },
            ManualPart {
                text: "portability-oriented filename check",
                color: colors.accent,
            },
            ManualPart {
                text: "rather than a general directory inspection.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --portable",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --portable ~/Documents",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--profile", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Build a broad profile of a directory. Instead of answering one narrow filesystem question, \
--profile gathers several kinds of information into a compact overview of the selected directory and its contents.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--profile",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when you want",
                color: "",
            },
            ManualPart {
                text: "the shape of a directory at a glance",
                color: colors.accent,
            },
            ManualPart {
                text: "before deciding which of noct's more specialized inspection modes to use next.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --profile ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--scripts", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Analyze executable scripts in a directory and group them by interpreter. This makes it easy to see \
which scripting environments are represented and how heavily each one is used.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--sort",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to order the report by",
                color: "",
            },
            ManualPart {
                text: "count, interpreter,",
                color: colors.accent,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "name.",
                color: colors.accent,
            },
            ManualPart {
                text: "The",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "option restricts how many results are shown.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --scripts",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --scripts ~/bin --sort=count",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --scripts ~/bin --sort interpreter --limit=10",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--timeline", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Group directory entries by modification age. Instead of presenting files as one ordinary listing, \
--timeline organizes them into age-based groups so you can quickly see how activity in a directory is distributed over time.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--since",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to restrict the report to entries changed within a chosen period, and",
                color: "",
            },
            ManualPart {
                text: "--limit",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to restrict how many entries are shown.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --timeline",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --timeline ~/Downloads",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --timeline ~/ --since=2h --limit 30",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("--tree", inner_width, colors.option, &colors);

    print_indented_paragraph(
        &[ManualPart {
            text: "Display a directory hierarchy as a Tree. This gives you a structural view of the filesystem below \
a selected path and makes parent-child relationships visible in a way that a flat listing cannot.",
            color: "",
        }],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--level",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "-L",
                color: colors.emphasis,
            },
            ManualPart {
                text: "to limit the displayed depth, and",
                color: "",
            },
            ManualPart {
                text: "--dirs-only",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "-D",
                color: colors.emphasis,
            },
            ManualPart {
                text: "when only the directory structure matters.",
                color: "",
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Hidden entries can be included with",
                color: "",
            },
            ManualPart {
                text: "--all",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or",
                color: "",
            },
            ManualPart {
                text: "--almost-all.",
                color: colors.emphasis,
            },
            ManualPart {
                text: "Use",
                color: "",
            },
            ManualPart {
                text: "--summary",
                color: colors.emphasis,
            },
            ManualPart {
                text: "for a summary after the Tree. Entries may also be ordered with",
                color: "",
            },
            ManualPart {
                text: "--sort",
                color: colors.emphasis,
            },
            ManualPart {
                text: "or left in filesystem order with",
                color: "",
            },
            ManualPart {
                text: "--unsorted.",
                color: colors.emphasis,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --tree ~/Projects",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --tree ~/Projects --level=3",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --tree ~/Projects -D --summary",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_indented_paragraph(
        &[
            ManualPart {
                text: "Example:",
                color: colors.emphasis,
            },
            ManualPart {
                text: "nt --tree ~/Projects --almost-all --sort name",
                color: colors.example,
            },
        ],
        inner_width,
        4,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_bottom_border(inner_width, &colors);
}

/* =======================================================================
                         --theme-help option
======================================================================= */
pub fn print_theme_help(palette: &AnsiPalette) {
    wait_for_manual_terminal_size(palette.enabled);

    let inner_width = manual_inner_width();

    let colors = ManualColors::new(palette.enabled);

    print_top_border(inner_width, &colors);

    print_line("Noct Theme Help", inner_width, colors.title, &colors);

    print_line(
        "quick reference for creating and customizing themes",
        inner_width,
        colors.text,
        &colors,
    );

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_line("Getting Started", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Noct themes live in",
                color: "",
            },
            ManualPart {
                text: "~/.config/noct/themes/",
                color: colors.example,
            },
            ManualPart {
                text: "and are selected in the first entry, above the",
                color: "",
            },
            ManualPart {
                text: "[display]",
                color: colors.emphasis,
            },
            ManualPart {
                text: "section, in",
                color: "",
            },
            ManualPart {
                text: "~/.config/noct/noct.toml",
                color: colors.example,
            },
            ManualPart {
                text: "with a setting such as",
                color: "",
            },
            ManualPart {
                text: "theme = \"ash_melon\".",
                color: colors.example,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "The easiest way to build a custom theme is to copy an existing",
                color: "",
            },
            ManualPart {
                text: "Noct",
                color: colors.option,
            },
            ManualPart {
                text: "theme, rename the file,",
                color: "",
            },
            ManualPart {
                text: "set theme in",
                color: "",
            },
            ManualPart {
                text: "noct.toml",
                color: colors.example,
            },
            ManualPart {
                text: "to the filename you renamed, and edit its colors. Theme colors use",
                color: "",
            },
            ManualPart {
                text: "#RRGGBB",
                color: colors.example,
            },
            ManualPart {
                text: "hexadecimal values.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Filesystem Colors", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "The top-level setting",
                color: "",
            },
            ManualPart {
                text: "filesystem_colors = \"theme\"",
                color: colors.example,
            },
            ManualPart {
                text: "uses filesystem classification colors supplied by the selected theme.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Using",
                color: "",
            },
            ManualPart {
                text: "filesystem_colors = \"standard\"",
                color: colors.example,
            },
            ManualPart {
                text: "always uses Noct's built-in filesystem classification palette instead.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "filesystem_colors = \"directories\"",
                color: colors.example,
            },
            ManualPart {
                text: "keeps ordinary filenames unclassified while retaining directory coloring in \
                listings. Tree output continues to use filesystem classification colors.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("[classification]", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "The",
                color: "",
            },
            ManualPart {
                text: "[classification]",
                color: colors.example,
            },
            ManualPart {
                text: "section controls the colors used for classified filesystem entries such as source code, \
documents, archives, images, executables, directories, symlinks, and other file families.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Every classification entry is optional. If an entry is omitted, Noct uses its built-in \
classification color as the fallback value",
                color: "",
            },
            ManualPart {
                text: "throughout all themes.",
                color: colors.accent,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[ManualPart {
            text: "Because those fallback colors are theme-independent, filename and directory colors may not \
harmonize with the rest of a custom theme when filesystem classification entries are omitted.",
            color: colors.accent,
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "When working from an existing theme, consider",
                color: "",
            },
            ManualPart {
                text: "commenting out",
                color: colors.accent,
            },
            ManualPart {
                text: "classification entries rather than deleting them. This keeps every available field visible \
while allowing Noct's built-in fallback color to apply for the omitted entries.",
                color: "",
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Icons and Filenames", inner_width, colors.title, &colors);

    print_paragraph(
        &[ManualPart {
            text: "File icons may use more diversity than the color palette, but an icon \
always inherits the same classification color as its filename.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_line("Useful Commands", inner_width, colors.title, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Generate a commented configuration template with",
                color: "",
            },
            ManualPart {
                text: "nt --generate-config",
                color: colors.example,
            },
            ManualPart {
                text: "and read the full explanatory reference with",
                color: "",
            },
            ManualPart {
                text: "nt --manual.",
                color: colors.example,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_bottom_border(inner_width, &colors);
}

/* =======================================================================
                              --help option
======================================================================= */
pub fn print_help(palette: &AnsiPalette) {
    wait_for_manual_terminal_size(palette.enabled);

    let inner_width = manual_inner_width();

    let colors = ManualColors::new(palette.enabled);

    print_top_border(inner_width, &colors);

    print_help_line("The nt --help text", inner_width, colors.title, &colors);

    print_help_line(
        "noctilucent filesystem inspection tool",
        inner_width,
        colors.text,
        &colors,
    );

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("USAGE:", inner_width, colors.title, &colors);

    print_help_line(
        "  nt [options] [path]",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  noct [options] [path]",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("META OPTIONS:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "  --manual                    Show the explanatory Noct manual",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --theme-help                Show the Noct theming quick reference",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --generate-config           Generate an editable configuration template",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -?, --help                  Show this help text",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -v, --version               Show version information",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("DISPLAY OPTIONS:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "  -a, --all                   Show hidden dotfiles plus . and ..",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -A, --almost-all            Show hidden dotfiles, but not . and ..",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -h, --header                Show column header",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -l, --long                  Show long table view",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -1, --oneline               Show one entry per line",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -d, --directory             List directories themselves, not their contents",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -T, --tree                  Show directory contents as a recursive Tree",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -L, --level DEPTH           Limit Tree recursion to DEPTH directory levels",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --no-colors                 Disable colored output",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --icons                     Show file-type icons",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --no-age                    Hide the AGE column in long view",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --no-date                   Hide the timestamp column in long view",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --no-user                   Hide the USER column in long view",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --no-perms                  Hide the MODE/permissions column in long view",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --no-state                  Hide the STATE column in long view",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --state                     Show detected file state in grid/oneline views",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --summary                   Show a summary line at the bottom",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --limit N                   Limit output to N entries where supported",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --tail N                    Show the last N entries after filtering and sorting",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -w, --wide                  Use more generous grid columns",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -W, --width COLS            Format grid output as COLS columns wide",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -s, --sort FIELD            Sort entries by field",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -S                          Sort by size, largest first",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -r, --reverse               Reverse the selected sort order",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -U, --unsorted              Do not sort; keep filesystem directory order",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -f, --filter KIND VALUE     Filter entries by kind and value",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -t, --time FIELD            Select timestamp field",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("NOCT-SPECIFIC OPTIONS:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[ManualPart {
            text: "These options produce focused filesystem views or analyses and may be used directly as Noct operations. Many accept additional modifiers, fields, or kinds listed in the reference sections below.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line(
        "  --audit                     Audit directory for attention-worthy filesystem entries",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --big                       Show large regular files, default min 10 MiB",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --cold                      Show old/stale entries recursively",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --danger                    Show only attention-worthy entries",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --du                        Walk dirs and display their size",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --duplicates PATH           Find duplicate regular entries in PATH",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --empty                     Show empty regular files",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --empty-dirs                Find empty directories recursively",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --executables               Show only executable files",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --exts                      Show extension profile for regular files",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --fresh                     Sort/show entries changed within the last 24 hours",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --graveyard                 Show cleanup candidates",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --hot                       Show recently modified entries recursively",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --inspect PATH              Show detailed information about a file or directory",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --leaves                    Show directories containing files but no subdirectories",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --link-to LINK              Show the target for the symlink LINK",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --map                       Show a visual size map of a directory",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --mounts                    Show filesystem boundaries below a directory",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --names                     Analyze filename patterns",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --only-state STATE          Only normal, empty, broken-link, setuid, setgid,",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "                              world-writable (See STATES)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --orphans                   Show broken links, missing owners, and missing groups",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --owners                    Show recursive owner profile for a directory",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --owns PATH|COMMAND         Show which package owns a path or command",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --package-bins PACKAGE      Show executable commands installed by a package",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --permissions MODE          Match exact permission bits, for example 755",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --perms                     Show permission profile for directory entries",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --portable                  Find filenames that may be awkward across shells/filesystems",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --profile                   Show rich directory profile",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --scripts                   Show executable scripts grouped by interpreter",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --sgid                      Show setgid entries",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --suid                      Show setuid entries",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --stale                     Show entries older than 30 days",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --tiny                      Show tiny regular files, default 1 KiB",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --writable                  Show only writable files",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --world-writable            Show world-writable entries",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --zero                      Show zero-byte files (same as --empty)",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("MODIFIER OPTIONS:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);


    print_help_line(
        "  --greater-than SIZE         Show regular files at least SIZE (See SIZE)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --less-than SIZE            Show regular files at most SIZE (See SIZE)",
        inner_width,
        colors.example,
        &colors,
    );
    print_help_line(
        "  --newer-than AGE            Show entries changed within AGE (See AGE)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --older-than AGE            Show entries older than AGE (See AGE)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --since AGE                 Show entries changed within AGE (See AGE)",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "SPECIAL MODIFIER OPTIONS:",
        inner_width,
        colors.title,
        &colors,
    );

    print_blank_line(inner_width, &colors);


    print_help_line(
        "  -D, --dirs-only             Used with --tree",
        inner_width,
        colors.example,
        &colors,
    );

    print_line(
        "  --kind KIND                 Used with --graveyard (See KINDS)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --min-depth N               Used with --empty-dirs",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --min-files N               Used with --leaves",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  --min-size SIZE             Used with --duplicates, --graveyard, --leaves,",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "                              and --cold (See SIZE)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  -R, --recursive             Used with --duplicates and --graveyard",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("FIELDS:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("  SORT:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "    name                      Sort by name",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    size                      Sort by size, largest first (also works with --owners)",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    age                       Sort by age, newest first",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    date                      Selected timestamp, newest first",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    extension, ext            Sort by extension",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line("  TIME:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "    accessed, access          Use access time",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    created, create           Use creation time",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    modified, mod             Use modification time",
        inner_width,
        colors.example,
        &colors,
    );


    print_blank_line(inner_width, &colors);

    print_help_line("  NOCT SPECIFIC SORT:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "    --cold:                   age, path, size",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --empty-dirs:             age, depth, path",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --exts                    count, name, size",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --graveyard:              age, name, score, size",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --hot:                    age, path",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --leaves:                 files, path, size",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --owners:                 entries, owner, size",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --perms:                  attention, count, modes",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "    --scripts:                count, interpreter, name",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("KINDS:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[ManualPart {
            text: "Kinds are named values accepted by options that classify or select a specific type of result.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line("-f, --filter:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "  extension, ext              Match files by extension",
        inner_width,
        colors.example,
        &colors,
    );

    print_line(
        "  type                        Match entries by filesystem type",
        inner_width,
        colors.example,
        &colors,
    );

    print_line(
        "    file",
        inner_width,
        colors.example,
        &colors,
    );

    print_line(
        "    directory, dir",
        inner_width,
        colors.example,
        &colors,
    );

    print_line(
        "    symlink, link",
        inner_width,
        colors.example,
        &colors,
    );

    print_line(
        "    special",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line("--graveyard:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("  archive", inner_width, colors.example, &colors);

    print_help_line("  backup", inner_width, colors.example, &colors);

    print_help_line("  cache", inner_width, colors.example, &colors);

    print_help_line("  empty", inner_width, colors.example, &colors);

    print_help_line("  large", inner_width, colors.example, &colors);

    print_help_line("  log", inner_width, colors.example, &colors);

    print_help_line("  old", inner_width, colors.example, &colors);

    print_help_line("  save", inner_width, colors.example, &colors);

    print_help_line("  temp", inner_width, colors.example, &colors);

    print_help_line("  trash", inner_width, colors.example, &colors);

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("STATES:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("  DEFAULT", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);


    print_help_line("    broken-link", inner_width, colors.example, &colors);

    print_help_line("    empty", inner_width, colors.example, &colors);

    print_help_line("    no-group", inner_width, colors.example, &colors);

    print_help_line("    no-owner", inner_width, colors.example, &colors);

    print_help_line("    normal", inner_width, colors.example, &colors);

    print_help_line("    setgid", inner_width, colors.example, &colors);

    print_help_line("    setuid", inner_width, colors.example, &colors);

    print_help_line("    world-writable", inner_width, colors.example, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("  --only-state:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("    broken-link", inner_width, colors.example, &colors);

    print_help_line("    empty", inner_width, colors.example, &colors);

    print_help_line("    normal", inner_width, colors.example, &colors);

    print_help_line("    setgid", inner_width, colors.example, &colors);

    print_help_line("    setuid", inner_width, colors.example, &colors);

    print_help_line("    world-writable", inner_width, colors.example, &colors);

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line("AGE FORMAT:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_help_line(
        "  1m, 2h, 3d, 4w, 5mo, 6y",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line("SIZE FORMAT:", inner_width, colors.title, &colors);

    print_blank_line(inner_width, &colors);

    print_paragraph(
        &[ManualPart {
            text: "All units may be written in lower, upper, or mixed case; size input is case-insensitive.",
            color: "",
        }],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line(
        "  B                      Bytes",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  K, KB, KiB             Kibibytes",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  M, MB, MiB             Mebibytes",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  G, GB, GiB             Gibibytes",
        inner_width,
        colors.example,
        &colors,
    );

    print_help_line(
        "  T, TB, TiB             Tebibytes",
        inner_width,
        colors.example,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_separator(inner_width, &colors);

    print_paragraph(
        &[
            ManualPart {
                text: "Send",
                color: colors.accent,
            },
            ManualPart {
                text: "noct",
                color: colors.title,
            },
            ManualPart {
                text: "into the filesystem and see what comes crawling back.",
                color: colors.accent,
            },
        ],
        inner_width,
        &colors,
    );

    print_blank_line(inner_width, &colors);

    print_help_line(
        "You can learn the sharper tools later. Preferably after coffee. :)",
        inner_width,
        colors.accent,
        &colors,
    );

    print_bottom_border(inner_width, &colors);
}

fn wait_for_manual_terminal_size(colors_enabled: bool) {
    if !io::stdout().is_terminal() {
        return;
    }

    let Some((mut width, mut height)) = terminal_size() else {
        return;
    };

    if width >= MANUAL_MIN_WIDTH && height >= MANUAL_MIN_HEIGHT {
        return;
    }

    MANUAL_RESIZE_INTERRUPTED.store(false, Ordering::SeqCst);

    let previous_sigint_handler = install_manual_resize_sigint_handler();

    enter_alternate_screen();

    loop {
        if MANUAL_RESIZE_INTERRUPTED.load(Ordering::SeqCst) {
            leave_alternate_screen();

            restore_manual_resize_sigint_handler(previous_sigint_handler);

            println!();

            std::process::exit(130);
        }

        draw_manual_resize_screen(width, height, colors_enabled);

        thread::sleep(Duration::from_millis(75));

        if MANUAL_RESIZE_INTERRUPTED.load(Ordering::SeqCst) {
            leave_alternate_screen();

            restore_manual_resize_sigint_handler(previous_sigint_handler);

            println!();

            std::process::exit(130);
        }

        let Some((new_width, new_height)) = terminal_size() else {
            leave_alternate_screen();

            restore_manual_resize_sigint_handler(previous_sigint_handler);

            return;
        };

        width = new_width;
        height = new_height;

        if width >= MANUAL_MIN_WIDTH && height >= MANUAL_MIN_HEIGHT {
            leave_alternate_screen();

            restore_manual_resize_sigint_handler(previous_sigint_handler);

            break;
        }
    }
}

fn draw_manual_resize_screen(width: usize, height: usize, colors_enabled: bool) {
    clear_terminal_screen();

    let logo_width = MANUAL_LOGO
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    let content_height = 13usize;

    let top_padding = height.saturating_sub(content_height) / 2;

    for _ in 0..top_padding {
        println!();
    }

    if width >= logo_width {
        let left_padding = width.saturating_sub(logo_width) / 2;

        for (index, line) in MANUAL_LOGO.iter().enumerate() {
            print!("{}", " ".repeat(left_padding));

            if colors_enabled {
                println!("{}{}{}", MANUAL_LOGO_COLORS[index], line, "\x1b[0m",);
            } else {
                println!("{}", line);
            }
        }
    } else {
        print_centered_resize_line("NOCT", width, MANUAL_RESIZE_WHITE, colors_enabled);
    }

    println!();

    print_centered_resize_line(
        "Noct Manual requires a larger terminal.",
        width,
        MANUAL_RESIZE_GREEN,
        colors_enabled,
    );

    println!();

    print_manual_dimension_line(
        "Minimum size:",
        MANUAL_MIN_WIDTH,
        MANUAL_MIN_HEIGHT,
        width,
        MANUAL_RESIZE_WHITE,
        colors_enabled,
    );

    let current_color = if width < MANUAL_MIN_WIDTH || height < MANUAL_MIN_HEIGHT {
        MANUAL_RESIZE_RED
    } else {
        MANUAL_RESIZE_WHITE
    };

    print_manual_dimension_line(
        "Current size:",
        width,
        height,
        width,
        current_color,
        colors_enabled,
    );

    println!();

    print_centered_resize_line(
        "Please enlarge the terminal window.",
        width,
        MANUAL_RESIZE_MUTED,
        colors_enabled,
    );

    let _ = io::stdout().flush();
}

fn clear_terminal_screen() {
    print!("\x1b[2J\x1b[H");

    let _ = io::stdout().flush();
}

fn enter_alternate_screen() {
    print!("\x1b[?1049h\x1b[2J\x1b[H");

    let _ = io::stdout().flush();
}

fn leave_alternate_screen() {
    print!("\x1b[?1049l");

    let _ = io::stdout().flush();
}

fn install_manual_resize_sigint_handler() -> libc::sigaction {
    unsafe {
        let mut new_action: libc::sigaction = std::mem::zeroed();
        let mut old_action: libc::sigaction = std::mem::zeroed();

        let handler: extern "C" fn(libc::c_int) = manual_resize_sigint_handler;

        new_action.sa_sigaction = handler as usize;

        libc::sigemptyset(&mut new_action.sa_mask);

        new_action.sa_flags = 0;

        libc::sigaction(libc::SIGINT, &new_action, &mut old_action);

        old_action
    }
}

fn restore_manual_resize_sigint_handler(previous: libc::sigaction) {
    unsafe {
        libc::sigaction(libc::SIGINT, &previous, std::ptr::null_mut());
    }
}

fn print_centered_resize_line(
    text: &str,
    terminal_width: usize,
    color: &str,
    colors_enabled: bool,
) {
    let left_padding = terminal_width.saturating_sub(text.chars().count()) / 2;

    print!("{}", " ".repeat(left_padding));

    if colors_enabled {
        println!("{}{}{}", color, text, "\x1b[0m",);
    } else {
        println!("{}", text);
    }
}

fn print_manual_dimension_line(
    label: &str,
    value_width: usize,
    value_height: usize,
    terminal_width: usize,
    value_color: &str,
    colors_enabled: bool,
) {
    let value = format!("{} × {}", value_width, value_height,);

    let plain_line = format!("{} {}", label, value,);

    let left_padding = terminal_width.saturating_sub(plain_line.chars().count()) / 2;

    print!("{}", " ".repeat(left_padding));

    if colors_enabled {
        println!(
            "{}{}{} {}{}{}",
            MANUAL_RESIZE_MUTED, label, "\x1b[0m", value_color, value, "\x1b[0m",
        );
    } else {
        println!("{} {}", label, value);
    }
}

fn manual_inner_width() -> usize {
    let available = terminal_width().saturating_sub(8);

    available.clamp(MIN_INNER_WIDTH, MAX_INNER_WIDTH)
}

fn print_top_border(width: usize, colors: &ManualColors) {
    println!(
        "{}┌{}┐{}",
        colors.border,
        "─".repeat(width + 4),
        colors.reset,
    );
}

fn print_separator(width: usize, colors: &ManualColors) {
    println!(
        "{}├{}┤{}",
        colors.border,
        "─".repeat(width + 4),
        colors.reset,
    );
}

fn print_bottom_border(width: usize, colors: &ManualColors) {
    println!(
        "{}└{}┘{}",
        colors.border,
        "─".repeat(width + 4),
        colors.reset,
    );
}

fn print_blank_line(width: usize, colors: &ManualColors) {
    println!(
        "{}│{}  {}  {}│{}",
        colors.border,
        colors.reset,
        " ".repeat(width),
        colors.border,
        colors.reset,
    );
}

fn print_line(text: &str, width: usize, color: &str, colors: &ManualColors) {
    let visible_width = text.chars().count();

    let padding = width.saturating_sub(visible_width);

    println!(
        "{}│{}  {}{}{}{}  {}│{}",
        colors.border,
        colors.reset,
        color,
        text,
        colors.reset,
        " ".repeat(padding),
        colors.border,
        colors.reset,
    );
}

fn print_help_line(
    text: &str,
    width: usize,
    color: &str,
    colors: &ManualColors,
) {
    let visible_width = text.chars().count();

    /*
     * Most help lines already fit and should remain completely untouched.
     */
    if visible_width <= width {
        print_line(text, width, color, colors);

        return;
    }

    let characters: Vec<char> = text.chars().collect();

    /*
     * Find the gap between the option/field name and its description.
     *
     * Help entries are already formatted like:
     *
     *   --option VALUE             Description...
     *
     * The continuation line begins at the same column as Description.
     */
    let mut description_start = None;

    let mut index = 0usize;

    let mut seen_non_space = false;

    while index < characters.len() {
        if characters[index] != ' ' {
            seen_non_space = true;

            index += 1;

            continue;
        }

        let gap_start = index;

        while index < characters.len() && characters[index] == ' ' {
            index += 1;
        }

        let gap_width = index - gap_start;

        if seen_non_space && gap_width >= 2 && index < characters.len() {
            description_start = Some(index);

            break;
        }
    }

    /*
     * A long line without a table-style description gap is not an option
     * entry. Fall back to the ordinary renderer rather than guessing at its
     * structure.
     */
    let Some(description_start) = description_start else {
        print_line(text, width, color, colors);

        return;
    };

    let prefix: String = characters[..description_start].iter().collect();

    let description: String = characters[description_start..].iter().collect();

    let continuation_prefix = " ".repeat(description_start);

    let description_width = width.saturating_sub(description_start);

    if description_width == 0 {
        print_line(text, width, color, colors);

        return;
    }

    let mut current_line = String::new();

    let mut first_line = true;

    for word in description.split_whitespace() {
        let word_width = word.chars().count();

        let separator_width = usize::from(!current_line.is_empty());

        if !current_line.is_empty()
            && current_line.chars().count() + separator_width + word_width > description_width
        {
            let line = if first_line {
                format!("{}{}", prefix, current_line)
            } else {
                format!("{}{}", continuation_prefix, current_line)
            };

            print_line(&line, width, color, colors);

            current_line.clear();

            first_line = false;
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }

        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        let line = if first_line {
            format!("{}{}", prefix, current_line)
        } else {
            format!("{}{}", continuation_prefix, current_line)
        };

        print_line(&line, width, color, colors);
    }
}

fn print_paragraph(parts: &[ManualPart<'_>], width: usize, colors: &ManualColors) {
    let mut current_width = 0usize;

    print_left_border(colors);

    for part in parts {
        for word in part.text.split_whitespace() {
            let word_width = word.chars().count();

            let separator_width = usize::from(current_width > 0);

            if current_width + separator_width + word_width > width {
                print_right_padding(width, current_width, colors);

                current_width = 0;

                print_left_border(colors);
            }

            if current_width > 0 {
                print!(" ");

                current_width += 1;
            }

            if part.color.is_empty() {
                print!("{}{}{}", colors.text, word, colors.reset,);
            } else {
                print!("{}{}{}", part.color, word, colors.reset);
            }

            current_width += word_width;
        }
    }

    print_right_padding(width, current_width, colors);
}

fn print_indented_paragraph(
    parts: &[ManualPart<'_>],
    width: usize,
    indent: usize,
    colors: &ManualColors,
) {
    let content_width = width.saturating_sub(indent);

    let mut current_width = 0usize;

    print_left_border_with_indent(indent, colors);

    for part in parts {
        for word in part.text.split_whitespace() {
            let word_width = word.chars().count();

            let separator_width = usize::from(current_width > 0);

            if current_width + separator_width + word_width > content_width {
                print_right_padding(content_width, current_width, colors);

                current_width = 0;

                print_left_border_with_indent(indent, colors);
            }

            if current_width > 0 {
                print!(" ");

                current_width += 1;
            }

            if part.color.is_empty() {
                print!("{}{}{}", colors.text, word, colors.reset,);
            } else {
                print!("{}{}{}", part.color, word, colors.reset);
            }

            current_width += word_width;
        }
    }

    print_right_padding(content_width, current_width, colors);
}

fn print_left_border_with_indent(indent: usize, colors: &ManualColors) {
    print!("{}│{}  {}", colors.border, colors.reset, " ".repeat(indent),);
}

fn print_left_border(colors: &ManualColors) {
    print!("{}│{}  ", colors.border, colors.reset,);
}

fn print_right_padding(width: usize, current_width: usize, colors: &ManualColors) {
    let padding = width.saturating_sub(current_width);

    println!(
        "{}{}│{}",
        " ".repeat(padding + 2),
        colors.border,
        colors.reset,
    );
}
