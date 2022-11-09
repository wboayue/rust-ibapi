use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Debug, PartialEq, EnumString)]
/// The security's type
pub enum SecurityType {
    /// Stock (or ETF)
    STK,
    /// Option
    OPT,
    /// Future
    FUT,
    /// Index
    IND,
    /// Futures option
    FOP,
    /// Forex pair
    CASH,
    /// Combo
    BAG,
    ///  Warrant
    WAR,
    /// Bond
    BOND,
    /// Commodity
    CMDTY,
    /// News
    NEWS,
    /// Mutual fund
    FUND,
}

#[derive(Debug)]
/// Describes an instrument's definition
pub struct Contract {
    /// The unique IB contract identifier.
    pub contract_id: i32,
    /// The underlying's asset symbol.
    pub symbol: String,
    pub security_type: SecurityType,
    /// The contract's last trading day or contract month (for Options and Futures).
    /// Strings with format YYYYMM will be interpreted as the Contract Month whereas YYYYMMDD will be interpreted as Last Trading Day.
    pub last_trade_date_or_contract_month: String,
    /// The option's strike price.
    pub strike: f64,
    /// Either Put or Call (i.e. Options). Valid values are P, PUT, C, CALL.
    pub right: String,
    /// The instrument's multiplier (i.e. options, futures).
    pub multiplier: String,
    /// The destination exchange.
    pub exchange: String,
    /// The underlying's currency.
    pub currency: String,
    /// The contract's symbol within its primary exchange For options, this will be the OCC symbol.
    pub local_symbol: String,
    /// The contract's primary exchange.
    /// For smart routed contracts, used to define contract in case of ambiguity.
    /// Should be defined as native exchange of contract, e.g. ISLAND for MSFT For exchanges which contain a period in name, will only be part of exchange name prior to period, i.e. ENEXT for ENEXT.BE.
    pub primary_exchange: String,
    /// The trading class name for this contract. Available in TWS contract description window as well. For example, GBL Dec '13 future's trading class is "FGBL".
    pub trading_class: String,
    /// If set to true, contract details requests and historical data queries can be performed pertaining to expired futures contracts. Expired options or other instrument types are not available.
    pub include_expired: bool,
    /// Security's identifier when querying contract's details or placing orders ISIN - Example: Apple: US0378331005 CUSIP - Example: Apple: 037833100.
    pub security_id_type: String,
    /// Identifier of the security type.
    pub security_id: String,
    /// Description of the combo legs.
    pub combo_legs_description: String,
    pub combo_legs: Box<[ComboLeg]>,
    /// Delta and underlying price for Delta-Neutral combo orders. Underlying (STK or FUT), delta and underlying price goes into this attribute.
    pub delta_neutral_contract: DeltaNeutralContract,
}

#[derive(Debug)]
pub struct ComboLeg {
    // 	ContractId int    // The Contract's IB's unique id.
    // 	Ratio      int    // Select the relative number of contracts for the leg you are constructing. To help determine the ratio for a specific combination order, refer to the Interactive Analytics section of the User's Guide.
    // 	Action     string //The side (buy or sell) of the leg:
    // 	Exchange   string // The destination exchange to which the order will be routed.

    // 	// Specifies whether an order is an open or closing order. For instituational customers to determine if this order is to open or close a position. 0 - Same as the parent security. This is the only option for retail customers.
    // 	// 1 - Open. This value is only valid for institutional customers.
    // 	// 2 - Close. This value is only valid for institutional customers.
    // 	// 3 - Unknown.
    // 	OpenClose int

    // 	ShortSaleSlot      int    // For stock legs when doing short selling. Set to 1 = clearing broker, 2 = third party.
    // 	DesignatedLocation string // When ShortSaleSlot is 2, this field shall contain the designated location.
    // 	ExemptCode         int    // DOC_TODO.
}

#[derive(Debug)]
/// Delta and underlying price for Delta-Neutral combo orders. Underlying (STK or FUT), delta and underlying price goes into this attribute.
pub struct DeltaNeutralContract {
    // 	ContractId string  // The unique contract identifier specifying the security. Used for Delta-Neutral Combo contracts.
    // 	Delta      float64 // The underlying stock or future delta. Used for Delta-Neutral Combo contracts.
    // 	Price      float64 // The price of the underlying. Used for Delta-Neutral Combo contracts.
}

