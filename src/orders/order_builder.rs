use super::{Action, Order, OrderComboLeg, TagValue};

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
pub fn discretionary(action: Action, quantity: f64, price: f64, discretionary_amount: f64) -> Order {
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

/// A Market if Touched (MIT) is an order to buy (or sell) a contract below (or above) the market. Its purpose is to take advantage
/// of sudden or unexpected changes in share or other prices and provides investors with a trigger price to set an order in motion.
/// Investors may be waiting for excessive strength (or weakness) to cease, which might be represented by a specific price point.
/// MIT orders can be used to determine whether or not to enter the market once a specific price level has been achieved. This order
/// is held in the system until the trigger price is touched, and is then submitted as a market order. An MIT order is similar to a
/// stop order, except that an MIT sell order is placed above the current market price, and a stop sell order is placed below
/// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
pub fn market_if_touched(action: Action, quantity: f64, price: f64) -> Order {
    Order {
        action,
        order_type: "MIT".to_owned(),
        total_quantity: quantity,
        aux_price: Some(price),
        ..Order::default()
    }
}

/// A Market-on-Close (MOC) order is a market order that is submitted to execute as close to the closing price as possible.
/// Products: CFD, FUT, STK, WAR
pub fn market_on_close(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "MOC".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    }
}

/// A Market-on-Open (MOO) order combines a market order with the OPG time in force to create an order that is automatically
/// submitted at the market's open and fills at the market price.
/// Products: CFD, STK, OPT, WAR
pub fn market_on_open(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "MKT".to_owned(),
        total_quantity: quantity,
        tif: "OPG".to_owned(),
        ..Order::default()
    }
}

/// ISE Midpoint Match (MPM) orders always execute at the midpoint of the NBBO. You can submit market and limit orders direct-routed
/// to ISE for MPM execution. Market orders execute at the midpoint whenever an eligible contra-order is available. Limit orders
/// execute only when the midpoint price is better than the limit price. Standard MPM orders are completely anonymous.
/// Products: STK
pub fn midpoint_match(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "MKT".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    }
}

// A Midprice order is designed to split the difference between the bid and ask prices, and fill at the current midpoint of
// the NBBO or better. Set an optional price cap to define the highest price (for a buy order) or the lowest price (for a sell
// order) you are willing to accept. Requires TWS 975+. Smart-routing to US stocks only.
pub fn midprice(action: Action, quantity: f64, price_cap: f64) -> Order {
    Order {
        action,
        order_type: "MIDPRICE".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price_cap),
        ..Order::default()
    }
}

/// A pegged-to-market order is designed to maintain a purchase price relative to the national best offer (NBO) or a sale price
/// relative to the national best bid (NBB). Depending on the width of the quote, this order may be passive or aggressive.
/// The trader creates the order by entering a limit price which defines the worst limit price that they are willing to accept.
/// Next, the trader enters an offset amount which computes the active limit price as follows:
///     Sell order price = Bid price + offset amount
///     Buy order price = Ask price - offset amount
/// Products: STK
pub fn pegged_to_market(action: Action, quantity: f64, market_offset: f64) -> Order {
    Order {
        action,
        order_type: "PEG MKT".to_owned(),
        total_quantity: quantity, // TODO: why was this 100?
        aux_price: Some(market_offset),
        ..Order::default()
    }
}

/// A Pegged to Stock order continually adjusts the option order price by the product of a signed user-define delta and the change of
/// the option's underlying stock price. The delta is entered as an absolute and assumed to be positive for calls and negative for puts.
/// A buy or sell call order price is determined by adding the delta times a change in an underlying stock price to a specified starting
/// price for the call. To determine the change in price, the stock reference price is subtracted from the current NBBO midpoint.
/// The Stock Reference Price can be defined by the user, or defaults to the NBBO midpoint at the time of the order if no reference price
/// is entered. You may also enter a high/low stock price range which cancels the order when reached. The delta times the change in stock
/// price will be rounded to the nearest penny in favor of the order.
/// Products: OPT
pub fn pegged_to_stock(action: Action, quantity: f64, delta: f64, stock_reference_price: f64, starting_price: f64) -> Order {
    Order {
        action,
        order_type: "PEG STK".to_owned(),
        total_quantity: quantity,
        delta: Some(delta),
        stock_ref_price: Some(stock_reference_price),
        starting_price: Some(starting_price),
        ..Order::default()
    }
}

