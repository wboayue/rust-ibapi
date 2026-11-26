use std::time::Duration;

use super::*;
use crate::common::test_utils::helpers::test_notice;

const USHMDS_BROKEN: (i32, &str) = (2105, "HMDS data farm connection is broken:ushmds");
const CASHHMDS_BROKEN: (i32, &str) = (2105, "HMDS data farm connection is broken:cashhmds");
const USHMDS_OK: (i32, &str) = (2106, "HMDS data farm connection is OK:ushmds");
const CASHHMDS_OK: (i32, &str) = (2106, "HMDS data farm connection is OK:cashhmds");
const USHMDS_INACTIVE: (i32, &str) = (2107, "HMDS data farm connection is inactive but should be available upon demand.ushmds");
const JFARM_BROKEN: (i32, &str) = (2103, "Market data farm connection is broken:jfarm");
const RESTORED: (i32, &str) = (
    1102,
    "Connectivity between IBKR and Trader Workstation has been restored - data maintained. All data farms are connected: cashhmds; ushmds.",
);

struct Case {
    name: &'static str,
    /// Notices before the mark.
    before: &'static [(i32, &'static str)],
    /// Notices after it.
    after: &'static [(i32, &'static str)],
    expected: Option<FarmRecovery>,
}

/// Notice sequences from the outage runs, and how a wait begun between them ends.
const CASES: &[Case] = &[
    Case {
        name: "no farm broken: waits, only the deadline ends it",
        before: &[],
        after: &[],
        expected: None,
    },
    Case {
        name: "both HMDS farms broken: waits",
        before: &[USHMDS_BROKEN, CASHHMDS_BROKEN],
        after: &[],
        expected: None,
    },
    Case {
        name: "one of two HMDS farms back: still waits",
        before: &[USHMDS_BROKEN, CASHHMDS_BROKEN],
        after: &[USHMDS_OK],
        expected: None,
    },
    Case {
        name: "both HMDS farms back: recovered",
        before: &[USHMDS_BROKEN, CASHHMDS_BROKEN],
        after: &[USHMDS_OK, CASHHMDS_OK],
        expected: Some(FarmRecovery::Recovered),
    },
    Case {
        name: "OK before the mark: nothing down, waits",
        before: &[USHMDS_BROKEN, USHMDS_OK],
        after: &[],
        expected: None,
    },
    Case {
        name: "an inactive HMDS farm is not broken, but not confirmed back",
        before: &[USHMDS_BROKEN],
        after: &[USHMDS_INACTIVE],
        expected: Some(FarmRecovery::Unconfirmed),
    },
    Case {
        name: "inactive, then OK: recovered",
        before: &[USHMDS_BROKEN],
        after: &[USHMDS_INACTIVE, USHMDS_OK],
        expected: Some(FarmRecovery::Recovered),
    },
    Case {
        name: "an OK from another farm does not confirm an inactive one",
        before: &[USHMDS_BROKEN, CASHHMDS_BROKEN],
        after: &[USHMDS_INACTIVE, CASHHMDS_OK],
        expected: Some(FarmRecovery::Unconfirmed),
    },
    Case {
        name: "broken again after inactive: waits",
        before: &[USHMDS_BROKEN, USHMDS_INACTIVE, USHMDS_BROKEN],
        after: &[],
        expected: None,
    },
    Case {
        name: "1102 confirms an inactive farm",
        before: &[USHMDS_BROKEN, USHMDS_INACTIVE],
        after: &[RESTORED],
        expected: Some(FarmRecovery::Recovered),
    },
    Case {
        name: "1102 before the mark clears an inactive farm: nothing down, waits",
        before: &[USHMDS_BROKEN, USHMDS_INACTIVE, RESTORED],
        after: &[],
        expected: None,
    },
    Case {
        name: "a broken market data farm is ignored: nothing down, waits",
        before: &[JFARM_BROKEN],
        after: &[],
        expected: None,
    },
    Case {
        name: "nothing down at the mark, then a break and its OK: recovered",
        before: &[],
        after: &[USHMDS_BROKEN, USHMDS_OK],
        expected: Some(FarmRecovery::Recovered),
    },
    Case {
        name: "nothing down at the mark, then a break and inactive: unconfirmed",
        before: &[],
        after: &[USHMDS_BROKEN, USHMDS_INACTIVE],
        expected: Some(FarmRecovery::Unconfirmed),
    },
    Case {
        name: "1102 ends the wait though no OK came",
        before: &[USHMDS_BROKEN],
        after: &[RESTORED],
        expected: Some(FarmRecovery::Recovered),
    },
    Case {
        name: "1102 before the mark does not",
        before: &[USHMDS_BROKEN, RESTORED],
        after: &[],
        expected: None,
    },
];