// ContractDetails struct {
// 	Contract       Contract // A fully-defined Contract object.
// 	MarketName     string   // The market name for this product.
// 	MinTick        float64  // The minimum allowed price variation. Note that many securities vary their minimum tick size according to their price. This value will only show the smallest of the different minimum tick sizes regardless of the product's price. Full information about the minimum increment price structure can be obtained with the reqMarketRule function or the IB Contract and Security Search site.
// 	PriceMagnifier int      // Allows execution and strike prices to be reported consistently with market data, historical data and the order price, i.e. Z on LIFFE is reported in Index points and not GBP. In TWS versions prior to 972, the price magnifier is used in defining future option strike prices (e.g. in the API the strike is specified in dollars, but in TWS it is specified in cents). In TWS versions 972 and higher, the price magnifier is not used in defining futures option strike prices so they are consistent in TWS and the API.
// 	OrderTypes     string   //Supported order types for this product.
// 	ValidExchanges string   // Valid exchange fields when placing an order for this contract.
// 	// The list of exchanges will is provided in the same order as the corresponding MarketRuleIds list.
// 	UnderContractId        int        // For derivatives, the contract ID (conID) of the underlying instrument.
// 	LongName               string     // Descriptive name of the product.
// 	ContractMonth          string     // Typically the contract month of the underlying for a Future contract.
// 	Industry               string     // The industry classification of the underlying/product. For example, Financial.
// 	Category               string     // The industry category of the underlying. For example, InvestmentSvc.
// 	Subcategory            string     // The industry subcategory of the underlying. For example, Brokerage.
// 	TimeZoneId             string     // The time zone for the trading hours of the product. For example, EST.
// 	TradingHours           string     // The trading hours of the product. This value will contain the trading hours of the current day as well as the next's. For example, 20090507:0700-1830,1830-2330;20090508:CLOSED. In TWS versions 965+ there is an option in the Global Configuration API settings to return 1 month of trading hours. In TWS version 970+, the format includes the date of the closing time to clarify potential ambiguity, ex: 20180323:0400-20180323:2000;20180326:0400-20180326:2000 The trading hours will correspond to the hours for the product on the associated exchange. The same instrument can have different hours on different exchanges.
// 	LiquidHours            string     // The liquid hours of the product. This value will contain the liquid hours (regular trading hours) of the contract on the specified exchange. Format for TWS versions until 969: 20090507:0700-1830,1830-2330;20090508:CLOSED. In TWS versions 965+ there is an option in the Global Configuration API settings to return 1 month of trading hours. In TWS v970 and above, the format includes the date of the closing time to clarify potential ambiguity, e.g. 20180323:0930-20180323:1600;20180326:0930-20180326:1600.
// 	EvRule                 string     // Contains the Economic Value Rule name and the respective optional argument. The two values should be separated by a colon. For example, aussieBond:YearsToExpiration=3. When the optional argument is not present, the first value will be followed by a colon.
// 	EvMultiplier           int        // Tells you approximately how much the market value of a contract would change if the price were to change by 1. It cannot be used to get market value by multiplying the price by the approximate multiplier.
// 	AggGroup               int        // Aggregated group Indicates the smart-routing group to which a contract belongs. contracts which cannot be smart-routed have aggGroup = -1.
// 	SecIdList              []TagValue // A list of contract identifiers that the customer is allowed to view. CUSIP/ISIN/etc. For US stocks, receiving the ISIN requires the CUSIP market data subscription. For Bonds, the CUSIP or ISIN is input directly into the symbol field of the Contract class.
// 	UnderSymbol            string     // For derivatives, the symbol of the underlying contract.
// 	UnderSecType           string     // For derivatives, returns the underlying security type.
// 	MarketRuleIds          string     // The list of market rule IDs separated by comma Market rule IDs can be used to determine the minimum price increment at a given price.
// 	RealExpirationDate     string     // Real expiration date. Requires TWS 968+ and API v973.04+. Python API specifically requires API v973.06+.
// 	LastTradeTime          string     //Last trade time.
// 	StockType              string     // Stock type.
// 	Cusip                  string     // The nine-character bond CUSIP. For Bonds only. Receiving CUSIPs requires a CUSIP market data subscription.
// 	Ratings                string     // Identifies the credit rating of the issuer. This field is not currently available from the TWS API. For Bonds only. A higher credit rating generally indicates a less risky investment. Bond ratings are from Moody's and S&P respectively. Not currently implemented due to bond market data restrictions.
// 	DescAppend             string     // A description string containing further descriptive information about the bond. For Bonds only.
// 	BondType               string     // The type of bond, such as "CORP.".
// 	CouponType             string     // The type of bond coupon. This field is currently not available from the TWS API. For Bonds only.
// 	Callable               bool       // If true, the bond can be called by the issuer under certain conditions. This field is currently not available from the TWS API. For Bonds only.
// 	Putable                bool       // Values are True or False. If true, the bond can be sold back to the issuer under certain conditions. This field is currently not available from the TWS API. For Bonds only.
// 	Coupon                 float64    // The interest rate used to calculate the amount you will receive in interest payments over the course of the year. This field is currently not available from the TWS API. For Bonds only.
// 	Convertible            bool       // Values are True or False. If true, the bond can be converted to stock under certain conditions. This field is currently not available from the TWS API. For Bonds only.
// 	Maturity               string     // The date on which the issuer must repay the face value of the bond. This field is currently not available from the TWS API. For Bonds only. Not currently implemented due to bond market data restrictions.
// 	IssueDate              string     // The date the bond was issued. This field is currently not available from the TWS API. For Bonds only. Not currently implemented due to bond market data restrictions.
// 	NextOptionDate         string     // Only if bond has embedded options. This field is currently not available from the TWS API. Refers to callable bonds and puttable bonds. Available in TWS description window for bonds.
// 	NextOptionType         string     // Type of embedded option. This field is currently not available from the TWS API. Only if bond has embedded options.
// 	NextOptionPartial      bool       // Only if bond has embedded options. This field is currently not available from the TWS API. For Bonds only.
// 	Notes                  string     // If populated for the bond in IB's database. For Bonds only.
// 	MinSize                float64    // Order's minimal size.
// 	SizeIncrement          float64    // Order's size increment.
// 	SuggestedSizeIncrement float64    // Order's suggested size increment.
// }