/// Relative (a.k.a. Pegged-to-Primary) orders provide a means for traders to seek a more aggressive price than the National Best Bid
/// and Offer (NBBO). By acting as liquidity providers, and placing more aggressive bids and offers than the current best bids and offers,
/// traders increase their odds of filling their order. Quotes are automatically adjusted as the markets move, to remain aggressive.
/// For a buy order, your bid is pegged to the NBB by a more aggressive offset, and if the NBB moves up, your bid will also move up.
/// If the NBB moves down, there will be no adjustment because your bid will become even more aggressive and execute. For sales, your
/// offer is pegged to the NBO by a more aggressive offset, and if the NBO moves down, your offer will also move down. If the NBO moves up,
/// there will be no adjustment because your offer will become more aggressive and execute. In addition to the offset, you can define an
/// absolute cap, which works like a limit price, and will prevent your order from being executed above or below a specified level.
/// Stocks, Options and Futures - not available on paper trading
/// Products: CFD, STK, OPT, FUT
pub fn relative_pegged_to_primary(action: Action, quantity: f64, price_cap: f64, offset_amount: f64) -> Order {
    Order {
        action,
        order_type: "REL".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price_cap),
        aux_price: Some(offset_amount),
        ..Order::default()
    }
}

/// Sweep-to-fill orders are useful when a trader values speed of execution over price. A sweep-to-fill order identifies the best price
/// and the exact quantity offered/available at that price, and transmits the corresponding portion of your order for immediate execution.
/// Simultaneously it identifies the next best price and quantity offered/available, and submits the matching quantity of your order for
/// immediate execution.
/// Products: CFD, STK, WAR
pub fn sweep_to_fill(action: Action, quantity: f64, price: f64) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price),
        sweep_to_fill: true,
        ..Order::default()
    }
}

/// For option orders routed to the Boston Options Exchange (BOX) you may elect to participate in the BOX's price improvement auction in
/// pennies. All BOX-directed price improvement orders are immediately sent from Interactive Brokers to the BOX order book, and when the
/// terms allow, IB will evaluate it for inclusion in a price improvement auction based on price and volume priority. In the auction, your
/// order will have priority over broker-dealer price improvement orders at the same price.
/// An Auction Limit order at a specified price. Use of a limit order ensures that you will not receive an execution at a price less favorable
/// than the limit price. Enter limit orders in penny increments with your auction improvement amount computed as the difference between your
/// limit order price and the nearest listed increment.
/// Products: OPT
/// Supported Exchanges: BOX
pub fn auction_limit(action: Action, quantity: f64, price: f64, auction_strategy: i32) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price),
        auction_strategy: Some(auction_strategy),
        ..Order::default()
    }
}

/// For option orders routed to the Boston Options Exchange (BOX) you may elect to participate in the BOX's price improvement auction in pennies.
/// All BOX-directed price improvement orders are immediately sent from Interactive Brokers to the BOX order book, and when the terms allow,
/// IB will evaluate it for inclusion in a price improvement auction based on price and volume priority. In the auction, your order will have
/// priority over broker-dealer price improvement orders at the same price.
/// An Auction Pegged to Stock order adjusts the order price by the product of a signed delta (which is entered as an absolute and assumed to be
/// positive for calls, negative for puts) and the change of the option's underlying stock price. A buy or sell call order price is determined
/// by adding the delta times a change in an underlying stock price change to a specified starting price for the call. To determine the change
/// in price, a stock reference price (NBBO midpoint at the time of the order is assumed if no reference price is entered) is subtracted from
/// the current NBBO midpoint. A stock range may also be entered that cancels an order when reached. The delta times the change in stock price
/// will be rounded to the nearest penny in favor of the order and will be used as your auction improvement amount.
/// Products: OPT
/// Supported Exchanges: BOX
pub fn auction_pegged_to_stock(action: Action, quantity: f64, starting_price: f64, delta: f64) -> Order {
    Order {
        action,
        order_type: "PEG STK".to_owned(),
        total_quantity: quantity,
        delta: Some(delta),
        starting_price: Some(starting_price),
        ..Order::default()
    }
}

/// For option orders routed to the Boston Options Exchange (BOX) you may elect to participate in the BOX's price improvement auction in pennies.
/// All BOX-directed price improvement orders are immediately sent from Interactive Brokers to the BOX order book, and when the terms allow,
/// IB will evaluate it for inclusion in a price improvement auction based on price and volume priority. In the auction, your order will have
/// priority over broker-dealer price improvement orders at the same price.
/// An Auction Relative order that adjusts the order price by the product of a signed delta (which is entered as an absolute and assumed to be
/// positive for calls, negative for puts) and the change of the option's underlying stock price. A buy or sell call order price is determined
/// by adding the delta times a change in an underlying stock price change to a specified starting price for the call. To determine the change
/// in price, a stock reference price (NBBO midpoint at the time of the order is assumed if no reference price is entered) is subtracted from
/// the current NBBO midpoint. A stock range may also be entered that cancels an order when reached. The delta times the change in stock price
/// will be rounded to the nearest penny in favor of the order and will be used as your auction improvement amount.
/// Products: OPT
/// Supported Exchanges: BOX
pub fn auction_relative(action: Action, quantity: f64, offset: f64) -> Order {
    Order {
        action,
        order_type: "REL".to_owned(),
        total_quantity: quantity,
        aux_price: Some(offset),
        ..Order::default()
    }
}