const DATA_LOST: (i32, &str) = (1101, "Connectivity between IBKR and Trader Workstation has been restored - data lost.");

/// A wait that should still be blocked is given this long.
const BLOCKED: Duration = Duration::from_millis(50);

fn feed(observe: impl Fn(&Notice), notices: &[(i32, &str)]) {
    for (code, message) in notices {
        observe(&test_notice(*code, *message));
    }
}

#[cfg(feature = "sync")]
mod sync_tests {
    use std::time::Instant;

    use super::*;
    use crate::transport::farm_watch::sync_impl::FarmWatch;

    #[test]
    fn wait_ends_per_notice_sequence() {
        for case in CASES {
            let watch = FarmWatch::default();
            feed(|n| watch.observe(n), case.before);
            let mark = watch.mark();
            feed(|n| watch.observe(n), case.after);

            let recovery = watch.wait(mark, Instant::now() + BLOCKED);
            assert_eq!(recovery, case.expected.unwrap_or(FarmRecovery::TimedOut), "{}", case.name);
        }
    }

    #[test]
    fn notice_wakes_a_blocked_waiter() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
        let mark = watch.mark();
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| watch.wait(mark, Instant::now() + Duration::from_secs(10)));
            std::thread::sleep(BLOCKED);
            assert!(!waiter.is_finished(), "returned while the farm was broken");
            feed(|n| watch.observe(n), &[USHMDS_OK]);
            assert_eq!(waiter.join().unwrap(), FarmRecovery::Recovered);
        });
    }

    #[test]
    fn reset_and_close_end_the_session() {
        for end in [FarmWatch::reset as fn(&FarmWatch), FarmWatch::close] {
            let watch = FarmWatch::default();
            feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
            let mark = watch.mark();
            std::thread::scope(|scope| {
                let waiter = scope.spawn(|| watch.wait(mark, Instant::now() + Duration::from_secs(10)));
                std::thread::sleep(BLOCKED);
                end(&watch);
                assert_eq!(waiter.join().unwrap(), FarmRecovery::SessionEnded);
            });
        }
    }

    /// The new session starts with no farm broken.
    #[test]
    fn reset_clears_broken_farms() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
        watch.reset();
        let mark = watch.mark();
        assert!(!mark.farm_down());
        assert_eq!(watch.wait(mark, Instant::now()), FarmRecovery::TimedOut);
    }

    /// 1101 counts as data lost; 1102 does not. Both wake the signal.
    #[test]
    fn data_lost_on_1101_only() {
        let watch = FarmWatch::default();
        let signal = watch.signal();
        feed(|n| watch.observe(n), &[DATA_LOST]);
        let mark = watch.mark();
        let _ = signal.receiver().try_recv();

        feed(|n| watch.observe(n), &[RESTORED]);
        let after_1102 = (watch.data_lost_since(mark), signal.receiver().try_recv().is_ok());
        feed(|n| watch.observe(n), &[DATA_LOST]);
        let after_1101 = (watch.data_lost_since(mark).map(|n| n.code), signal.receiver().try_recv().is_ok());

        assert_eq!(after_1102, (None, true));
        assert_eq!(after_1101, (Some(DATA_LOST.0), true));
    }

    /// An OK after the farm went inactive confirms it and wakes the signal;
    /// an OK from a farm never broken changes nothing.
    #[test]
    fn confirmed_after_inactive_farm_reports_ok() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN, USHMDS_INACTIVE]);
        let mark = watch.mark();
        let signal = watch.signal();

        feed(|n| watch.observe(n), &[CASHHMDS_OK]);
        let after_other_ok = (watch.confirmed(mark), signal.receiver().try_recv().is_ok());
        feed(|n| watch.observe(n), &[USHMDS_OK]);
        let after_ok = (watch.confirmed(mark), signal.receiver().try_recv().is_ok());

        assert_eq!(after_other_ok, (false, false));
        assert_eq!(after_ok, (true, true));
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use tokio::time::Instant;

    use super::*;
    use crate::transport::farm_watch::async_impl::FarmWatch;

    #[tokio::test]
    async fn wait_ends_per_notice_sequence() {
        for case in CASES {
            let watch = FarmWatch::default();
            feed(|n| watch.observe(n), case.before);
            let mark = watch.mark();
            feed(|n| watch.observe(n), case.after);

            let recovery = watch.wait(mark, Instant::now() + BLOCKED).await;
            assert_eq!(recovery, case.expected.unwrap_or(FarmRecovery::TimedOut), "{}", case.name);
        }
    }

    #[tokio::test]
    async fn notice_wakes_a_blocked_waiter() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
        let mark = watch.mark();
        let waiter = watch.wait(mark, Instant::now() + Duration::from_secs(10));
        tokio::pin!(waiter);
        assert!(
            tokio::time::timeout(BLOCKED, &mut waiter).await.is_err(),
            "returned while the farm was broken"
        );
        feed(|n| watch.observe(n), &[USHMDS_OK]);
        assert_eq!(waiter.await, FarmRecovery::Recovered);
    }

    #[tokio::test]
    async fn reset_and_close_end_the_session() {
        for end in [FarmWatch::reset as fn(&FarmWatch), FarmWatch::close] {
            let watch = FarmWatch::default();
            feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
            let mark = watch.mark();
            let waiter = watch.wait(mark, Instant::now() + Duration::from_secs(10));
            tokio::pin!(waiter);
            assert!(tokio::time::timeout(BLOCKED, &mut waiter).await.is_err());
            end(&watch);
            assert_eq!(waiter.await, FarmRecovery::SessionEnded);
        }
    }

    #[tokio::test]
    async fn reset_clears_broken_farms() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
        watch.reset();
        let mark = watch.mark();
        assert!(!mark.farm_down());
        assert_eq!(watch.wait(mark, Instant::now()).await, FarmRecovery::TimedOut);
    }

    /// Resolves on the first 1101 after the mark, not on an 1102.
    #[tokio::test]
    async fn data_lost_on_1101_only() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[DATA_LOST]);
        let mark = watch.mark();
        let lost = watch.data_lost(mark);
        tokio::pin!(lost);

        feed(|n| watch.observe(n), &[RESTORED]);
        let after_1102 = tokio::time::timeout(BLOCKED, &mut lost).await.is_err();
        feed(|n| watch.observe(n), &[DATA_LOST]);

        assert!(after_1102, "resolved on an 1102 or on the 1101 before the mark");
        assert_eq!(lost.await.code, DATA_LOST.0);
    }

    /// Resolves once the inactive farm reports OK, not on another farm's OK.
    #[tokio::test]
    async fn confirmed_after_inactive_farm_reports_ok() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN, USHMDS_INACTIVE]);
        let confirmed = watch.confirmed(watch.mark());
        tokio::pin!(confirmed);

        feed(|n| watch.observe(n), &[CASHHMDS_OK]);
        let after_other_ok = tokio::time::timeout(BLOCKED, &mut confirmed).await.is_err();
        feed(|n| watch.observe(n), &[USHMDS_OK]);

        assert!(after_other_ok, "resolved on an OK from a farm never broken");
        assert!(tokio::time::timeout(BLOCKED, confirmed).await.is_ok(), "did not resolve on the OK");
    }

    /// A waiter dropped mid-wait leaves no receiver behind.
    #[tokio::test]
    async fn dropped_waiter_leaves_nothing() {
        let watch = FarmWatch::default();
        feed(|n| watch.observe(n), &[USHMDS_BROKEN]);
        let mut waiter = Box::pin(watch.wait(watch.mark(), Instant::now() + Duration::from_secs(10)));
        assert!(tokio::time::timeout(BLOCKED, &mut waiter).await.is_err());
        assert_eq!(watch.waiters(), 1);
        drop(waiter);
        assert_eq!(watch.waiters(), 0);
    }
}
