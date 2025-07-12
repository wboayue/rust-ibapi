/*
Description	Generic tick required	Delivery Method	Tick Id
Disable Default Market Data	Disables standard market data stream and allows the TWS & API feed to prioritize other listed generic tick types.	mdoff	–	–
Bid Size	Number of contracts or lots offered at the bid price.	–	IBApi.EWrapper.tickSize	0
Bid Price	Highest priced bid for the contract.	–	IBApi.EWrapper.tickPrice	1
Ask Price	Lowest price offer on the contract.	–	IBApi.EWrapper.tickPrice	2
Ask Size	Number of contracts or lots offered at the ask price.	–	IBApi.EWrapper.tickSize	3
Last Price	Last price at which the contract traded (does not include some trades in RTVolume).	–	IBApi.EWrapper.tickPrice	4
Last Size	Number of contracts or lots traded at the last price.	–	IBApi.EWrapper.tickSize	5
High	High price for the day.	–	IBApi.EWrapper.tickPrice	6
Low	Low price for the day.	–	IBApi.EWrapper.tickPrice	7
Volume	Trading volume for the day for the selected contract (US Stocks: multiplier 100).	–	IBApi.EWrapper.tickSize	8
Close Price	“The last available closing price for the previous day. For US Equities we use corporate action processing to get the closing price so the close price is adjusted to reflect forward and reverse splits and cash and stock dividends.”	–	IBApi.EWrapper.tickPrice	9
Bid Option Computation	Computed Greeks and implied volatility based on the underlying stock price and the option bid price. See Option Greeks	–	IBApi.EWrapper.tickOptionComputation	10
Ask Option Computation	Computed Greeks and implied volatility based on the underlying stock price and the option ask price. See Option Greeks	–	IBApi.EWrapper.tickOptionComputation	11
Last Option Computation	Computed Greeks and implied volatility based on the underlying stock price and the option last traded price. See Option Greeks	–	IBApi.EWrapper.tickOptionComputation	12
Model Option Computation	Computed Greeks and implied volatility based on the underlying stock price and the option model price. Correspond to greeks shown in TWS. See Option Greeks	–	IBApi.EWrapper.tickOptionComputation	13
Open Tick	Current session’s opening price. Before open will refer to previous day. The official opening price requires a market data subscription to the native exchange of the instrument.	–	IBApi.EWrapper.tickPrice	14
Low 13 Weeks	Lowest price for the last 13 weeks. For stocks only.	165	IBApi.EWrapper.tickPrice	15
High 13 Weeks	Highest price for the last 13 weeks. For stocks only.	165	IBApi.EWrapper.tickPrice	16
Low 26 Weeks	Lowest price for the last 26 weeks. For stocks only.	165	IBApi.EWrapper.tickPrice	17
High 26 Weeks	Highest price for the last 26 weeks. For stocks only.	165	IBApi.EWrapper.tickPrice	18
Low 52 Weeks	Lowest price for the last 52 weeks. For stocks only.	165	IBApi.EWrapper.tickPrice	19
High 52 Weeks	Highest price for the last 52 weeks. For stocks only.	165	IBApi.EWrapper.tickPrice	20
Average Volume	The average daily trading volume over 90 days. Multiplier of 100. For stocks only.	165	IBApi.EWrapper.tickSize	21
Open Interest	“(Deprecated not currently in use) Total number of options that are not closed.”	–	IBApi.EWrapper.tickSize	22
Option Historical Volatility	The 30-day historical volatility (currently for stocks).	104	IBApi.EWrapper.tickGeneric	23
Option Implied Volatility	“A prediction of how volatile an underlying will be in the future. The IB 30-day volatility is the at-market volatility estimated for a maturity thirty calendar days forward of the current trading day and is based on option prices from two consecutive expiration months.”	106	IBApi.EWrapper.tickGeneric	24
Option Bid Exchange	Not Used.	–	IBApi.EWrapper.tickString	25
Option Ask Exchange	Not Used.	–	IBApi.EWrapper.tickString	26
Option Call Open Interest	Call option open interest.	101	IBApi.EWrapper.tickSize	27
Option Put Open Interest	Put option open interest.	101	IBApi.EWrapper.tickSize	28
Option Call Volume	Call option volume for the trading day.	100	IBApi.EWrapper.tickSize	29
Option Put Volume	Put option volume for the trading day.	100	IBApi.EWrapper.tickSize	30
Index Future Premium	The number of points that the index is over the cash index.	162	IBApi.EWrapper.tickGeneric	31
Bid Exchange	“For stock and options identifies the exchange(s) posting the bid price. See Component Exchanges”	–	IBApi.EWrapper.tickString	32
Ask Exchange	“For stock and options identifies the exchange(s) posting the ask price. See Component Exchanges”	–	IBApi.EWrapper.tickString	33
Auction Volume	The number of shares that would trade if no new orders were received and the auction were held now.	225	IBApi.EWrapper.tickSize	34
Auction Price	The price at which the auction would occur if no new orders were received and the auction were held now- the indicative price for the auction. Typically received after Auction imbalance (tick type 36)	225	IBApi.EWrapper.tickPrice	35
Auction Imbalance	The number of unmatched shares for the next auction; returns how many more shares are on one side of the auction than the other. Typically received after Auction Volume (tick type 34)	225	IBApi.EWrapper.tickSize	36
Mark Price	“The mark price is the current theoretical calculated value of an instrument. Since it is a calculated value it will typically have many digits of precision.”	232	IBApi.EWrapper.tickPrice	37
Bid EFP Computation	Computed EFP bid price	–	IBApi.EWrapper.tickEFP	38
Ask EFP Computation	Computed EFP ask price	–	IBApi.EWrapper.tickEFP	39
Last EFP Computation	Computed EFP last price	–	IBApi.EWrapper.tickEFP	40
Open EFP Computation	Computed EFP open price	–	IBApi.EWrapper.tickEFP	41
High EFP Computation	Computed high EFP traded price for the day	–	IBApi.EWrapper.tickEFP	42
Low EFP Computation	Computed low EFP traded price for the day	–	IBApi.EWrapper.tickEFP	43
Close EFP Computation	Computed closing EFP price for previous day	–	IBApi.EWrapper.tickEFP	44
Last Timestamp	Time of the last trade (in UNIX time).	–	IBApi.EWrapper.tickString	45
Shortable	Describes the level of difficulty with which the contract can be sold short. See Shortable	236	IBApi.EWrapper.tickGeneric	46
RT Volume (Time & Sales)	“Last trade details (Including both “”Last”” and “”Unreportable Last”” trades). See RT Volume”	233	IBApi.EWrapper.tickString	48
Halted	Indicates if a contract is halted. See Halted	–	IBApi.EWrapper.tickGeneric	49
Bid Yield	Implied yield of the bond if it is purchased at the current bid.	–	IBApi.EWrapper.tickPrice	50
Ask Yield	Implied yield of the bond if it is purchased at the current ask.	–	IBApi.EWrapper.tickPrice	51
Last Yield	Implied yield of the bond if it is purchased at the last price.	–	IBApi.EWrapper.tickPrice	52
Custom Option Computation	Greek values are based off a user customized price.	–	IBApi.EWrapper.tickOptionComputation	53
Trade Count	Trade count for the day.	293	IBApi.EWrapper.tickGeneric	54
Trade Rate	Trade count per minute.	294	IBApi.EWrapper.tickGeneric	55
Volume Rate	Volume per minute.	295	IBApi.EWrapper.tickGeneric	56
Last RTH Trade	Last Regular Trading Hours traded price.	318	IBApi.EWrapper.tickPrice	57
RT Historical Volatility	30-day real time historical volatility.	411	IBApi.EWrapper.tickGeneric	58
IB Dividends	Contract’s dividends. See IB Dividends.	456	IBApi.EWrapper.tickString	59
Bond Factor Multiplier	The bond factor is a number that indicates the ratio of the current bond principal to the original principal	460	IBApi.EWrapper.tickGeneric	60
Regulatory Imbalance	The imbalance that is used to determine which at-the-open or at-the-close orders can be entered following the publishing of the regulatory imbalance.	225	IBApi.EWrapper.tickSize	61
News	Contract’s news feed.	292	IBApi.EWrapper.tickString	62
Short-Term Volume 3 Minutes	The past three minutes volume. Interpolation may be applied. For stocks only.	595	IBApi.EWrapper.tickSize	63
Short-Term Volume 5 Minutes	The past five minutes volume. Interpolation may be applied. For stocks only.	595	IBApi.EWrapper.tickSize	64
Short-Term Volume 10 Minutes	The past ten minutes volume. Interpolation may be applied. For stocks only.	595	IBApi.EWrapper.tickSize	65
Delayed Bid	Delayed bid price. See Market Data Types.	–	IBApi.EWrapper.tickPrice	66
Delayed Ask	Delayed ask price. See Market Data Types.	–	IBApi.EWrapper.tickPrice	67
Delayed Last	Delayed last traded price. See Market Data Types.	–	IBApi.EWrapper.tickPrice	68
Delayed Bid Size	Delayed bid size. See Market Data Types.	–	IBApi.EWrapper.tickSize	69
Delayed Ask Size	Delayed ask size. See Market Data Types.	–	IBApi.EWrapper.tickSize	70
Delayed Last Size	Delayed last size. See Market Data Types.	–	IBApi.EWrapper.tickSize	71
Delayed High Price	Delayed highest price of the day. See Market Data Types.	–	IBApi.EWrapper.tickPrice	72
Delayed Low Price	Delayed lowest price of the day. See Market Data Types	–	IBApi.EWrapper.tickPrice	73
Delayed Volume	Delayed traded volume of the day. See Market Data Types	–	IBApi.EWrapper.tickSize	74
Delayed Close	The prior day’s closing price.	–	IBApi.EWrapper.tickPrice	75
Delayed Open	Not currently available	–	IBApi.EWrapper.tickPrice	76
RT Trade Volume	“Last trade details that excludes “”Unreportable Trades””. See RT Trade Volume”	375	IBApi.EWrapper.tickString	77
Creditman mark price	Not currently available		IBApi.EWrapper.tickPrice	78
Creditman slow mark price	Slower mark price update used in system calculations	619	IBApi.EWrapper.tickPrice	79
Delayed Bid Option	Computed greeks based on delayed bid price. See Market Data Types and Option Greeks.		IBApi.EWrapper.tickPrice	80
Delayed Ask Option	Computed greeks based on delayed ask price. See Market Data Types and Option Greeks.		IBApi.EWrapper.tickPrice	81
Delayed Last Option	Computed greeks based on delayed last price. See Market Data Types and Option Greeks.		IBApi.EWrapper.tickPrice	82
Delayed Model Option	Computed Greeks and model’s implied volatility based on delayed stock and option prices.		IBApi.EWrapper.tickPrice	83
Last Exchange	Exchange of last traded price		IBApi.EWrapper.tickString	84
Last Regulatory Time	Timestamp (in Unix ms time) of last trade returned with regulatory snapshot		IBApi.EWrapper.tickString	85
Futures Open Interest	Total number of outstanding futures contracts. *HSI open interest requested with generic tick 101	588	IBApi.EWrapper.tickSize	86
Average Option Volume	Average volume of the corresponding option contracts(TWS Build 970+ is required)	105	IBApi.EWrapper.tickSize	87
Delayed Last Timestamp	Delayed time of the last trade (in UNIX time) (TWS Build 970+ is required)		IBApi.EWrapper.tickString	88
Shortable Shares	Number of shares available to short (TWS Build 974+ is required)	236	IBApi.EWrapper.tickSize	89
ETF Nav Close	Today’s closing price of ETF’s Net Asset Value (NAV). Calculation is based on prices of ETF’s underlying securities.	578	IBApi.EWrapper.tickPrice	92
ETF Nav Prior Close	Yesterday’s closing price of ETF’s Net Asset Value (NAV). Calculation is based on prices of ETF’s underlying securities.	578	IBApi.EWrapper.tickPrice	93
ETF Nav Bid	The bid price of ETF’s Net Asset Value (NAV). Calculation is based on prices of ETF’s underlying securities.	576	IBApi.EWrapper.tickPrice	94
ETF Nav Ask	The ask price of ETF’s Net Asset Value (NAV). Calculation is based on prices of ETF’s underlying securities.	576	IBApi.EWrapper.tickPrice	95
ETF Nav Last	The last price of Net Asset Value (NAV). For ETFs: Calculation is based on prices of ETF’s underlying securities. For NextShares: Value is provided by NASDAQ	577	IBApi.EWrapper.tickPrice	96
ETF Nav Frozen Last	ETF Nav Last for Frozen data	623	IBApi.EWrapper.tickPrice	97
ETF Nav High	The high price of ETF’s Net Asset Value (NAV)	614	IBApi.EWrapper.tickPrice	98
ETF Nav Low	The low price of ETF’s Net Asset Value (NAV)	614	IBApi.EWrapper.tickPrice	99
Estimated IPO – Midpoint	Midpoint is calculated based on IPO price range	586	IBApi.EWrapper.tickGeneric	101
Final IPO Price	Final price for IPO	586	IBApi.EWrapper.tickGeneric	102
Delayed Yield Bid	Delayed implied yield of the bond if it is purchased at the current bid.	–	IBApi.EWrapper.tickPrice	103
Delayed Yield Ask	Delayed implied yield of the bond if it is purchased at the current ask.	–	IBApi.EWrapper.tickPrice	104
*/
