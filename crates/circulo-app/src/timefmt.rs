use time::{OffsetDateTime, UtcOffset};

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

pub fn local_offset_or_utc() -> UtcOffset {
    UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC)
}

pub fn is_same_local_day(now: OffsetDateTime, then: OffsetDateTime, offset: UtcOffset) -> bool {
    let now_local = now.to_offset(offset);
    let then_local = then.to_offset(offset);
    now_local.date() == then_local.date()
}

#[cfg(test)]
mod tests {
    use super::{format_relative, is_same_local_day};
    use time::{OffsetDateTime, UtcOffset};

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

    #[test]
    fn same_local_day_with_offset() {
        let offset = UtcOffset::from_hms(-5, 0, 0).expect("offset");
        // 2023-11-14 23:30 UTC = 2023-11-14 18:30 local (UTC-5)
        let evening = ts(1_700_000_000);
        // 2023-11-15 02:00 UTC = 2023-11-14 21:00 local (UTC-5) — same local day
        let later = ts(1_700_009_200);
        assert!(is_same_local_day(evening, later, offset));
    }

    #[test]
    fn different_local_day_with_offset() {
        let offset = UtcOffset::from_hms(-5, 0, 0).expect("offset");
        let day_one = ts(1_700_000_000);
        let day_two = ts(1_700_086_400);
        assert!(!is_same_local_day(day_one, day_two, offset));
    }
}
