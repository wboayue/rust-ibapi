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

pub struct FamilyCode {
    pub account_id: String,
    pub family_code_str: String,
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
