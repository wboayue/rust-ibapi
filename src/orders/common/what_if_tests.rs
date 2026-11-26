use super::*;
use crate::orders::OrderStatusKind;

const WARNING: &str = "If your order is not immediately executable, our systems may reject it";

fn warning_only() -> OrderState {
    OrderState {
        status: OrderStatusKind::PreSubmitted,
        commission_and_fees: Some(0.0),
        warning_text: WARNING.to_string(),
        ..Default::default()
    }
}

fn with_margins() -> OrderState {
    OrderState {
        status: OrderStatusKind::PreSubmitted,
        initial_margin_before: Some(257638.32),
        initial_margin_change: Some(578.24),
        initial_margin_after: Some(258216.56),
        commission_and_fees: Some(1.00003),
        commission_and_fees_currency: "USD".to_string(),
        margin_currency: "USD".to_string(),
        ..Default::default()
    }
}

fn over_margin() -> OrderState {
    OrderState {
        commission_and_fees: None,
        commission_and_fees_currency: String::new(),
        suggested_size: Some(4000.0),
        ..with_margins()
    }
}

#[test]
fn single_frame_with_margins_is_the_answer() {
    let mut reply = WhatIfReply::default();

    assert_eq!(reply.observe(with_margins()), Some(with_margins()));
}

#[test]
fn warning_only_frame_waits_for_margins_and_keeps_its_warning() {
    let mut reply = WhatIfReply::default();

    let first = reply.observe(warning_only());
    let second = reply.observe(with_margins());

    let expected = OrderState {
        warning_text: WARNING.to_string(),
        ..with_margins()
    };
    assert_eq!(first, None);
    assert_eq!(second, Some(expected));
}

#[test]
fn warning_of_the_answer_frame_wins() {
    let mut reply = WhatIfReply::default();
    let answer = OrderState {
        warning_text: "second".to_string(),
        ..with_margins()
    };

    let first = reply.observe(warning_only());
    let second = reply.observe(answer.clone());

    assert_eq!(first, None);
    assert_eq!(second, Some(answer));
}

#[test]
fn frame_without_margins_or_warning_is_the_answer() {
    let mut reply = WhatIfReply::default();
    let bare = OrderState {
        status: OrderStatusKind::PreSubmitted,
        ..Default::default()
    };

    assert_eq!(reply.observe(bare.clone()), Some(bare));
}

#[test]
fn over_margin_frame_waits_for_the_rejection() {
    let mut reply = WhatIfReply::default();

    assert_eq!(reply.observe(over_margin()), None);
}

#[test]
fn reply_deadline_uses_the_configured_timeout_when_representable() {
    let start = Instant::now();
    let timeout = ONE_SHOT_TIMEOUT * 2;

    assert_eq!(reply_deadline(start, None), Some(start + ONE_SHOT_TIMEOUT));
    assert_eq!(reply_deadline(start, Some(timeout)), Some(start + timeout));
    assert_eq!(reply_deadline(start, Some(Duration::MAX)), None);
}