/// The Block attribute is used for large volume option orders on ISE that consist of at least 50 contracts. To execute large-volume
/// orders over time without moving the market, use the Accumulate/Distribute algorithm.
/// Products: OPT
pub fn block(action: Action, quantity: f64, price: f64) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(price),
        block_order: true,
        ..Order::default()
    }
}

/// A Box Top order executes as a market order at the current best price. If the order is only partially filled, the remainder is submitted as
/// a limit order with the limit price equal to the price at which the filled portion of the order executed.
/// Products: OPT
/// Supported Exchanges: BOX
pub fn box_top(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "BOX TOP".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    }
}

/// A Limit order is an order to buy or sell at a specified price or better. The Limit order ensures that if the order fills,
/// it will not fill at a price less favorable than your limit price, but it does not guarantee a fill.
/// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
pub fn limit_order(action: Action, quantity: f64, limit_price: f64) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        ..Order::default()
    }
}

/// Forex orders can be placed in demonination of second currency in pair using cash_qty field
/// Requires TWS or IBG 963+
/// https://www.interactivebrokers.com/en/index.php?f=23876#963-02
pub fn limit_order_with_cash_qty(action: Action, limit_price: f64, cash_qty: f64) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        limit_price: Some(limit_price),
        cash_qty: Some(cash_qty),
        ..Order::default()
    }
}

/// A Limit if Touched is an order to buy (or sell) a contract at a specified price or better, below (or above) the market. This order is
/// held in the system until the trigger price is touched. An LIT order is similar to a stop limit order, except that an LIT sell order is
/// placed above the current market price, and a stop limit sell order is placed below.
/// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
pub fn limit_if_touched(action: Action, quantity: f64, limit_price: f64, trigger_price: f64) -> Order {
    Order {
        action,
        order_type: "LIT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        aux_price: Some(trigger_price),
        ..Order::default()
    }
}

/// A Limit-on-close (LOC) order will be submitted at the close and will execute if the closing price is at or better than the submitted
/// limit price.
/// Products: CFD, FUT, STK, WAR
pub fn limit_on_close(action: Action, quantity: f64, limit_price: f64) -> Order {
    Order {
        action,
        order_type: "LOC".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        ..Order::default()
    }
}

/// A Limit-on-Open (LOO) order combines a limit order with the OPG time in force to create an order that is submitted at the market's open,
/// and that will only execute at the specified limit price or better. Orders are filled in accordance with specific exchange rules.
/// Products: CFD, STK, OPT, WAR
pub fn limit_on_open(action: Action, quantity: f64, limit_price: f64) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        tif: "OPG".to_owned(),
        ..Order::default()
    }
}

/// Passive Relative orders provide a means for traders to seek a less aggressive price than the National Best Bid and Offer (NBBO) while
/// keeping the order pegged to the best bid (for a buy) or ask (for a sell). The order price is automatically adjusted as the markets move
/// to keep the order less aggressive. For a buy order, your order price is pegged to the NBB by a less aggressive offset, and if the NBB
/// moves up, your bid will also move up. If the NBB moves down, there will be no adjustment because your bid will become aggressive and execute.
/// For a sell order, your price is pegged to the NBO by a less aggressive offset, and if the NBO moves down, your offer will also move down.
/// If the NBO moves up, there will be no adjustment because your offer will become aggressive and execute. In addition to the offset, you can
/// define an absolute cap, which works like a limit price, and will prevent your order from being executed above or below a specified level.
/// The Passive Relative order is similar to the Relative/Pegged-to-Primary order, except that the Passive relative subtracts the offset from
/// the bid and the Relative adds the offset to the bid.
/// Products: STK, WAR
pub fn passive_relative(action: Action, quantity: f64, offset: f64) -> Order {
    Order {
        action,
        order_type: "PASSV REL".to_owned(),
        total_quantity: quantity,
        aux_price: Some(offset),
        ..Order::default()
    }
}

