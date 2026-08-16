use time::OffsetDateTime;

pub fn format_relative(now: OffsetDateTime, then: OffsetDateTime) -> String {
    let secs = (now - then).whole_seconds().unsigned_abs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::format_relative;
    use time::OffsetDateTime;

    fn ts(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(secs).expect("ts")
    }

    #[test]
    fn formats_minutes_and_hours() {
        let now = ts(1_700_000_000);
        assert_eq!(format_relative(now, ts(1_699_999_400)), "10m");
        assert_eq!(format_relative(now, ts(1_699_992_800)), "2h");
        assert_eq!(format_relative(now, ts(1_699_827_200)), "2d");
    }
}
