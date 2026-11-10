//! Small formatting helpers shared by the UI.

/// Format a byte count with binary units, e.g. `8.86 GiB`.
pub fn fmt_bytes(bytes: u64) -> String {
    fmt_scaled(bytes as f64, &["B", "KiB", "MiB", "GiB", "TiB", "PiB"])
}

/// Format a byte count compactly, e.g. `547M`, for dense tables.
pub fn fmt_bytes_short(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    if bytes == 0 {
        return "0B".to_string();
    }
    let (value, unit) = scale(bytes as f64, &UNITS);
    format!("{value}{unit}")
}

/// Format a transfer rate, e.g. `1.64 MiB/s`.
pub fn fmt_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1024.0 {
        format!("{bytes_per_sec:.0} B/s")
    } else {
        format!("{}/s", fmt_bytes(bytes_per_sec as u64))
    }
}

/// Human readable uptime, e.g. `5d 03:12`.
pub fn fmt_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}")
    } else {
        format!("{hours:02}:{minutes:02}")
    }
}

/// Human readable process run time, e.g. `3h 12m`.
pub fn fmt_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {:02}s", seconds % 60)
    } else {
        format!("{}s", seconds % 60)
    }
}

/// Truncate a string to `max` characters, adding an ellipsis when needed.
pub fn truncate(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }

    if text.chars().count() <= max {
        return text.to_string();
    }

    if max == 1 {
        return "…".to_string();
    }

    let mut out: String = text.chars().take(max - 1).collect();
    out.push('…');
    out
}

fn fmt_scaled(bytes: f64, units: &[&'static str]) -> String {
    let (number, unit) = scale(bytes, units);
    format!("{number} {unit}")
}

fn scale(mut value: f64, units: &[&'static str]) -> (String, &'static str) {
    let mut index = 0;
    while value >= 1024.0 && index + 1 < units.len() {
        value /= 1024.0;
        index += 1;
    }

    let number = if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };

    (number, units[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_scaled() {
        assert_eq!(fmt_bytes(0), "0.00 B");
        assert_eq!(fmt_bytes(1024), "1.00 KiB");
        assert_eq!(fmt_bytes(1536), "1.50 KiB");
        assert_eq!(fmt_bytes_short(547 * 1024 * 1024), "547M");
    }

    #[test]
    fn uptime_is_readable() {
        assert_eq!(fmt_uptime(90), "00:01");
        assert_eq!(fmt_uptime(90_000), "1d 01:00");
    }

    #[test]
    fn strings_are_truncated() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
    }
}