/// A pegged-to-midpoint order provides a means for traders to seek a price at the midpoint of the National Best Bid and Offer (NBBO).
/// The price automatically adjusts to peg the midpoint as the markets move, to remain aggressive. For a buy order, your bid is pegged to
/// the NBBO midpoint and the order price adjusts automatically to continue to peg the midpoint if the market moves. The price only adjusts
/// to be more aggressive. If the market moves in the opposite direction, the order will execute.
/// Products: STK
pub fn pegged_to_midpoint(action: Action, quantity: f64, offset: f64, limit_price: f64) -> Order {
    Order {
        action,
        order_type: "PEG MID".to_owned(),
        total_quantity: quantity,
        aux_price: Some(offset),
        limit_price: Some(limit_price),
        ..Order::default()
    }
}

/// Bracket orders are designed to help limit your loss and lock in a profit by "bracketing" an order with two opposite-side orders.
/// A BUY order is bracketed by a high-side sell limit order and a low-side sell stop order. A SELL order is bracketed by a high-side buy
/// stop order and a low side buy limit order.
/// Products: CFD, BAG, FOP, CASH, FUT, OPT, STK, WAR
pub fn bracket_order(
    parent_order_id: i32,
    action: Action,
    quantity: f64,
    limit_price: f64,
    take_profit_limit_price: f64,
    stop_loss_price: f64,
) -> Vec<Order> {
    //This will be our main or "parent" order
    let parent = Order {
        order_id: parent_order_id,
        action: action.clone(),
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        transmit: false,
        ..Order::default()
    };

    let take_profit = Order {
        order_id: parent.order_id + 1,
        action: action.reverse(),
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(take_profit_limit_price),
        parent_id: parent_order_id,
        transmit: false,
        ..Order::default()
    };

    let stop_loss = Order {
        order_id: parent.order_id + 2,
        action: action.reverse(),
        order_type: "STP".to_owned(),
        //Stop trigger price
        aux_price: Some(stop_loss_price),
        total_quantity: quantity,
        parent_id: parent_order_id,
        //In this case, the low side order will be the last child being sent. Therefore, it needs to set this attribute to true
        //to activate all its predecessors
        transmit: true,
        ..Order::default()
    };

    vec![parent, take_profit, stop_loss]
}

/// Products:CFD, FUT, FOP, OPT, STK, WAR
/// A Market-to-Limit (MTL) order is submitted as a market order to execute at the current best market price. If the order is only
/// partially filled, the remainder of the order is canceled and re-submitted as a limit order with the limit price equal to the price
/// at which the filled portion of the order executed.
pub fn market_to_limit(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "MTL".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    }
}

/// This order type is useful for futures traders using Globex. A Market with Protection order is a market order that will be cancelled and
/// resubmitted as a limit order if the entire order does not immediately execute at the market price. The limit price is set by Globex to be
/// close to the current market price, slightly higher for a sell order and lower for a buy order.
/// Products: FUT, FOP
pub fn market_with_protection(action: Action, quantity: f64) -> Order {
    Order {
        action,
        order_type: "MKT PRT".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    }
}

/// A Stop order is an instruction to submit a buy or sell market order if and when the user-specified stop trigger price is attained or
/// penetrated. A Stop order is not guaranteed a specific execution price and may execute significantly away from its stop price. A Sell
/// Stop order is always placed below the current market price and is typically used to limit a loss or protect a profit on a long stock
/// position. A Buy Stop order is always placed above the current market price. It is typically used to limit a loss or help protect a
/// profit on a short sale.
/// Products: CFD, BAG, CASH, FUT, FOP, OPT, STK, WAR
pub fn stop(action: Action, quantity: f64, stop_price: f64) -> Order {
    Order {
        action,
        order_type: "STP".to_owned(),
        total_quantity: quantity,
        aux_price: Some(stop_price),
        ..Order::default()
    }
}

/// A Stop-Limit order is an instruction to submit a buy or sell limit order when the user-specified stop trigger price is attained or
/// penetrated. The order has two basic components: the stop price and the limit price. When a trade has occurred at or through the stop
/// price, the order becomes executable and enters the market as a limit order, which is an order to buy or sell at a specified price or better.
/// Products: CFD, CASH, FUT, FOP, OPT, STK, WAR
pub fn stop_limit(action: Action, quantity: f64, limit_price: f64, stop_price: f64) -> Order {
    Order {
        action,
        order_type: "STP LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        aux_price: Some(stop_price),
        ..Order::default()
    }
}

