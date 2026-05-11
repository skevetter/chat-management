use chrono::Utc;
use regex::Regex;

pub fn parse_relative_duration(s: &str) -> Option<String> {
    let re = Regex::new(r"^(\d+)(s|m|h|d)$").unwrap();
    let caps = re.captures(s)?;
    let amount: i64 = caps[1].parse().ok()?;
    let seconds = match &caps[2] {
        "s" => amount,
        "m" => amount * 60,
        "h" => amount * 3600,
        "d" => amount * 86400,
        _ => return None,
    };
    let now = Utc::now();
    let past = now - chrono::Duration::seconds(seconds);
    Some(past.to_rfc3339())
}

pub fn resolve_since(since: &str) -> Result<String, String> {
    if let Some(ts) = parse_relative_duration(since) {
        return Ok(ts);
    }
    if chrono::DateTime::parse_from_rfc3339(since).is_ok() {
        return Ok(since.to_string());
    }
    Err(format!(
        "Invalid since value: '{since}'. Use a relative duration (e.g., '5m', '1h', '30s') or an ISO 8601 timestamp."
    ))
}
