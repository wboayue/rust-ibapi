use super::*;
use crate::orders::Order;

/// Mock client that gates on a fixed server version, mirroring
/// `Client::check_server_version` without a live connection.
struct MockClient {
    server_version: i32,
}

impl VersionedClient for MockClient {
    fn check_version(&self, version: i32, message: &str) -> Result<(), Error> {
        if version <= self.server_version {
            Ok(())
        } else {
            Err(Error::ServerVersion(version, self.server_version, message.into()))
        }
    }
}

#[test]
fn hedge_max_size_rejected_below_gate() {
    let order = Order {
        hedge_max_size: Some(100),
        ..Default::default()
    };
    let client = MockClient {
        server_version: server_versions::HEDGE_MAX_SIZE - 1,
    };

    match verify_order(&client, &order, 1) {
        Err(Error::ServerVersion(required, actual, _)) => {
            assert_eq!(required, server_versions::HEDGE_MAX_SIZE);
            assert_eq!(actual, server_versions::HEDGE_MAX_SIZE - 1);
        }
        other => panic!("expected ServerVersion error, got {other:?}"),
    }
}

#[test]
fn hedge_max_size_accepted_at_gate() {
    let order = Order {
        hedge_max_size: Some(100),
        ..Default::default()
    };
    let client = MockClient {
        server_version: server_versions::HEDGE_MAX_SIZE,
    };

    assert!(verify_order(&client, &order, 1).is_ok());
}

#[test]
fn hedge_max_size_none_skips_gate() {
    let order = Order::default();
    let client = MockClient {
        server_version: server_versions::HEDGE_MAX_SIZE - 1,
    };

    assert!(verify_order(&client, &order, 1).is_ok());
}