/// A Stop with Protection order combines the functionality of a stop limit order with a market with protection order. The order is set
/// to trigger at a specified stop price. When the stop price is penetrated, the order is triggered as a market with protection order,
/// which means that it will fill within a specified protected price range equal to the trigger price +/- the exchange-defined protection
/// point range. Any portion of the order that does not fill within this protected range is submitted as a limit order at the exchange-defined
/// trigger price +/- the protection points.
/// Products: FUT
pub fn stop_with_protection(action: Action, quantity: f64, stop_price: f64) -> Order {
    Order {
        action,
        order_type: "STP PRT".to_owned(),
        total_quantity: quantity,
        aux_price: Some(stop_price),
        ..Order::default()
    }
}

/// A sell trailing stop order sets the stop price at a fixed amount below the market price with an attached "trailing" amount. As the
/// market price rises, the stop price rises by the trail amount, but if the stock price falls, the stop loss price doesn't change,
/// and a market order is submitted when the stop price is hit. This technique is designed to allow an investor to specify a limit on the
/// maximum possible loss, without setting a limit on the maximum possible gain. "Buy" trailing stop orders are the mirror image of sell
/// trailing stop orders, and are most appropriate for use in falling markets.
/// Products: CFD, CASH, FOP, FUT, OPT, STK, WAR
pub fn trailing_stop(action: Action, quantity: f64, trailing_percent: f64, trail_stop_price: f64) -> Order {
    Order {
        action,
        order_type: "TRAIL".to_owned(),
        total_quantity: quantity,
        trailing_percent: Some(trailing_percent),
        trail_stop_price: Some(trail_stop_price),
        ..Order::default()
    }
}

/// A trailing stop limit order is designed to allow an investor to specify a limit on the maximum possible loss, without setting a limit
/// on the maximum possible gain. A SELL trailing stop limit moves with the market price, and continually recalculates the stop trigger
/// price at a fixed amount below the market price, based on the user-defined "trailing" amount. The limit order price is also continually
/// recalculated based on the limit offset. As the market price rises, both the stop price and the limit price rise by the trail amount and
/// limit offset respectively, but if the stock price falls, the stop price remains unchanged, and when the stop price is hit a limit order
/// is submitted at the last calculated limit price. A "Buy" trailing stop limit order is the mirror image of a sell trailing stop limit,
/// and is generally used in falling markets.
/// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
pub fn trailing_stop_limit(action: Action, quantity: f64, lmt_price_offset: f64, trailing_amount: f64, trail_stop_price: f64) -> Order {
    Order {
        action,
        order_type: "TRAIL LIMIT".to_owned(),
        total_quantity: quantity,
        trail_stop_price: Some(trail_stop_price),
        lmt_price_offset: Some(lmt_price_offset),
        aux_price: Some(trailing_amount),
        ..Order::default()
    }
}

/// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
/// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
/// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
/// best execution.
/// Products: OPT, STK, FUT
pub fn combo_limit_order(action: Action, quantity: f64, limit_price: f64, non_guaranteed: bool) -> Order {
    let mut order = Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        ..Order::default()
    };

    if non_guaranteed {
        order = tag_order_non_guaranteed(order)
    }

    order
}

fn tag_order_non_guaranteed(mut order: Order) -> Order {
    order.smart_combo_routing_params = vec![];
    order.smart_combo_routing_params.push(TagValue {
        tag: "NonGuaranteed".to_owned(),
        value: "1".to_owned(),
    });
    order
}

/// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
/// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
/// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
/// best execution.
/// Products: OPT, STK, FUT
pub fn combo_market_order(action: Action, quantity: f64, non_guaranteed: bool) -> Order {
    let mut order = Order {
        action,
        order_type: "MKT".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    };

    if non_guaranteed {
        order = tag_order_non_guaranteed(order)
    }

    order
}

/// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
/// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
/// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
/// best execution.
/// Products: OPT, STK, FUT
pub fn limit_order_for_combo_with_leg_prices(action: Action, quantity: f64, leg_prices: Vec<f64>, non_guaranteed: bool) -> Order {
    let mut order = Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        order_combo_legs: vec![],
        ..Order::default()
    };

    for price in leg_prices {
        order.order_combo_legs.push(OrderComboLeg { price: Some(price) });
    }

    if non_guaranteed {
        order = tag_order_non_guaranteed(order)
    }

    order
}

/// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
/// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
/// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
/// best execution.
/// Products: OPT, STK, FUT
pub fn relative_limit_combo(action: Action, quantity: f64, limit_price: f64, non_guaranteed: bool) -> Order {
    let mut order = Order {
        action,
        order_type: "REL + LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        ..Order::default()
    };

    if non_guaranteed {
        order = tag_order_non_guaranteed(order)
    }

    order
}

/// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
/// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
/// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
/// best execution.
/// Products: OPT, STK, FUT
pub fn relative_market_combo(action: Action, quantity: f64, non_guaranteed: bool) -> Order {
    let mut order = Order {
        action,
        order_type: "REL + MKT".to_owned(),
        total_quantity: quantity,
        ..Order::default()
    };

    if non_guaranteed {
        order = tag_order_non_guaranteed(order)
    }

    order
}

/// One-Cancels All (OCA) order type allows an investor to place multiple and possibly unrelated orders assigned to a group. The aim is
/// to complete just one of the orders, which in turn will cause TWS to cancel the remaining orders. The investor may submit several
/// orders aimed at taking advantage of the most desirable price within the group. Completion of one piece of the group order causes
/// cancellation of the remaining group orders while partial completion causes the group to rebalance. An investor might desire to sell
/// 1000 shares of only ONE of three positions held above prevailing market prices. The OCA order group allows the investor to enter prices
/// at specified target levels and if one is completed, the other two will automatically cancel. Alternatively, an investor may wish to take
/// a LONG position in eMini S&P stock index futures in a falling market or else SELL US treasury futures at a more favorable price.
/// Grouping the two orders using an OCA order type offers the investor two chance to enter a similar position, while only running the risk
/// of taking on a single position.
/// Products: BOND, CASH, FUT, FOP, STK, OPT, WAR
pub fn one_cancels_all(oca_group: &str, mut oca_orders: Vec<Order>, oca_type: i32) -> Vec<Order> {
    for order in &mut oca_orders {
        order.oca_group = oca_group.to_owned();
        order.oca_type = oca_type;
    }

    oca_orders
}

/// Specific to US options, investors are able to create and enter Volatility-type orders for options and combinations rather than price orders.
/// Option traders may wish to trade and position for movements in the price of the option determined by its implied volatility. Because
/// implied volatility is a key determinant of the premium on an option, traders position in specific contract months in an effort to take
/// advantage of perceived changes in implied volatility arising before, during or after earnings or when company specific or broad market
/// volatility is predicted to change. In order to create a Volatility order, clients must first create a Volatility Trader page from the
/// Trading Tools menu and as they enter option contracts, premiums will display in percentage terms rather than premium. The buy/sell process
/// is the same as for regular orders priced in premium terms except that the client can limit the volatility level they are willing to pay or
/// receive.
/// Products: FOP, OPT
pub fn volatility(action: Action, quantity: f64, volatility_percent: f64, volatility_type: i32) -> Order {
    Order {
        action,
        order_type: "VOL".to_owned(),
        total_quantity: quantity,
        volatility: Some(volatility_percent),   //Expressed in percentage (40%)
        volatility_type: Some(volatility_type), // 1=daily, 2=annual
        ..Order::default()
    }
}

pub fn market_f_hedge(parent_order_id: i32, action: Action) -> Order {
    //FX Hedge orders can only have a quantity of 0
    let mut order = market_order(action, 0.0);
    order.parent_id = parent_order_id;
    order.hedge_type = "F".to_owned();

    order
}

pub fn pegged_to_benchmark(
    action: Action,
    quantity: f64,
    starting_price: f64,
    pegged_change_amount_decrease: bool,
    pegged_change_amount: f64,
    reference_change_amount: f64,
    reference_contract_id: i32,
    reference_exchange: &str,
    stock_reference_price: f64,
    reference_contract_lower_range: f64,
    reference_contract_upper_range: f64,
) -> Order {
    Order {
        action,
        order_type: "PEG BENCH".to_owned(),
        total_quantity: quantity,
        starting_price: Some(starting_price),
        is_pegged_change_amount_decrease: pegged_change_amount_decrease,
        pegged_change_amount,                              // by ... (and likewise for price moving in opposite direction)
        reference_change_amount,                           // whenever there is a price change of ...
        reference_contract_id,                             // in the reference contract ...
        reference_exchange: reference_exchange.to_owned(), // being traded at ...
        stock_ref_price: Some(stock_reference_price),      // starting reference price is ...
        //Keep order active as long as reference contract trades between ...
        stock_range_lower: Some(reference_contract_lower_range),
        stock_range_upper: Some(reference_contract_upper_range),
        ..Order::default()
    }
}

