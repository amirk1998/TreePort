use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const FSI: char = '\u{2068}';
pub const PDI: char = '\u{2069}';

pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star = None;
    let mut match_from = 0usize;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            match_from = ti;
            pi += 1;
        } else if let Some(si) = star {
            pi = si + 1;
            match_from += 1;
            ti = match_from;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let upper = s.to_uppercase();
    let (num, multiplier) = if let Some(n) = upper.strip_suffix("GB") {
        (n, 1024.0_f64.powi(3))
    } else if let Some(n) = upper.strip_suffix("MB") {
        (n, 1024.0_f64.powi(2))
    } else if let Some(n) = upper.strip_suffix("KB") {
        (n, 1024.0)
    } else if let Some(n) = upper.strip_suffix('B') {
        (n, 1.0)
    } else {
        (upper.as_str(), 1.0)
    };
    let value: f64 = num
        .trim()
        .parse()
        .map_err(|_| format!("invalid size: {s} (try e.g. 10MB, 500KB, 2048)"))?;
    if value < 0.0 {
        return Err(format!("size cannot be negative: {s}"));
    }
    let bytes = value * multiplier;
    if !bytes.is_finite() || bytes > u64::MAX as f64 {
        return Err(format!("size is too large: {s}"));
    }
    Ok(bytes.round() as u64)
}

pub fn parse_date(s: &str) -> Result<SystemTime, String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("invalid date: {s} (expected YYYY-MM-DD)"));
    }
    let y: i64 = parts[0]
        .parse()
        .map_err(|_| format!("invalid date: {s} (expected YYYY-MM-DD)"))?;
    let m: u32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid date: {s} (expected YYYY-MM-DD)"))?;
    let d: u32 = parts[2]
        .parse()
        .map_err(|_| format!("invalid date: {s} (expected YYYY-MM-DD)"))?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(format!("invalid date: {s} (expected YYYY-MM-DD)"));
    }
    let days = days_from_civil(y, m, d);
    if days < 0 {
        return Err(format!("date before the Unix epoch is not supported: {s}"));
    }
    Ok(UNIX_EPOCH + Duration::from_secs(days as u64 * 86_400))
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

pub fn format_time(t: Option<SystemTime>) -> String {
    match t {
        None => "unknown".to_string(),
        Some(t) => t
            .duration_since(UNIX_EPOCH)
            .map(format_unix_timestamp)
            .unwrap_or_else(|_| "unknown".to_string()),
    }
}

fn format_unix_timestamp(d: Duration) -> String {
    let secs = d.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    let (y, mo, da) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{da:02} {h:02}:{m:02}:{s:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "(no extension)".to_string())
}

pub fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn contains_rtl(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x0590..=0x08FF).contains(&cp)
            || (0xFB1D..=0xFDFF).contains(&cp)
            || (0xFE70..=0xFEFF).contains(&cp)
    })
}

pub fn bidi_safe(s: &str) -> String {
    if contains_rtl(s) {
        format!("{FSI}{s}{PDI}")
    } else {
        s.to_string()
    }
}

#[cfg(unix)]
pub fn format_mode(mode: Option<u32>) -> String {
    let Some(mode) = mode else {
        return "?????????".to_string();
    };
    let bits = [
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
    bits.iter()
        .map(|(mask, ch)| if mode & mask != 0 { *ch } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_supports_star_and_question_mark() {
        assert!(glob_match("*.log", "server.log"));
        assert!(glob_match("tmp_?", "tmp_1"));
        assert!(!glob_match("*.log", "server.txt"));
    }

    #[test]
    fn size_parser_is_binary() {
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1.5MB").unwrap(), 1_572_864);
    }
}
