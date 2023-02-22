use std::time::Duration;
use std::{thread, time};

use clap::{arg, Command, ArgMatches};
use log::{debug, info};

use ibapi::client::IBClient;
use ibapi::contracts::{self, Contract};
use ibapi::orders::{self, Order};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = Command::new("place_order")
        .version("1.0")
        .author("Wil Boayue <wil.boayue@gmail.com>")
        .about("Submits order to broker")
        .arg(arg!(--connection_string <VALUE>).default_value("odin:4002"))
        .arg(arg!(--stock <SYMBOL>).required(true))
        .arg(arg!(--buy <QUANTITY>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--sell <QUANTITY>).value_parser(clap::value_parser!(i32)))
        .get_matches();

    let connection_string = matches
        .get_one::<String>("connection_string")
        .expect("connection_string is required");
    let stock_symbol = matches
        .get_one::<String>("stock")
        .expect("stock symbol is required");

    if let Some((action, quantity)) = get_order(&matches) {
        println!("action: {action}, quantity: {quantity}");
    }
    // println!("action: {action}, quantity: {quantity}");

    println!("connection_string: {connection_string}, stock_symbol: {stock_symbol}");

    let mut client = IBClient::connect("odin:4002")?;

    info!("Connected {client:?}");

    let mut contract = Contract::stock("TSLA");
    contract.currency = "USD".to_string();
    debug!("contract template {contract:?}");

    thread::sleep(Duration::from_secs(2));

    let order_id = 12;
    let order = Order{ order_id, solicited: todo!(), client_id: todo!(), perm_id: todo!(), action: todo!(), total_quantity: todo!(), order_type: todo!(), limit_price: todo!(), aux_price: todo!(), tif: todo!(), oca_group: todo!(), oca_type: todo!(), order_ref: todo!(), transmit: todo!(), parent_id: todo!(), block_order: todo!(), sweep_to_fill: todo!(), display_size: todo!(), trigger_method: todo!(), outside_rth: todo!(), hidden: todo!(), good_after_time: todo!(), good_till_date: todo!(), override_percentage_constraints: todo!(), rule_80_a: todo!(), all_or_none: todo!(), min_qty: todo!(), percent_offset: todo!(), trail_stop_price: todo!(), trailing_percent: todo!(), fa_group: todo!(), fa_profile: todo!(), fa_method: todo!(), fa_percentage: todo!(), open_close: todo!(), origin: todo!(), short_sale_slot: todo!(), designated_location: todo!(), exempt_code: todo!(), discretionary_amt: todo!(), opt_out_smart_routing: todo!(), auction_strategy: todo!(), starting_price: todo!(), stock_ref_price: todo!(), delta: todo!(), stock_range_lower: todo!(), stock_range_upper: todo!(), volatility: todo!(), volatility_type: todo!(), continuous_update: todo!(), reference_price_type: todo!(), delta_neutral_order_type: todo!(), delta_neutral_aux_price: todo!(), delta_neutral_con_id: todo!(), delta_neutral_settling_firm: todo!(), delta_neutral_clearing_account: todo!(), delta_neutral_clearing_intent: todo!(), delta_neutral_open_close: todo!(), delta_neutral_short_sale: todo!(), delta_neutral_short_sale_slot: todo!(), delta_neutral_designated_location: todo!(), basis_points: todo!(), basis_points_type: todo!(), scale_init_level_size: todo!(), scale_subs_level_size: todo!(), scale_price_increment: todo!(), scale_price_adjust_value: todo!(), scale_price_adjust_interval: todo!(), scale_profit_offset: todo!(), scale_auto_reset: todo!(), scale_init_position: todo!(), scale_init_fill_qty: todo!(), scale_random_percent: todo!(), hedge_type: todo!(), hedge_param: todo!(), account: todo!(), settling_firm: todo!(), clearing_account: todo!(), clearing_intent: todo!(), algo_strategy: todo!(), algo_params: todo!(), what_if: todo!(), algo_id: todo!(), not_held: todo!(), smart_combo_routing_params: todo!(), order_combo_legs: todo!(), order_misc_options: todo!(), active_start_time: todo!(), active_stop_time: todo!(), scale_table: todo!(), model_code: todo!(), ext_operator: todo!(), cash_qty: todo!(), mifid2_decision_maker: todo!(), mifid2_decision_algo: todo!(), mifid2_execution_trader: todo!(), mifid2_execution_algo: todo!(), dont_use_auto_price_for_hedge: todo!(), auto_cancel_date: todo!(), filled_quantity: todo!(), ref_futures_con_id: todo!(), auto_cancel_parent: todo!(), shareholder: todo!(), imbalance_only: todo!(), route_marketable_to_bbo: todo!(), parent_perm_id: todo!(), advanced_error_override: todo!(), manual_order_time: todo!(), min_trade_qty: todo!(), min_complete_size: todo!(), compete_against_best_offset: todo!(), mid_offset_at_whole: todo!(), mid_offset_at_half: todo!(), randomize_size: todo!(), randomize_price: todo!(), reference_contract_id: todo!(), is_pegged_change_amount_decrease: todo!(), pegged_change_amount: todo!(), reference_change_amount: todo!(), reference_exchange: todo!(), adjusted_order_type: todo!(), trigger_price: todo!(), lmt_price_offset: todo!(), adjusted_stop_price: todo!(), adjusted_stop_limit_price: todo!(), adjusted_trailing_amount: todo!(), adjustable_trailing_unit: todo!(), conditions: todo!(), conditions_ignore_rth: todo!(), conditions_cancel_order: todo!(), soft_dollar_tier: todo!(), is_oms_container: todo!(), discretionary_up_to_limit_price: todo!(), use_price_mgmt_algo: todo!(), duration: todo!(), post_to_ats: todo!() };
    let results = orders::place_order(&mut client, order_id, &contract, &order)?;

    println!("order: {results:?}");
    
    thread::sleep(time::Duration::from_secs(5));

    Ok(())
}

fn get_order(matches: &ArgMatches) -> Option<(String, i32)> {
    if let Some(quantity) = matches.get_one::<i32>("buy") {
        Some(("BUY".to_string(), *quantity))
    } else if let Some(quantity) = matches.get_one::<i32>("sell") {
        Some(("SELL".to_string(), *quantity))   
    } else {
        None
    }
}

// MarketOrder(action:str, quantity:Decimal):

// https://github.com/InteractiveBrokers/tws-api/blob/master/samples/Python/Testbed/OrderSamples.py