/// An attached order that turns the parent order (a conventional STP order) into a STP order
/// in the opposite direction when the trigger is hit.
pub fn attach_adjustable_to_stop(parent: &Order, attached_order_stop_price: f64, trigger_price: f64, adjusted_stop_price: f64) -> Order {
    // Attached order is a conventional STP order
    let mut order = stop(parent.action.reverse(), parent.total_quantity, attached_order_stop_price);

    order.parent_id = parent.order_id;
    order.trigger_price = Some(trigger_price); // When trigger price is penetrated
    order.adjusted_order_type = "STP".to_owned(); // The parent order will be turned into a STP order
    order.adjusted_stop_price = Some(adjusted_stop_price); // With the given STP price

    order
}

/// An attached order that turns the parent order (a conventional STP order) into a STP LMT order
/// in the opposite direction when the trigger is hit.
pub fn attach_adjustable_to_stop_limit(
    parent: &Order,
    attached_order_stop_price: f64,
    trigger_price: f64,
    adjusted_stop_price: f64,
    adjusted_stop_limit_price: f64,
) -> Order {
    // Attached order is a conventional STP order
    let mut order = stop(parent.action.reverse(), parent.total_quantity, attached_order_stop_price);

    order.parent_id = parent.order_id;
    order.trigger_price = Some(trigger_price); // When trigger price is penetrated
    order.adjusted_order_type = "STP LMT".to_owned(); // The parent order will be turned into a STP LMT order
    order.adjusted_stop_price = Some(adjusted_stop_price); // With the given STP price
    order.adjusted_stop_limit_price = Some(adjusted_stop_limit_price); // And the given limit price

    order
}

/// An attached order that turns the parent order (a conventional STP order) into a
/// TRAIL order in the opposite direction when the trigger is hit.
pub fn attach_adjustable_to_trail(
    parent: &Order,
    attached_order_stop_price: f64,
    trigger_price: f64,
    adjusted_stop_price: f64,
    adjusted_trail_amount: f64,
    trail_unit: i32,
) -> Order {
    // Attached order is a conventional STP order
    let mut order = stop(parent.action.reverse(), parent.total_quantity, attached_order_stop_price);

    order.parent_id = parent.order_id;
    order.trigger_price = Some(trigger_price); // When trigger price is penetrated
    order.adjusted_order_type = "TRAIL".to_owned(); // The parent order will be turned into a TRAIL order
    order.adjusted_stop_price = Some(adjusted_stop_price); // With a stop price of ...
    order.adjustable_trailing_unit = trail_unit; // trailing by and amount (0) or a percent (100) ...
    order.adjusted_trailing_amount = Some(adjusted_trail_amount); // of ...

    order
}

pub fn what_if_limit_order(action: Action, quantity: f64, limit_price: f64) -> Order {
    let mut order = limit_order(action, quantity, limit_price);
    order.what_if = true;

    order
}

// https://github.com/InteractiveBrokers/tws-api/blob/07e54ceecda2c9cbd6ffb5f524894f0c837a9ecb/source/csharpclient/client/ContractCondition.cs
// pub fn price_condition(contract_id: i32, exchange: &str, price: f64, is_more: bool, is_conjunction: bool) -> PriceCondition
// {
//     //! [price_condition]
//     //Conditions have to be created via the OrderCondition.Create
//     PriceCondition priceCondition = (PriceCondition)OrderCondition.Create(OrderConditionType.Price);
//     //When this contract...
//     priceCondition.ConId = conId;
//     //traded on this exchange
//     priceCondition.Exchange = exchange;
//     //has a price above/below
//     priceCondition.IsMore = isMore;
//     //this quantity
//     priceCondition.Price = price;
//     //AND | OR next condition (will be ignored if no more conditions are added)
//     priceCondition.IsConjunctionConnection = isConjunction;
//     //! [price_condition]
//     return priceCondition;
// }

//     public static ExecutionCondition ExecutionCondition(string symbol, string secType, string exchange, bool isConjunction)
//     {
//         //! [execution_condition]
//         ExecutionCondition execCondition = (ExecutionCondition)OrderCondition.Create(OrderConditionType.Execution);
//         //When an execution on symbol
//         execCondition.Symbol = symbol;
//         //at exchange
//         execCondition.Exchange = exchange;
//         //for this secType
//         execCondition.SecType = secType;
//         //AND | OR next condition (will be ignored if no more conditions are added)
//         execCondition.IsConjunctionConnection = isConjunction;
//         //! [execution_condition]
//         return execCondition;
//     }

//     public static MarginCondition MarginCondition(int percent, bool isMore, bool isConjunction)
//     {
//         //! [margin_condition]
//         MarginCondition marginCondition = (MarginCondition)OrderCondition.Create(OrderConditionType.Margin);
//         //If margin is above/below
//         marginCondition.IsMore = isMore;
//         //given percent
//         marginCondition.Percent = percent;
//         //AND | OR next condition (will be ignored if no more conditions are added)
//         marginCondition.IsConjunctionConnection = isConjunction;
//         //! [margin_condition]
//         return marginCondition;
//     }