// TagValue struct {
// 	Tag   string
// 	Value string
// }

// Bar struct {
// 	Time   time.Time // The bar's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
// 	Open   float64   // The bar's open price.
// 	High   float64   // The bar's high price.
// 	Low    float64   // The bar's low price.
// 	Close  float64   // The bar's close price.
// 	Volume int64     // The bar's traded volume if available (only available for TRADES)
// 	WAP    float64   // The bar's Weighted Average Price (only available for TRADES)
// 	Count  int       // The number of trades during the bar's timespan (only available for TRADES)
// }

// Trade struct {
// 	TickType          string         // tick type: "Last" or "AllLast"
// 	Time              time.Time      // The trade's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
// 	Price             float64        // tick last price
// 	Size              int64          // tick last size
// 	TradeAttribute    TradeAttribute // tick attribs (bit 0 - past limit, bit 1 - unreported)
// 	Exchange          string         // tick exchange
// 	SpecialConditions string         // tick special conditions
// }

// TradeAttribute struct {
// 	PastLimit  bool
// 	Unreported bool
// }

// BidAsk struct {
// 	Time            time.Time       // The spread's date and time (either as a yyyymmss hh:mm:ss formatted string or as system time according to the request). Time zone is the TWS time zone chosen on login.
// 	BidPrice        float64         // tick-by-tick real-time tick bid price
// 	AskPrice        float64         // tick-by-tick real-time tick ask price
// 	BidSize         int64           // tick-by-tick real-time tick bid size
// 	AskSize         int64           // tick-by-tick real-time tick ask size
// 	BidAskAttribute BidAskAttribute // tick-by-tick real-time bid/ask tick attribs (bit 0 - bid past low, bit 1 - ask past high)
// }

// BidAskAttribute struct {
// 	BidPastLow  bool
// 	AskPastHigh bool
// }
