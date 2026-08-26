use std::env;
use std::fs;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Datelike, Local};

pub fn uid_exists(uid: u32) -> bool {
    let Ok(passwd) = fs::read_to_string("/etc/passwd") else {
        return true;
    };

    for line in passwd.lines() {
        let parts: Vec<&str> = line.split(':').collect();

        if parts.len() < 3 {
            continue;
        }

        let Ok(entry_uid) = parts[2].parse::<u32>() else {
            continue;
        };

        if entry_uid == uid {
            return true;
        }
    }

    false
}

pub fn gid_exists(gid: u32) -> bool {
    let Ok(group_file) = fs::read_to_string("/etc/group") else {
        return true;
    };

    for line in group_file.lines() {
        let parts: Vec<&str> = line.split(':').collect();

        if parts.len() < 3 {
            continue;
        }

        let Ok(entry_gid) = parts[2].parse::<u32>() else {
            continue;
        };

        if entry_gid == gid {
            return true;
        }
    }

    false
}

pub fn group_name(gid: u32) -> String {
    let Ok(group_file) = fs::read_to_string("/etc/group") else {
        return gid.to_string();
    };

    for line in group_file.lines() {
        let parts: Vec<&str> = line.split(':').collect();

        if parts.len() < 3 {
            continue;
        }

        let Ok(entry_gid) = parts[2].parse::<u32>() else {
            continue;
        };

        if entry_gid == gid {
            return parts[0].to_string();
        }
    }

    gid.to_string()
}

pub fn user_name(uid: u32) -> String {
    let Ok(passwd) = fs::read_to_string("/etc/passwd") else {
        return uid.to_string();
    };

    for line in passwd.lines() {
        let parts: Vec<&str> = line.split(':').collect();

        if parts.len() < 3 {
            continue;
        }

        let Ok(entry_uid) = parts[2].parse::<u32>() else {
            continue;
        };

        if entry_uid == uid {
            return parts[0].to_string();
        }
    }

    uid.to_string()
}

pub fn human_date(time: Option<SystemTime>) -> String {
    let Some(time) = time else {
        return "?".to_string();
    };

    let datetime: DateTime<Local> = time.into();

    let now = Local::now();

    if datetime.year() == now.year() {
        datetime.format("%-d %b %H:%M").to_string()
    } else {
        datetime.format("%-d %b %Y").to_string()
    }
}

pub fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub fn octal_mode(mode: u32) -> String {
    format!("{:04o}", mode & 0o7777)
}

pub fn extension_label(path: &std::path::Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_else(|| "none".to_string())
}

pub fn terminal_width() -> usize {
    terminal_width_from_ioctl()
        .or_else(terminal_width_from_environment)
        .unwrap_or(80)
}

pub fn terminal_size() -> Option<(usize, usize)> {
    let mut window_size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut window_size) };

    if result != 0 {
        return None;
    }

    let width = window_size.ws_col as usize;
    let height = window_size.ws_row as usize;

    if width == 0 || height == 0 {
        None
    } else {
        Some((width, height))
    }
}

fn terminal_width_from_environment() -> Option<usize> {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width > 0)
}

fn terminal_width_from_ioctl() -> Option<usize> {
    let mut window_size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut window_size) };

    if result != 0 {
        return None;
    }

    let width = window_size.ws_col as usize;

    if width == 0 { None } else { Some(width) }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["K", "M", "G", "T"];

    if bytes < 1024 {
        return bytes.to_string();
    }

    let mut size = bytes as f64 / 1024.0;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1}{}", size, UNITS[unit_index])
}

pub fn human_age(modified: Option<SystemTime>) -> String {
    let Some(modified_time) = modified else {
        return "?".to_string();
    };

    let age = SystemTime::now()
        .duration_since(modified_time)
        .unwrap_or(Duration::from_secs(0));

    let seconds = age.as_secs();

    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    const MONTH: u64 = 30 * DAY;
    const YEAR: u64 = 365 * DAY;

    if seconds < MINUTE {
        format!("{}s", seconds)
    } else if seconds < HOUR {
        format!("{}m", seconds / MINUTE)
    } else if seconds < DAY {
        format!("{}h", seconds / HOUR)
    } else if seconds < WEEK {
        format!("{}d", seconds / DAY)
    } else if seconds < MONTH {
        format!("{}w", seconds / WEEK)
    } else if seconds < YEAR {
        format!("{}mo", seconds / MONTH)
    } else {
        format!("{}y", seconds / YEAR)
    }
}
