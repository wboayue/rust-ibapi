use super::*;

#[test]
fn test_autofill_is_specified() {
    assert!(!AutoFill::default().is_specified());

    assert!(AutoFill {
        competitors: true,
        portfolio: false,
        watchlist: false,
    }
    .is_specified());

    assert!(AutoFill {
        competitors: false,
        portfolio: true,
        watchlist: false,
    }
    .is_specified());

    assert!(AutoFill {
        competitors: false,
        portfolio: false,
        watchlist: true,
    }
    .is_specified());
}

#[test]
fn test_autofill_combinations() {
    let combinations = vec![
        (false, false, false, false),
        (true, false, false, true),
        (false, true, false, true),
        (false, false, true, true),
        (true, true, false, true),
        (true, false, true, true),
        (false, true, true, true),
        (true, true, true, true),
    ];

    for (competitors, portfolio, watchlist, expected) in combinations {
        let autofill = AutoFill {
            competitors,
            portfolio,
            watchlist,
        };
        assert_eq!(
            autofill.is_specified(),
            expected,
            "Failed for combination: competitors={competitors}, portfolio={portfolio}, watchlist={watchlist}",
        );
    }
}
