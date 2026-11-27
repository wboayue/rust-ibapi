mod continuous_future_range_tests {
    use super::super::continuous_future_range;
    use crate::market_data::historical::Duration;
    use crate::Error;
    use time::macros::datetime;

    #[test]
    fn covers_start_to_now_without_end_date() {
        let range = continuous_future_range(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC)).unwrap();

        assert_eq!(range, (None, Duration::days(8)));
    }

    #[test]
    fn start_not_before_now_errors() {
        let now = datetime!(2026-04-15 0:00 UTC);

        assert!(matches!(continuous_future_range(now, now), Err(Error::InvalidArgument(_))));
    }
}

mod covering_duration_tests {
    use super::super::covering_duration;
    use crate::market_data::historical::Duration;
    use time::Duration as Span;

    #[test]
    fn up_to_one_day_uses_seconds() {
        assert_eq!(covering_duration(Span::hours(1)), Duration::seconds(3600));
        assert_eq!(covering_duration(Span::days(1)), Duration::seconds(86_400));
        assert_eq!(covering_duration(Span::milliseconds(500)), Duration::seconds(1));
    }

    // IBKR rejects `S` above 86400 (code 321); `D` is session-aligned, so one extra
    // session covers a range that starts mid-session.
    #[test]
    fn over_one_day_uses_days_plus_one_session() {
        assert_eq!(covering_duration(Span::days(1) + Span::seconds(1)), Duration::days(3));
        assert_eq!(covering_duration(Span::days(3)), Duration::days(4));
        assert_eq!(covering_duration(Span::days(364)), Duration::days(365));
    }

    // IBKR rejects `D` above 365 (code 321); `Y` is calendar years.
    #[test]
    fn over_365_sessions_uses_years() {
        assert_eq!(covering_duration(Span::days(365)), Duration::years(1));
        assert_eq!(covering_duration(Span::days(365) + Span::seconds(1)), Duration::years(2));
        assert_eq!(covering_duration(Span::days(3 * 365)), Duration::years(3));
    }
}
