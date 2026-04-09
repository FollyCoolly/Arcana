pub fn parse_date(date_str: &str) -> Result<(i32, u32, u32), String> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Invalid date '{}'. Expected format YYYY-MM-DD",
            date_str
        ));
    }

    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| format!("Invalid year in date '{}'", date_str))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid month in date '{}'", date_str))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid day in date '{}'", date_str))?;

    if !(1..=12).contains(&month) {
        return Err(format!("Invalid month '{}' in date '{}'", month, date_str));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("Invalid day '{}' in date '{}'", day, date_str));
    }

    Ok((year, month, day))
}

pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe) as i64
}

pub fn today_epoch_days() -> Result<i64, String> {
    let now_duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System clock before UNIX_EPOCH: {}", e))?;
    Ok((now_duration.as_secs() / 86_400) as i64)
}

pub fn epoch_days_to_civil(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

pub fn current_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;
    let (y, m, d) = epoch_days_to_civil(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

pub fn calculate_days_since(date_str: &str) -> Result<u64, String> {
    let (year, month, day) = parse_date(date_str)?;
    let date_days = days_from_civil(year, month, day) - days_from_civil(1970, 1, 1);
    let today_days = today_epoch_days()?;
    let diff = today_days - date_days;
    Ok(if diff > 0 { diff as u64 } else { 0 })
}
