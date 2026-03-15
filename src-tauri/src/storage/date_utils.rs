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

pub fn calculate_days_since(date_str: &str) -> Result<u64, String> {
    let (year, month, day) = parse_date(date_str)?;
    let date_days = days_from_civil(year, month, day) - days_from_civil(1970, 1, 1);
    let today_days = today_epoch_days()?;
    let diff = today_days - date_days;
    Ok(if diff > 0 { diff as u64 } else { 0 })
}