//     public static PercentChangeCondition PercentageChangeCondition(double pctChange, int conId, string exchange, bool isMore, bool isConjunction)
//     {
//         //! [percentage_condition]
//         PercentChangeCondition pctChangeCondition = (PercentChangeCondition)OrderCondition.Create(OrderConditionType.PercentCange);
//         //If there is a price percent change measured against last close price above or below...
//         pctChangeCondition.IsMore = isMore;
//         //this amount...
//         pctChangeCondition.ChangePercent = pctChange;
//         //on this contract
//         pctChangeCondition.ConId = conId;
//         //when traded on this exchange...
//         pctChangeCondition.Exchange = exchange;
//         //AND | OR next condition (will be ignored if no more conditions are added)
//         pctChangeCondition.IsConjunctionConnection = isConjunction;
//         //! [percentage_condition]
//         return pctChangeCondition;
//     }

//     public static TimeCondition TimeCondition(string time, bool isMore, bool isConjunction)
//     {
//         //! [time_condition]
//         TimeCondition timeCondition = (TimeCondition)OrderCondition.Create(OrderConditionType.Time);
//         //Before or after...
//         timeCondition.IsMore = isMore;
//         //this time..
//         timeCondition.Time = time;
//         //AND | OR next condition (will be ignored if no more conditions are added)
//         timeCondition.IsConjunctionConnection = isConjunction;
//         //! [time_condition]
//         return timeCondition;
//     }

//     public static VolumeCondition VolumeCondition(int conId, string exchange, bool isMore, int volume, bool isConjunction)
//     {
//         //! [volume_condition]
//         VolumeCondition volCond = (VolumeCondition)OrderCondition.Create(OrderConditionType.Volume);
//         //Whenever contract...
//         volCond.ConId = conId;
//         //When traded at
//         volCond.Exchange = exchange;
//         //reaches a volume higher/lower
//         volCond.IsMore = isMore;
//         //than this...
//         volCond.Volume = volume;
//         //AND | OR next condition (will be ignored if no more conditions are added)
//         volCond.IsConjunctionConnection = isConjunction;
//         //! [volume_condition]
//         return volCond;

//     }

pub fn limit_ibkrats(action: Action, quantity: f64, limit_price: f64) -> Order {
    Order {
        action,
        order_type: "LMT".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        not_held: true,
        ..Order::default()
    }
}

pub fn limit_order_with_manual_order_time(action: Action, quantity: f64, limit_price: f64, manual_order_time: &str) -> Order {
    let mut order = limit_order(action, quantity, limit_price);
    order.manual_order_time = manual_order_time.to_owned();

    order
}

pub fn peg_best_up_to_mid_order(
    action: Action,
    quantity: f64,
    limit_price: f64,
    min_trade_qty: i32,
    min_compete_size: i32,
    mid_offset_at_whole: f64,
    mid_offset_at_half: f64,
) -> Order {
    Order {
        action,
        order_type: "PEG BEST".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        not_held: true,
        min_trade_qty: Some(min_trade_qty),
        min_complete_size: Some(min_compete_size),
        compete_against_best_offset: super::COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID,
        mid_offset_at_whole: Some(mid_offset_at_whole),
        mid_offset_at_half: Some(mid_offset_at_half),
        ..Order::default()
    }
}

pub fn peg_best_order(
    action: Action,
    quantity: f64,
    limit_price: f64,
    min_trade_qty: i32,
    min_compete_size: i32,
    compete_against_best_offset: f64,
) -> Order {
    Order {
        action,
        order_type: "PEG BEST".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        not_held: true,
        min_trade_qty: Some(min_trade_qty),
        min_complete_size: Some(min_compete_size),
        compete_against_best_offset: Some(compete_against_best_offset),
        ..Order::default()
    }
}

pub fn peg_mid_order(
    action: Action,
    quantity: f64,
    limit_price: f64,
    min_trade_qty: i32,
    mid_offset_at_whole: f64,
    mid_offset_at_half: f64,
) -> Order {
    Order {
        action,
        order_type: "PEG MID".to_owned(),
        total_quantity: quantity,
        limit_price: Some(limit_price),
        not_held: true,
        min_trade_qty: Some(min_trade_qty),
        mid_offset_at_whole: Some(mid_offset_at_whole),
        mid_offset_at_half: Some(mid_offset_at_half),
        ..Order::default()
    }
}
