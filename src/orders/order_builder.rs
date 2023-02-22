use super::{Action, Order};

/// Make sure to test using only your paper trading account when applicable. A good way of finding out if an order type/exchange combination
/// is possible is by trying to place such order manually using the TWS.
/// Before contacting our API support team please refer to the available documentation.

/// An auction order is entered into the electronic trading system during the pre-market opening period for execution at the
/// Calculated Opening Price (COP). If your order is not filled on the open, the order is re-submitted as a limit order with
/// the limit price set to the COP or the best bid/ask after the market opens.
/// Products: FUT, STK
pub fn at_auction(action: Action, quantity: f64, price: f64) -> Order {
    Order {
        action,
        tif: "AUC".to_owned(),
        order_type: "MTL".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price),
        ..Order::default()
    }
}

/// A Discretionary order is a limit order submitted with a hidden, specified 'discretionary' amount off the limit price which
/// may be used to increase the price range over which the limit order is eligible to execute. The market sees only the limit price.
/// Products: STK
pub fn discretionary(
    action: Action,
    quantity: f64,
    price: f64,
    discretionary_amount: f64,
) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price),
        discretionary_amt: discretionary_amount,
        ..Order::default()
    }
}

/// A Market order is an order to buy or sell at the market bid or offer price. A market order may increase the likelihood of a fill
/// and the speed of execution, but unlike the Limit order a Market order provides no price protection and may fill at a price far
/// lower/higher than the current displayed bid/ask.
/// Products: BOND, CFD, EFP, CASH, FUND, FUT, FOP, OPT, STK, WAR
pub fn market_order(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "MKT".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    }
}
