use super::*;
use crate::common::test_utils::wire_enum::check_wire_code_round_trip;

#[test]
fn message_type_round_trips_every_wire_code() {
    check_wire_code_round_trip(
        &[
            (BulletinType::Regular, 1),
            (BulletinType::ExchangeUnavailable, 2),
            (BulletinType::ExchangeAvailable, 3),
            (BulletinType::Unknown(0), 0),
            (BulletinType::Unknown(4), 4),
            (BulletinType::Unknown(-1), -1),
        ],
        BulletinType::Unknown,
    );
}
