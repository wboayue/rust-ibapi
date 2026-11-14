use time::macros::datetime;

use super::*;

/// `before` and `since` set the one bound TWS reads; the later call replaces the earlier.
#[test]
fn last_bound_wins() {
    let before = datetime!(2026-11-04 12:00 UTC);
    let since = datetime!(2026-11-01 0:00 UTC);
    let cases = [
        ("no bound", HistoricalNewsBuilder::new(&(), 1, &[]), None),
        (
            "before",
            HistoricalNewsBuilder::new(&(), 1, &[]).before(before),
            Some(NewsBound::Before(before)),
        ),
        (
            "since",
            HistoricalNewsBuilder::new(&(), 1, &[]).since(since),
            Some(NewsBound::Since(since)),
        ),
        (
            "before, then since",
            HistoricalNewsBuilder::new(&(), 1, &[]).before(before).since(since),
            Some(NewsBound::Since(since)),
        ),
        (
            "since, then before",
            HistoricalNewsBuilder::new(&(), 1, &[]).since(since).before(before),
            Some(NewsBound::Before(before)),
        ),
    ];
    for (label, builder, expected) in cases {
        assert_eq!(builder.bound, expected, "{label}");
    }
}
