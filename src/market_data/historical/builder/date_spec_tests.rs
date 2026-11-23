mod continuous_future_range_tests {
    use super::super::continuous_future_range;
    use crate::market_data::historical::{BarSize, Duration};
    use crate::Error;
    use time::macros::datetime;

    #[test]
    fn covers_start_to_now_without_end_date() {
        let range = continuous_future_range(datetime!(2026-04-08 0:00 UTC), datetime!(2026-04-15 0:00 UTC), BarSize::Hour).unwrap();

        assert_eq!(range, (None, Duration::days(8)));
    }

    #[test]
    fn start_not_before_now_errors() {
        let now = datetime!(2026-04-15 0:00 UTC);

        assert!(matches!(continuous_future_range(now, now, BarSize::Hour), Err(Error::InvalidArgument(_))));
    }

    // TWS serves at most 2000 S of 1-second bars, and a continuous future's request cannot be split.
    #[test]
    fn one_second_bars_within_limit_of_now() {
        let now = datetime!(2026-04-15 0:00 UTC);

        let at_limit = continuous_future_range(now - time::Duration::seconds(2000), now, BarSize::Sec);
        let past_limit = continuous_future_range(now - time::Duration::seconds(2001), now, BarSize::Sec);

        assert_eq!(at_limit.unwrap(), (None, Duration::seconds(2000)));
        assert!(
            matches!(&past_limit, Err(Error::InvalidArgument(message)) if message.contains("2000 S")),
            "got: {past_limit:?}"
        );
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
    }

    // TWS rejects `S` under 30 (code 321, live: 2026-11-23).
    #[test]
    fn under_thirty_seconds_asks_for_thirty() {
        assert_eq!(covering_duration(Span::milliseconds(500)), Duration::seconds(30));
        assert_eq!(covering_duration(Span::seconds(29)), Duration::seconds(30));
        assert_eq!(covering_duration(Span::seconds(30)), Duration::seconds(30));
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
