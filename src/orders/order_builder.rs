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

        // / A Market if Touched (MIT) is an order to buy (or sell) a contract below (or above) the market. Its purpose is to take advantage 
        // / of sudden or unexpected changes in share or other prices and provides investors with a trigger price to set an order in motion. 
        // / Investors may be waiting for excessive strength (or weakness) to cease, which might be represented by a specific price point. 
        // / MIT orders can be used to determine whether or not to enter the market once a specific price level has been achieved. This order 
        // / is held in the system until the trigger price is touched, and is then submitted as a market order. An MIT order is similar to a 
        // / stop order, except that an MIT sell order is placed above the current market price, and a stop sell order is placed below
        // / Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR

    //     public static Order MarketIfTouched(string action, decimal quantity, double price)
    //     {
    //         //! [market_if_touched]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "MIT";
    //         order.TotalQuantity = quantity;
    //         order.AuxPrice = price;
    //         //! [market_if_touched]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Market-on-Close (MOC) order is a market order that is submitted to execute as close to the closing price as possible.
    //     /// Products: CFD, FUT, STK, WAR
    //     /// </summary>
    //     public static Order MarketOnClose(string action, decimal quantity)
    //     {
    //         //! [market_on_close]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "MOC";
    //         order.TotalQuantity = quantity;
    //         //! [market_on_close]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Market-on-Open (MOO) order combines a market order with the OPG time in force to create an order that is automatically
    //     /// submitted at the market's open and fills at the market price.
    //     /// Products: CFD, STK, OPT, WAR
    //     /// </summary>
    //     public static Order MarketOnOpen(string action, decimal quantity)
    //     {
    //         //! [market_on_open]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "MKT";
    //         order.TotalQuantity = quantity;
    //         order.Tif = "OPG";
    //         //! [market_on_open]
    //         return order;
    //     }

    //     /// <summary>
    //     /// ISE Midpoint Match (MPM) orders always execute at the midpoint of the NBBO. You can submit market and limit orders direct-routed 
    //     /// to ISE for MPM execution. Market orders execute at the midpoint whenever an eligible contra-order is available. Limit orders 
    //     /// execute only when the midpoint price is better than the limit price. Standard MPM orders are completely anonymous.
    //     /// Products: STK
    //     /// </summary>
    //     public static Order MidpointMatch(string action, decimal quantity)
    //     {
    //         //! [midpoint_match]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "MKT";
    //         order.TotalQuantity = quantity;
    //         //! [midpoint_match]
    //         return order;
    //     }

	// 	/// <summary>
	// 	// A Midprice order is designed to split the difference between the bid and ask prices, and fill at the current midpoint of 
	// 	// the NBBO or better. Set an optional price cap to define the highest price (for a buy order) or the lowest price (for a sell 
	// 	// order) you are willing to accept. Requires TWS 975+. Smart-routing to US stocks only.
    //     /// </summary>
	// 	public static Order Midprice(string action, decimal quantity, double priceCap)
    //     {
    //         //! [midprice]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "MIDPRICE";
    //         order.TotalQuantity = quantity;
	// 		order.LmtPrice = priceCap;
    //         //! [midprice]
    //         return order;
    //     }
		
    //     /// <summary>
    //     /// A pegged-to-market order is designed to maintain a purchase price relative to the national best offer (NBO) or a sale price 
    //     /// relative to the national best bid (NBB). Depending on the width of the quote, this order may be passive or aggressive. 
    //     /// The trader creates the order by entering a limit price which defines the worst limit price that they are willing to accept. 
    //     /// Next, the trader enters an offset amount which computes the active limit price as follows:
    //     ///     Sell order price = Bid price + offset amount
    //     ///     Buy order price = Ask price - offset amount
    //     /// Products: STK
    //     /// </summary>
    //     public static Order PeggedToMarket(string action, decimal quantity, double marketOffset)
    //     {
    //         //! [pegged_market]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "PEG MKT";
    //         order.TotalQuantity = 100;
    //         order.AuxPrice = marketOffset;//Offset price
    //         //! [pegged_market]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Pegged to Stock order continually adjusts the option order price by the product of a signed user-define delta and the change of 
    //     /// the option's underlying stock price. The delta is entered as an absolute and assumed to be positive for calls and negative for puts. 
    //     /// A buy or sell call order price is determined by adding the delta times a change in an underlying stock price to a specified starting 
    //     /// price for the call. To determine the change in price, the stock reference price is subtracted from the current NBBO midpoint. 
    //     /// The Stock Reference Price can be defined by the user, or defaults to the NBBO midpoint at the time of the order if no reference price 
    //     /// is entered. You may also enter a high/low stock price range which cancels the order when reached. The delta times the change in stock 
    //     /// price will be rounded to the nearest penny in favor of the order.
    //     /// Products: OPT
    //     /// </summary>
    //     public static Order PeggedToStock(string action, decimal quantity, double delta, double stockReferencePrice, double startingPrice)
    //     {
    //         //! [pegged_stock]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "PEG STK";
    //         order.TotalQuantity = quantity;
    //         order.Delta = delta;
    //         order.StockRefPrice = stockReferencePrice;
    //         order.StartingPrice = startingPrice;
    //         //! [pegged_stock]
    //         return order;
    //     }

    //     /// <summary>
    //     /// Relative (a.k.a. Pegged-to-Primary) orders provide a means for traders to seek a more aggressive price than the National Best Bid 
    //     /// and Offer (NBBO). By acting as liquidity providers, and placing more aggressive bids and offers than the current best bids and offers, 
    //     /// traders increase their odds of filling their order. Quotes are automatically adjusted as the markets move, to remain aggressive. 
    //     /// For a buy order, your bid is pegged to the NBB by a more aggressive offset, and if the NBB moves up, your bid will also move up. 
    //     /// If the NBB moves down, there will be no adjustment because your bid will become even more aggressive and execute. For sales, your 
    //     /// offer is pegged to the NBO by a more aggressive offset, and if the NBO moves down, your offer will also move down. If the NBO moves up, 
    //     /// there will be no adjustment because your offer will become more aggressive and execute. In addition to the offset, you can define an 
    //     /// absolute cap, which works like a limit price, and will prevent your order from being executed above or below a specified level.
    //     /// Stocks, Options and Futures - not available on paper trading
    //     /// Products: CFD, STK, OPT, FUT
    //     /// </summary>
    //     public static Order RelativePeggedToPrimary(string action, decimal quantity, double priceCap, double offsetAmount)
    //     {
    //         //! [relative_pegged_primary]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "REL";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = priceCap;
    //         order.AuxPrice = offsetAmount;
    //         //! [relative_pegged_primary]
    //         return order;
    //     }

    //     /// <summary>
    //     /// Sweep-to-fill orders are useful when a trader values speed of execution over price. A sweep-to-fill order identifies the best price 
    //     /// and the exact quantity offered/available at that price, and transmits the corresponding portion of your order for immediate execution. 
    //     /// Simultaneously it identifies the next best price and quantity offered/available, and submits the matching quantity of your order for 
    //     /// immediate execution.
    //     /// Products: CFD, STK, WAR
    //     /// </summary>
    //     public static Order SweepToFill(string action, decimal quantity, double price)
    //     {
    //         //! [sweep_to_fill]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LMT";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = price;
    //         order.SweepToFill = true;
    //         //! [sweep_to_fill]
    //         return order;
    //     }

    //     /// <summary>
    //     /// For option orders routed to the Boston Options Exchange (BOX) you may elect to participate in the BOX's price improvement auction in 
    //     /// pennies. All BOX-directed price improvement orders are immediately sent from Interactive Brokers to the BOX order book, and when the 
    //     /// terms allow, IB will evaluate it for inclusion in a price improvement auction based on price and volume priority. In the auction, your 
    //     /// order will have priority over broker-dealer price improvement orders at the same price.
    //     /// An Auction Limit order at a specified price. Use of a limit order ensures that you will not receive an execution at a price less favorable 
    //     /// than the limit price. Enter limit orders in penny increments with your auction improvement amount computed as the difference between your 
    //     /// limit order price and the nearest listed increment.
    //     /// Products: OPT
    //     /// Supported Exchanges: BOX
    //     /// </summary>
    //     public static Order AuctionLimit(string action, decimal quantity, double price, int auctionStrategy)
    //     {
    //         //! [auction_limit]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LMT";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = price;
    //         order.AuctionStrategy = auctionStrategy;
    //         //! [auction_limit]
    //         return order;
    //     }

    //     /// <summary>
    //     /// For option orders routed to the Boston Options Exchange (BOX) you may elect to participate in the BOX's price improvement auction in pennies. 
    //     /// All BOX-directed price improvement orders are immediately sent from Interactive Brokers to the BOX order book, and when the terms allow, 
    //     /// IB will evaluate it for inclusion in a price improvement auction based on price and volume priority. In the auction, your order will have 
    //     /// priority over broker-dealer price improvement orders at the same price.
    //     /// An Auction Pegged to Stock order adjusts the order price by the product of a signed delta (which is entered as an absolute and assumed to be 
    //     /// positive for calls, negative for puts) and the change of the option's underlying stock price. A buy or sell call order price is determined 
    //     /// by adding the delta times a change in an underlying stock price change to a specified starting price for the call. To determine the change 
    //     /// in price, a stock reference price (NBBO midpoint at the time of the order is assumed if no reference price is entered) is subtracted from 
    //     /// the current NBBO midpoint. A stock range may also be entered that cancels an order when reached. The delta times the change in stock price 
    //     /// will be rounded to the nearest penny in favor of the order and will be used as your auction improvement amount.
    //     /// Products: OPT
    //     /// Supported Exchanges: BOX
    //     /// </summary>
    //     public static Order AuctionPeggedToStock(string action, decimal quantity, double startingPrice, double delta)
    //     {
    //         //! [auction_pegged_stock]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "PEG STK";
    //         order.TotalQuantity = quantity;
    //         order.Delta = delta;
    //         order.StartingPrice = startingPrice;
    //         //! [auction_pegged_stock]
    //         return order;
    //     }

    //     /// <summary>
    //     /// For option orders routed to the Boston Options Exchange (BOX) you may elect to participate in the BOX's price improvement auction in pennies. 
    //     /// All BOX-directed price improvement orders are immediately sent from Interactive Brokers to the BOX order book, and when the terms allow, 
    //     /// IB will evaluate it for inclusion in a price improvement auction based on price and volume priority. In the auction, your order will have 
    //     /// priority over broker-dealer price improvement orders at the same price.
    //     /// An Auction Relative order that adjusts the order price by the product of a signed delta (which is entered as an absolute and assumed to be 
    //     /// positive for calls, negative for puts) and the change of the option's underlying stock price. A buy or sell call order price is determined 
    //     /// by adding the delta times a change in an underlying stock price change to a specified starting price for the call. To determine the change 
    //     /// in price, a stock reference price (NBBO midpoint at the time of the order is assumed if no reference price is entered) is subtracted from 
    //     /// the current NBBO midpoint. A stock range may also be entered that cancels an order when reached. The delta times the change in stock price 
    //     /// will be rounded to the nearest penny in favor of the order and will be used as your auction improvement amount.
    //     /// Products: OPT
    //     /// Supported Exchanges: BOX
    //     /// </summary>
    //     public static Order AuctionRelative(string action, decimal quantity, double offset)
    //     {
    //         //! [auction_relative]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "REL";
    //         order.TotalQuantity = quantity;
    //         order.AuxPrice = offset;
    //         //! [auction_relative]
    //         return order;
    //     }

    //     /// <summary>
    //     /// The Block attribute is used for large volume option orders on ISE that consist of at least 50 contracts. To execute large-volume 
    //     /// orders over time without moving the market, use the Accumulate/Distribute algorithm.
    //     /// Products: OPT
    //     /// </summary>
    //     public static Order Block(string action, decimal quantity, double price)
    //     {
    //         // ! [block]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LMT";
    //         order.TotalQuantity = quantity;//Large volumes!
    //         order.LmtPrice = price;
    //         order.BlockOrder = true;
    //         // ! [block]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Box Top order executes as a market order at the current best price. If the order is only partially filled, the remainder is submitted as 
    //     /// a limit order with the limit price equal to the price at which the filled portion of the order executed.
    //     /// Products: OPT
    //     /// Supported Exchanges: BOX
    //     /// </summary>
    //     public static Order BoxTop(string action, decimal quantity)
    //     {
    //         // ! [boxtop]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "BOX TOP";
    //         order.TotalQuantity = quantity;
    //         // ! [boxtop]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Limit order is an order to buy or sell at a specified price or better. The Limit order ensures that if the order fills, 
    //     /// it will not fill at a price less favorable than your limit price, but it does not guarantee a fill.
    //     /// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
    //     /// </summary>
    //     public static Order LimitOrder(string action, decimal quantity, double limitPrice)
    //     {
    //         // ! [limitorder]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LMT";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = limitPrice;
    //         // ! [limitorder]
    //         return order;
    //     }

	// 	/// <summary>
	// 	/// Forex orders can be placed in demonination of second currency in pair using cashQty field
	// 	/// Requires TWS or IBG 963+
	// 	/// https://www.interactivebrokers.com/en/index.php?f=23876#963-02
	// 	/// <summary>

    //     public static Order LimitOrderWithCashQty(string action, double limitPrice, double cashQty)
    //     {
    //         // ! [limitorderwithcashqty]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LMT";
    //         order.LmtPrice = limitPrice;
    //         order.CashQty = cashQty;
    //         // ! [limitorderwithcashqty]
    //         return order;
    //     }


    //     /// <summary>
    //     /// A Limit if Touched is an order to buy (or sell) a contract at a specified price or better, below (or above) the market. This order is 
    //     /// held in the system until the trigger price is touched. An LIT order is similar to a stop limit order, except that an LIT sell order is 
    //     /// placed above the current market price, and a stop limit sell order is placed below.
    //     /// Products: BOND, CFD, CASH, FUT, FOP, OPT, STK, WAR
    //     /// </summary>
    //     public static Order LimitIfTouched(string action, decimal quantity, double limitPrice, double triggerPrice)
    //     {
    //         // ! [limitiftouched]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LIT";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = limitPrice;
    //         order.AuxPrice = triggerPrice;
    //         // ! [limitiftouched]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Limit-on-close (LOC) order will be submitted at the close and will execute if the closing price is at or better than the submitted 
    //     /// limit price.
    //     /// Products: CFD, FUT, STK, WAR
    //     /// </summary>
    //     public static Order LimitOnClose(string action, decimal quantity, double limitPrice)
    //     {
    //         // ! [limitonclose]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "LOC";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = limitPrice;
    //         // ! [limitonclose]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A Limit-on-Open (LOO) order combines a limit order with the OPG time in force to create an order that is submitted at the market's open, 
    //     /// and that will only execute at the specified limit price or better. Orders are filled in accordance with specific exchange rules.
    //     /// Products: CFD, STK, OPT, WAR
    //     /// </summary>
    //     public static Order LimitOnOpen(string action, decimal quantity, double limitPrice)
    //     {
    //         // ! [limitonopen]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.Tif = "OPG";
    //         order.OrderType = "LMT";
    //         order.TotalQuantity = quantity;
    //         order.LmtPrice = limitPrice;
    //         // ! [limitonopen]
    //         return order;
    //     }

    //     /// <summary>
    //     /// Passive Relative orders provide a means for traders to seek a less aggressive price than the National Best Bid and Offer (NBBO) while 
    //     /// keeping the order pegged to the best bid (for a buy) or ask (for a sell). The order price is automatically adjusted as the markets move 
    //     /// to keep the order less aggressive. For a buy order, your order price is pegged to the NBB by a less aggressive offset, and if the NBB 
    //     /// moves up, your bid will also move up. If the NBB moves down, there will be no adjustment because your bid will become aggressive and execute. 
    //     /// For a sell order, your price is pegged to the NBO by a less aggressive offset, and if the NBO moves down, your offer will also move down. 
    //     /// If the NBO moves up, there will be no adjustment because your offer will become aggressive and execute. In addition to the offset, you can 
    //     /// define an absolute cap, which works like a limit price, and will prevent your order from being executed above or below a specified level. 
    //     /// The Passive Relative order is similar to the Relative/Pegged-to-Primary order, except that the Passive relative subtracts the offset from 
    //     /// the bid and the Relative adds the offset to the bid.
    //     /// Products: STK, WAR
    //     /// </summary>
    //     public static Order PassiveRelative(string action, decimal quantity, double offset)
    //     {
    //         // ! [passive_relative]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "PASSV REL";
    //         order.TotalQuantity = quantity;
    //         order.AuxPrice = offset;
    //         // ! [passive_relative]
    //         return order;
    //     }

    //     /// <summary>
    //     /// A pegged-to-midpoint order provides a means for traders to seek a price at the midpoint of the National Best Bid and Offer (NBBO). 
    //     /// The price automatically adjusts to peg the midpoint as the markets move, to remain aggressive. For a buy order, your bid is pegged to 
    //     /// the NBBO midpoint and the order price adjusts automatically to continue to peg the midpoint if the market moves. The price only adjusts 
    //     /// to be more aggressive. If the market moves in the opposite direction, the order will execute.
    //     /// Products: STK
    //     /// </summary>
    //     public static Order PeggedToMidpoint(string action, decimal quantity, double offset, double limitPrice)
    //     {
    //         // ! [pegged_midpoint]
    //         Order order = new Order();
    //         order.Action = action;
    //         order.OrderType = "PEG MID";
    //         order.TotalQuantity = quantity;
    //         order.AuxPrice = offset;
	// 		order.LmtPrice = limitPrice;
    //         // ! [pegged_midpoint]
    //         return order;
    //     }

    //     /// <summary>
    //     /// Bracket orders are designed to help limit your loss and lock in a profit by "bracketing" an order with two opposite-side orders. 
    //     /// A BUY order is bracketed by a high-side sell limit order and a low-side sell stop order. A SELL order is bracketed by a high-side buy 
    //     /// stop order and a low side buy limit order.
    //     /// Products: CFD, BAG, FOP, CASH, FUT, OPT, STK, WAR
    //     /// </summary>
    //     //! [bracket]
    //     public static List<Order> BracketOrder(int parentOrderId, string action, decimal quantity, double limitPrice, 
    //         double takeProfitLimitPrice, double stopLossPrice)
    //     {
    //         //This will be our main or "parent" order
    //         Order parent = new Order();
    //         parent.OrderId = parentOrderId;
    //         parent.Action = action;
    //         parent.OrderType = "LMT";
    //         parent.TotalQuantity = quantity;
    //         parent.LmtPrice = limitPrice;
    //         //The parent and children orders will need this attribute set to false to prevent accidental executions.
    //         //The LAST CHILD will have it set to true, 
    //         parent.Transmit = false;

    //         Order takeProfit = new Order();
    //         takeProfit.OrderId = parent.OrderId + 1;
    //         takeProfit.Action = action.Equals("BUY") ? "SELL" : "BUY";
    //         takeProfit.OrderType = "LMT";
    //         takeProfit.TotalQuantity = quantity;
    //         takeProfit.LmtPrice = takeProfitLimitPrice;
    //         takeProfit.ParentId = parentOrderId;
    //         takeProfit.Transmit = false;

    //         Order stopLoss = new Order();
    //         stopLoss.OrderId = parent.OrderId + 2;
    //         stopLoss.Action = action.Equals("BUY") ? "SELL" : "BUY";
    //         stopLoss.OrderType = "STP";
    //         //Stop trigger price
    //         stopLoss.AuxPrice = stopLossPrice;
    //         stopLoss.TotalQuantity = quantity;
    //         stopLoss.ParentId = parentOrderId;
    //         //In this case, the low side order will be the last child being sent. Therefore, it needs to set this attribute to true 
    //         //to activate all its predecessors
    //         stopLoss.Transmit = true;

    //         List<Order> bracketOrder = new List<Order>();
    //         bracketOrder.Add(parent);
    //         bracketOrder.Add(takeProfit);
    //         bracketOrder.Add(stopLoss);
    //         return bracketOrder;
    //     }
    //     //! [bracket]

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
