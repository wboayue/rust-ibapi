use time::OffsetDateTime;

pub struct DepthMktDataDescription {
    pub exchange: String,
    pub sec_type: String,
    pub listing_exch: String,
    pub service_data_type: String,
    pub agg_group: i32,
}

pub struct SmartComponent {
    pub bit_number: i32,
    pub exchange: String,
    pub exchange_letter: String,
}

pub struct TickAttrib {
    pub can_auto_execute: bool,
    pub past_limit: bool,
    pub pre_open: bool,
}

pub struct TickAttribBidAsk {
    pub bid_past_low: bool,
    pub ask_past_high: bool,
}

pub struct TickAttribLast {
    pub past_limit: bool,
    pub unreported: bool,
}

pub struct FamilyCode {
    pub account_id: String,
    pub family_code_str: String,
}

#[derive(Clone, Debug)]
pub struct NewsProvider {
    pub code: String,
    pub name: String,
}

pub enum ComboParam {
    NonGuaranteed,
    PriceCondConid,
    CondPriceMax,
    CondPriceMin,
    ChangeToMktTime1,
    ChangeToMktTime2,
    DiscretionaryPct,
    DontLeginNext,
    LeginPrio,
    MaxSegSize,
}

pub enum HedgeType {
    None,
    Delta,
    Beta,
    Fx,
    Pair,
}

pub enum Right {
    None,
    Put,
    Call,
}

pub enum VolatilityType {
    None,
    Daily,
    Annual,
}

pub enum ReferencePriceType {
    None,
    Midpoint,
    BidOrAsk,
}
