use super::{Action, Order};

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
pub fn pegged_to_stock(
    action: Action,
    quantity: f64,
    delta: f64,
    stock_reference_price: f64,
    starting_price: f64,
) -> Order {
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
pub fn relative_pegged_to_primary(
    action: Action,
    quantity: f64,
    price_cap: f64,
    offset_amount: f64,
) -> Order {
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
pub fn auction_pegged_to_stock(
    action: Action,
    quantity: f64,
    starting_price: f64,
    delta: f64,
) -> Order {
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
pub fn limit_if_touched(
    action: Action,
    quantity: f64,
    limit_price: f64,
    trigger_price: f64,
) -> Order {
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

//     /// <summary>
//     /// Products:CFD, FUT, FOP, OPT, STK, WAR
//     /// A Market-to-Limit (MTL) order is submitted as a market order to execute at the current best market price. If the order is only
//     /// partially filled, the remainder of the order is canceled and re-submitted as a limit order with the limit price equal to the price
//     /// at which the filled portion of the order executed.
//     /// </summary>
//     public static Order MarketToLimit(string action, decimal quantity)
//     {
//         // ! [markettolimit]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "MTL";
//         order.TotalQuantity = quantity;
//         // ! [markettolimit]
//         return order;
//     }

//     /// <summary>
//     /// This order type is useful for futures traders using Globex. A Market with Protection order is a market order that will be cancelled and
//     /// resubmitted as a limit order if the entire order does not immediately execute at the market price. The limit price is set by Globex to be
//     /// close to the current market price, slightly higher for a sell order and lower for a buy order.
//     /// Products: FUT, FOP
//     /// </summary>
//     public static Order MarketWithProtection(string action, decimal quantity)
//     {
//         // ! [marketwithprotection]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "MKT PRT";
//         order.TotalQuantity = quantity;
//         // ! [marketwithprotection]
//         return order;
//     }

//     /// <summary>
//     /// A Stop order is an instruction to submit a buy or sell market order if and when the user-specified stop trigger price is attained or
//     /// penetrated. A Stop order is not guaranteed a specific execution price and may execute significantly away from its stop price. A Sell
//     /// Stop order is always placed below the current market price and is typically used to limit a loss or protect a profit on a long stock
//     /// position. A Buy Stop order is always placed above the current market price. It is typically used to limit a loss or help protect a
//     /// profit on a short sale.
//     /// Products: CFD, BAG, CASH, FUT, FOP, OPT, STK, WAR
//     /// </summary>
//     public static Order Stop(string action, decimal quantity, double stopPrice)
//     {
//         // ! [stop]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "STP";
//         order.AuxPrice = stopPrice;
//         order.TotalQuantity = quantity;
//         // ! [stop]
//         return order;
//     }

//     /// <summary>
//     /// A Stop-Limit order is an instruction to submit a buy or sell limit order when the user-specified stop trigger price is attained or
//     /// penetrated. The order has two basic components: the stop price and the limit price. When a trade has occurred at or through the stop
//     /// price, the order becomes executable and enters the market as a limit order, which is an order to buy or sell at a specified price or better.
//     /// Products: CFD, CASH, FUT, FOP, OPT, STK, WAR
//     /// </summary>
//     public static Order StopLimit(string action, decimal quantity, double limitPrice, double stopPrice)
//     {
//         // ! [stoplimit]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "STP LMT";
//         order.TotalQuantity = quantity;
//         order.LmtPrice = limitPrice;
//         order.AuxPrice = stopPrice;
//         // ! [stoplimit]
//         return order;
//     }

//     /// <summary>
//     /// A Stop with Protection order combines the functionality of a stop limit order with a market with protection order. The order is set
//     /// to trigger at a specified stop price. When the stop price is penetrated, the order is triggered as a market with protection order,
//     /// which means that it will fill within a specified protected price range equal to the trigger price +/- the exchange-defined protection
//     /// point range. Any portion of the order that does not fill within this protected range is submitted as a limit order at the exchange-defined
//     /// trigger price +/- the protection points.
//     /// Products: FUT
//     /// </summary>
//     public static Order StopWithProtection(string action, decimal quantity, double stopPrice)
//     {
//         // ! [stopwithprotection]
//         Order order = new Order();
//         order.TotalQuantity = quantity;
//         order.Action = action;
//         order.OrderType = "STP PRT";
//         order.AuxPrice = stopPrice;
//         // ! [stopwithprotection]
//         return order;
//     }

//     /// <summary>
//     /// A sell trailing stop order sets the stop price at a fixed amount below the market price with an attached "trailing" amount. As the
//     /// market price rises, the stop price rises by the trail amount, but if the stock price falls, the stop loss price doesn't change,
//     /// and a market order is submitted when the stop price is hit. This technique is designed to allow an investor to specify a limit on the
//     /// maximum possible loss, without setting a limit on the maximum possible gain. "Buy" trailing stop orders are the mirror image of sell
//     /// trailing stop orders, and are most appropriate for use in falling markets.
//     /// Products: CFD, CASH, FOP, FUT, OPT, STK, WAR
//     /// </summary>
//     public static Order TrailingStop(string action, decimal quantity, double trailingPercent, double trailStopPrice)
//     {
//         // ! [trailingstop]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "TRAIL";
//         order.TotalQuantity = quantity;
//         order.TrailingPercent = trailingPercent;
//         order.TrailStopPrice = trailStopPrice;
//         // ! [trailingstop]
//         return order;
//     }

//     /// <summary>
//     /// A trailing stop limit order is designed to allow an investor to specify a limit on the maximum possible loss, without setting a limit
//     /// on the maximum possible gain. A SELL trailing stop limit moves with the market price, and continually recalculates the stop trigger
//     /// price at a fixed amount below the market price, based on the user-defined "trailing" amount. The limit order price is also continually
//     /// recalculated based on the limit offset. As the market price rises, both the stop price and the limit price rise by the trail amount and
//     /// limit offset respectively, but if the stock price falls, the stop price remains unchanged, and when the stop price is hit a limit order
//     /// is submitted at the last calculated limit price. A "Buy" trailing stop limit order is the mirror image of a sell trailing stop limit,
//     /// and is generally used in falling markets.
//     /// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
//     /// </summary>
//     public static Order TrailingStopLimit(string action, decimal quantity, double lmtPriceOffset, double trailingAmount, double trailStopPrice)
//     {
//         // ! [trailingstoplimit]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "TRAIL LIMIT";
//         order.TotalQuantity = quantity;
//         order.TrailStopPrice = trailStopPrice;
//         order.LmtPriceOffset = lmtPriceOffset;
//         order.AuxPrice = trailingAmount;
//         // ! [trailingstoplimit]
//         return order;
//     }

//     /// <summary>
//     /// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
//     /// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
//     /// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
//     /// best execution.
//     /// Products: OPT, STK, FUT
//     /// </summary>
//     public static Order ComboLimitOrder(string action, decimal quantity, double limitPrice, bool nonGuaranteed)
//     {
//         // ! [combolimit]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "LMT";
//         order.TotalQuantity = quantity;
//         order.LmtPrice = limitPrice;
//         if (nonGuaranteed)
//         {
//             order.SmartComboRoutingParams = new List<TagValue>();
//             order.SmartComboRoutingParams.Add(new TagValue("NonGuaranteed", "1"));
//         }
//         // ! [combolimit]
//         return order;
//     }

//     /// <summary>
//     /// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
//     /// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
//     /// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
//     /// best execution.
//     /// Products: OPT, STK, FUT
//     /// </summary>
//     public static Order ComboMarketOrder(string action, decimal quantity, bool nonGuaranteed)
//     {
//         // ! [combomarket]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "MKT";
//         order.TotalQuantity = quantity;
//         if (nonGuaranteed)
//         {
//             order.SmartComboRoutingParams = new List<TagValue>();
//             order.SmartComboRoutingParams.Add(new TagValue("NonGuaranteed", "1"));
//         }
//         // ! [combomarket]
//         return order;
//     }

//     /// <summary>
//     /// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
//     /// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
//     /// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
//     /// best execution.
//     /// Products: OPT, STK, FUT
//     /// </summary>
//     public static Order LimitOrderForComboWithLegPrices(string action, decimal quantity, double[] legPrices, bool nonGuaranteed)
//     {
//         // ! [limitordercombolegprices]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "LMT";
//         order.TotalQuantity = quantity;
//         order.OrderComboLegs = new List<OrderComboLeg>();
//         foreach(double price in legPrices)
//         {
//             OrderComboLeg comboLeg = new OrderComboLeg();
//             comboLeg.Price = 5.0;
//             order.OrderComboLegs.Add(comboLeg);
//         }
//         if (nonGuaranteed)
//         {
//             order.SmartComboRoutingParams = new List<TagValue>();
//             order.SmartComboRoutingParams.Add(new TagValue("NonGuaranteed", "1"));
//         }
//         // ! [limitordercombolegprices]
//         return order;
//     }

//     /// <summary>
//     /// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
//     /// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
//     /// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
//     /// best execution.
//     /// Products: OPT, STK, FUT
//     /// </summary>
//     public static Order RelativeLimitCombo(string action, decimal quantity, double limitPrice, bool nonGuaranteed)
//     {
//         // ! [relativelimitcombo]
//         Order order = new Order();
//         order.Action = action;
//         order.TotalQuantity = quantity;
//         order.OrderType = "REL + LMT";
//         order.LmtPrice = limitPrice;
//         if (nonGuaranteed)
//         {
//             order.SmartComboRoutingParams = new List<TagValue>();
//             order.SmartComboRoutingParams.Add(new TagValue("NonGuaranteed", "1"));
//         }
//         // ! [relativelimitcombo]
//         return order;
//     }

//     /// <summary>
//     /// Create combination orders that include options, stock and futures legs (stock legs can be included if the order is routed
//     /// through SmartRouting). Although a combination/spread order is constructed of separate legs, it is executed as a single transaction
//     /// if it is routed directly to an exchange. For combination orders that are SmartRouted, each leg may be executed separately to ensure
//     /// best execution.
//     /// Products: OPT, STK, FUT
//     /// </summary>
//     public static Order RelativeMarketCombo(string action, decimal quantity, bool nonGuaranteed)
//     {
//         // ! [relativemarketcombo]
//         Order order = new Order();
//         order.Action = action;
//         order.TotalQuantity = quantity;
//         order.OrderType = "REL + MKT";
//         if (nonGuaranteed)
//         {
//             order.SmartComboRoutingParams = new List<TagValue>();
//             order.SmartComboRoutingParams.Add(new TagValue("NonGuaranteed", "1"));
//         }
//         // ! [relativemarketcombo]
//         return order;
//     }

//     /// <summary>
//     /// One-Cancels All (OCA) order type allows an investor to place multiple and possibly unrelated orders assigned to a group. The aim is
//     /// to complete just one of the orders, which in turn will cause TWS to cancel the remaining orders. The investor may submit several
//     /// orders aimed at taking advantage of the most desirable price within the group. Completion of one piece of the group order causes
//     /// cancellation of the remaining group orders while partial completion causes the group to rebalance. An investor might desire to sell
//     /// 1000 shares of only ONE of three positions held above prevailing market prices. The OCA order group allows the investor to enter prices
//     /// at specified target levels and if one is completed, the other two will automatically cancel. Alternatively, an investor may wish to take
//     /// a LONG position in eMini S&P stock index futures in a falling market or else SELL US treasury futures at a more favorable price.
//     /// Grouping the two orders using an OCA order type offers the investor two chance to enter a similar position, while only running the risk
//     /// of taking on a single position.
//     /// Products: BOND, CASH, FUT, FOP, STK, OPT, WAR
//     /// </summary>
//    // ! [oca]
//     public static List<Order> OneCancelsAll(string ocaGroup, List<Order> ocaOrders, int ocaType)
//     {
//         foreach (Order o in ocaOrders)
//         {
//             o.OcaGroup = ocaGroup;
//             o.OcaType = ocaType;
//         }
//         return ocaOrders;
//     }
//     // ! [oca]

//     /// <summary>
//     /// Specific to US options, investors are able to create and enter Volatility-type orders for options and combinations rather than price orders.
//     /// Option traders may wish to trade and position for movements in the price of the option determined by its implied volatility. Because
//     /// implied volatility is a key determinant of the premium on an option, traders position in specific contract months in an effort to take
//     /// advantage of perceived changes in implied volatility arising before, during or after earnings or when company specific or broad market
//     /// volatility is predicted to change. In order to create a Volatility order, clients must first create a Volatility Trader page from the
//     /// Trading Tools menu and as they enter option contracts, premiums will display in percentage terms rather than premium. The buy/sell process
//     /// is the same as for regular orders priced in premium terms except that the client can limit the volatility level they are willing to pay or
//     /// receive.
//     /// Products: FOP, OPT
//     /// </summary>
//     public static Order Volatility(string action, decimal quantity, double volatilityPercent, int volatilityType)
//     {
//         // ! [volatility]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "VOL";
//         order.TotalQuantity = quantity;
//         order.Volatility = volatilityPercent;//Expressed in percentage (40%)
//         order.VolatilityType = volatilityType;// 1=daily, 2=annual
//         // ! [volatility]
//         return order;
//     }

//     //! [fhedge]
//     public static Order MarketFHedge(int parentOrderId, string action)
//     {
//         //FX Hedge orders can only have a quantity of 0
//         Order order = MarketOrder(action, 0);
//         order.ParentId = parentOrderId;
//         order.HedgeType = "F";
//         return order;
//     }
//     //! [fhedge]

//     public static Order PeggedToBenchmark(string action, decimal quantity, double startingPrice, bool peggedChangeAmountDecrease, double peggedChangeAmount,
//          double referenceChangeAmount, int referenceConId, string referenceExchange, double stockReferencePrice,
//         double referenceContractLowerRange, double referenceContractUpperRange)
//     {
//         //! [pegged_benchmark]
//         Order order = new Order();
//         order.OrderType = "PEG BENCH";
//         //BUY or SELL
//         order.Action = action;
//         order.TotalQuantity = quantity;
//         //Beginning with price...
//         order.StartingPrice = startingPrice;
//         //increase/decrease price..
//         order.IsPeggedChangeAmountDecrease = peggedChangeAmountDecrease;
//         //by... (and likewise for price moving in opposite direction)
//         order.PeggedChangeAmount = peggedChangeAmount;
//         //whenever there is a price change of...
//         order.ReferenceChangeAmount = referenceChangeAmount;
//         //in the reference contract...
//         order.ReferenceContractId = referenceConId;
//         //being traded at...
//         order.ReferenceExchange = referenceExchange;
//         //starting reference price is...
//         order.StockRefPrice = stockReferencePrice;
//         //Keep order active as long as reference contract trades between...
//         order.StockRangeLower = referenceContractLowerRange;
//         //and...
//         order.StockRangeUpper = referenceContractUpperRange;
//         //! [pegged_benchmark]
//         return order;
//     }

//     public static Order AttachAdjustableToStop(Order parent, double attachedOrderStopPrice, double triggerPrice, double adjustStopPrice)
//     {
//         //! [adjustable_stop]
//         //Attached order is a conventional STP order in opposite direction
//         Order order = Stop(parent.Action.Equals("BUY") ? "SELL" : "BUY", parent.TotalQuantity, attachedOrderStopPrice);
//         order.ParentId = parent.OrderId;
//         //When trigger price is penetrated
//         order.TriggerPrice = triggerPrice;
//         //The parent order will be turned into a STP order
//         order.AdjustedOrderType = "STP";
//         //With the given STP price
//         order.AdjustedStopPrice = adjustStopPrice;
//         //! [adjustable_stop]
//         return order;
//     }

//     public static Order AttachAdjustableToStopLimit(Order parent, double attachedOrderStopPrice, double triggerPrice,
//         double adjustedStopPrice, double adjustedStopLimitPrice)
//     {
//         //! [adjustable_stop_limit]
//         //Attached order is a conventional STP order
//         Order order = Stop(parent.Action.Equals("BUY") ? "SELL" : "BUY", parent.TotalQuantity, attachedOrderStopPrice);
//         order.ParentId = parent.OrderId;
//         //When trigger price is penetrated
//         order.TriggerPrice = triggerPrice;
//         //The parent order will be turned into a STP LMT order
//         order.AdjustedOrderType = "STP LMT";
//         //With the given stop price
//         order.AdjustedStopPrice = adjustedStopPrice;
//         //And the given limit price
//         order.AdjustedStopLimitPrice = adjustedStopLimitPrice;
//         //! [adjustable_stop_limit]
//         return order;
//     }

// 	public static Order AttachAdjustableToTrail(Order parent, double attachedOrderStopPrice, double triggerPrice, double adjustedStopPrice,
//         double adjustedTrailAmount, int trailUnit)
//     {
//         //! [adjustable_trail]
//         //Attached order is a conventional STP order
//         Order order = Stop(parent.Action.Equals("BUY") ? "SELL" : "BUY", parent.TotalQuantity, attachedOrderStopPrice);
//         order.ParentId = parent.OrderId;
//         //When trigger price is penetrated
//         order.TriggerPrice = triggerPrice;
//         //The parent order will be turned into a TRAIL order
//         order.AdjustedOrderType = "TRAIL";
//         //With a stop price of...
//         order.AdjustedStopPrice = adjustedStopPrice;
//         //traling by and amount (0) or a percent (100)...
//         order.AdjustableTrailingUnit = trailUnit;
//         //of...
//         order.AdjustedTrailingAmount = adjustedTrailAmount;
//         //! [adjustable_trail]
//         return order;
//     }

//     public static Order WhatIfLimitOrder(string action, decimal quantity, double limitPrice)
//     {
//         // ! [whatiflimitorder]
//         Order order = LimitOrder(action, quantity, limitPrice);
//         order.WhatIf = true;
//         // ! [whatiflimitorder]
//         return order;
//     }

//     public static PriceCondition PriceCondition(int conId, string exchange, double price, bool isMore, bool isConjunction)
//     {
//         //! [price_condition]
//         //Conditions have to be created via the OrderCondition.Create
//         PriceCondition priceCondition = (PriceCondition)OrderCondition.Create(OrderConditionType.Price);
//         //When this contract...
//         priceCondition.ConId = conId;
//         //traded on this exchange
//         priceCondition.Exchange = exchange;
//         //has a price above/below
//         priceCondition.IsMore = isMore;
//         //this quantity
//         priceCondition.Price = price;
//         //AND | OR next condition (will be ignored if no more conditions are added)
//         priceCondition.IsConjunctionConnection = isConjunction;
//         //! [price_condition]
//         return priceCondition;
//     }

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

// 	public static Order LimitIBKRATS(string action, decimal quantity, double limitPrice)
//     {
//         // ! [limit_ibkrats]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "LMT";
// 		order.LmtPrice = limitPrice;
//         order.TotalQuantity = quantity;
// 		order.NotHeld = true;
//         // ! [limit_ibkrats]
//         return order;
//     }

//     public static Order LimitOrderWithManualOrderTime(string action, decimal quantity, double limitPrice, string manualOrderTime)
//     {
//         // ! [limit_order_with_manual_order_time]
//         Order order = OrderSamples.LimitOrder(action, quantity, limitPrice);
//         order.ManualOrderTime = manualOrderTime;
//         // ! [limit_order_with_manual_order_time]
//         return order;
//     }

//     public static Order PegBestUpToMidOrder(string action, decimal quantity, double limitPrice, int minTradeQty, int minCompeteSize, double midOffsetAtWhole, double midOffsetAtHalf)
//     {
//         // ! [peg_best_up_to_mid_order]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "PEG BEST";
//         order.LmtPrice = limitPrice;
//         order.TotalQuantity = quantity;
//         order.NotHeld = true;
//         order.MinTradeQty = minTradeQty;
//         order.MinCompeteSize = minCompeteSize;
//         order.CompeteAgainstBestOffset = Order.COMPETE_AGAINST_BEST_OFFSET_UP_TO_MID;
//         order.MidOffsetAtWhole = midOffsetAtWhole;
//         order.MidOffsetAtHalf = midOffsetAtHalf;
//         // ! [peg_best_up_to_mid_order]
//         return order;
//     }

//     public static Order PegBestOrder(string action, decimal quantity, double limitPrice, int minTradeQty, int minCompeteSize, double competeAgainstBestOffset)
//     {
//         // ! [peg_best_order]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "PEG BEST";
//         order.LmtPrice = limitPrice;
//         order.TotalQuantity = quantity;
//         order.NotHeld = true;
//         order.MinTradeQty = minTradeQty;
//         order.MinCompeteSize = minCompeteSize;
//         order.CompeteAgainstBestOffset = competeAgainstBestOffset;
//         // ! [peg_best_order]
//         return order;
//     }

//     public static Order PegMidOrder(string action, decimal quantity, double limitPrice, int minTradeQty, double midOffsetAtWhole, double midOffsetAtHalf)
//     {
//         // ! [peg_mid_order]
//         Order order = new Order();
//         order.Action = action;
//         order.OrderType = "PEG MID";
//         order.LmtPrice = limitPrice;
//         order.TotalQuantity = quantity;
//         order.NotHeld = true;
//         order.MinTradeQty = minTradeQty;
//         order.MidOffsetAtWhole = midOffsetAtWhole;
//         order.MidOffsetAtHalf = midOffsetAtHalf;
//         // ! [peg_mid_order]
//         return order;
//     }
