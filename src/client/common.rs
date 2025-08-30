#[cfg(test)]
pub mod mocks {
    use byteorder::{BigEndian, ReadBytesExt};
    use std::io::{Cursor, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use time_tz::timezones;

    use crate::messages::{encode_length, OutgoingMessages};

    #[derive(Debug, Clone)]
    struct Interaction {
        request: OutgoingMessages,
        responses: Vec<String>,
    }

    pub struct MockGateway {
        handle: Option<thread::JoinHandle<()>>,
        requests: Arc<Mutex<Vec<String>>>,
        interactions: Vec<Interaction>,
        server_version: i32,
        address: Option<String>,
    }

    impl MockGateway {
        pub fn new(server_version: i32) -> Self {
            MockGateway {
                handle: None,
                requests: Arc::new(Mutex::new(Vec::new())),
                interactions: Vec::new(),
                server_version,
                address: None,
            }
        }

        pub fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        pub fn address(&self) -> String {
            self.address.clone().unwrap_or_default()
        }

        pub fn add_interaction(&mut self, request: OutgoingMessages, responses: Vec<String>) {
            self.interactions.push(Interaction { request, responses });
        }

        pub fn server_version(&self) -> i32 {
            self.server_version
        }

        pub fn time_zone(&self) -> Option<&time_tz::Tz> {
            Some(timezones::db::EST)
        }

        pub fn start(&mut self) -> Result<(), anyhow::Error> {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;

            let requests = Arc::clone(&self.requests);
            let interactions = self.interactions.clone();
            let server_version = self.server_version;

            let handle = thread::spawn(move || {
                // Handle single request and exit
                let stream = match listener.accept() {
                    Ok((stream, addr)) => {
                        println!("MockGateway: Accepted connection from {}", addr);
                        stream
                    }
                    Err(e) => {
                        eprintln!("MockGateway: Failed to accept connection: {}", e);
                        return;
                    }
                };

                let mut handler = ConnectionHandler::new(server_version, requests.clone(), interactions.clone());
                if let Err(err) = handler.handle(stream) {
                    // Error handling connection
                    eprintln!("MockGateway: Error handling connection: {}", err);
                }
            });

            self.handle = Some(handle);
            self.address = Some(address.to_string());

            Ok(())
        }
    }

    impl Drop for MockGateway {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                handle.join().expect("Failed to join mock gateway thread");
            }
        }
    }

    struct ConnectionHandler {
        requests: Arc<Mutex<Vec<String>>>,
        interactions: Vec<Interaction>,
        current_interaction: usize,
        server_version: i32,
    }

    impl ConnectionHandler {
        pub fn new(server_version: i32, requests: Arc<Mutex<Vec<String>>>, interactions: Vec<Interaction>) -> Self {
            ConnectionHandler {
                requests,
                interactions,
                current_interaction: 0,
                server_version,
            }
        }

        pub fn handshake_response(&self) -> String {
            format!("{}\020240120 12:00:00 EST\0", self.server_version)
        }

        pub fn read_message(&mut self, stream: &mut TcpStream) -> Result<String, std::io::Error> {
            let size = self.read_size(stream)?;
            let mut buf = vec![0u8; size];
            stream.read_exact(&mut buf)?;
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }

        pub fn write_message(&mut self, stream: &mut TcpStream, message: String) -> Result<(), std::io::Error> {
            let packet = encode_length(&message);
            stream.write_all(&packet)?;
            Ok(())
        }

        pub fn handle(&mut self, mut stream: TcpStream) -> Result<(), std::io::Error> {
            self.handle_startup(&mut stream)?;

            if self.interactions.is_empty() {
                self.send_shutdown(&mut stream)?;
                return Ok(());
            }

            // Set a read timeout so we don't wait forever for requests
            stream.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

            while let Ok(request) = self.read_message(&mut stream) {
                self.add_request(request.clone());

                // Check if we have a matching interaction
                if self.current_interaction < self.interactions.len() {
                    let interaction = self.interactions[self.current_interaction].clone();
                    if request.starts_with(&format!("{}\0", interaction.request)) {
                        println!("MockGateway: Sending {} responses", interaction.responses.len());
                        for (i, response) in interaction.responses.iter().enumerate() {
                            let msg_type = response.split('\0').next().unwrap_or("unknown");
                            println!("MockGateway: Sending response {} (type {})", i + 1, msg_type);
                            self.write_message(&mut stream, response.clone())?;
                            stream.flush()?;
                            // Small delay between messages to ensure proper delivery
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        self.current_interaction += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Keep the connection alive for a bit to allow client to read all messages
            std::thread::sleep(std::time::Duration::from_millis(500));

            self.send_shutdown(&mut stream)?;
            println!("MockGateway: Shutdown sent, closing connection");

            // Give the client a moment to read the shutdown message
            std::thread::sleep(std::time::Duration::from_millis(50));

            Ok(())
        }

        pub fn handle_startup(&mut self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
            let magic_token = self.read_magic_token(stream)?;
            assert_eq!(magic_token, "API\0");

            // Supported server versions
            let supported_versions = self.read_message(stream)?;
            println!("Supported server versions: {}", supported_versions);

            // Send handshake response
            self.write_message(stream, self.handshake_response())?;

            // Start API
            let message = self.read_message(stream)?;
            // For server versions > 72 (OPTIONAL_CAPABILITIES), expect an extra empty field
            if self.server_version > 72 {
                assert_eq!(message, "71\02\0100\0\0");
            } else {
                assert_eq!(message, "71\02\0100\0");
            }

            // next valid order id
            self.write_message(stream, "9\01\090\0".to_string())?;

            // managed accounts
            self.write_message(stream, "15\01\02334\0".to_string())?;

            Ok(())
        }

        pub fn send_shutdown(&mut self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
            // signal shutdown
            println!("Sending shutdown message");
            self.write_message(stream, "-2\01\0".to_string())?;

            // Flush to ensure the message is sent
            stream.flush()?;

            Ok(())
        }

        pub fn add_request(&mut self, request: String) {
            let mut requests = self.requests.lock().unwrap();
            requests.push(request);
        }

        pub fn read_magic_token(&mut self, stream: &mut TcpStream) -> Result<String, std::io::Error> {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf)?;
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }

        fn read_size(&mut self, stream: &mut TcpStream) -> Result<usize, std::io::Error> {
            let buffer = &mut [0_u8; 4];
            stream.read_exact(buffer)?;
            let mut reader = Cursor::new(buffer);
            let count = reader.read_u32::<BigEndian>()?;
            Ok(count as usize)
        }
    }
}

#[cfg(test)]
pub mod tests {
    use time::OffsetDateTime;

    use crate::{client::common::mocks::MockGateway, messages::OutgoingMessages, server_versions};

    pub fn setup_connect() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);
        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub struct ServerTimeExpectations {
        pub server_time: OffsetDateTime,
    }

    pub fn setup_server_time() -> (MockGateway, ServerTimeExpectations) {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        let server_time = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();

        gateway.add_interaction(
            OutgoingMessages::RequestCurrentTime,
            vec![format!("49\01\0{}\0", server_time.unix_timestamp())],
        );

        gateway.start().expect("Failed to start mock gateway");

        (gateway, ServerTimeExpectations { server_time })
    }

    pub struct ManagedAccountsExpectations {
        pub accounts: Vec<String>,
    }

    pub fn setup_managed_accounts() -> (MockGateway, ManagedAccountsExpectations) {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);
        let expected_accounts = vec!["DU1234567".to_string(), "DU1234568".to_string()];

        gateway.add_interaction(
            OutgoingMessages::RequestManagedAccounts,
            vec![format!("15\01\0{}\0", expected_accounts.join(","))],
        );

        gateway.start().expect("Failed to start mock gateway");

        (gateway, ManagedAccountsExpectations { accounts: expected_accounts })
    }

    pub struct NextValidOrderIdExpectations {
        pub next_valid_order_id: i32,
    }

    pub fn setup_next_valid_order_id() -> (MockGateway, NextValidOrderIdExpectations) {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);
        let expected_order_id = 12345;

        gateway.add_interaction(OutgoingMessages::RequestIds, vec![format!("9\01\0{}\0", expected_order_id)]);

        gateway.start().expect("Failed to start mock gateway");

        (
            gateway,
            NextValidOrderIdExpectations {
                next_valid_order_id: expected_order_id,
            },
        )
    }

    pub fn setup_positions() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestPositions,
            vec![
                "61\03\0DU1234567\012345\0AAPL\0STK\0\00.0\0\0\0SMART\0USD\0AAPL\0AAPL\0500.0\0150.25\0".to_string(),
                "62\01\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_positions_multi() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestPositionsMulti,
            vec![
                "71\03\09000\0DU1234567\012345\0AAPL\0STK\0\00.0\0\0\0SMART\0USD\0AAPL\0AAPL\0500.0\0150.25\0MODEL1\0".to_string(),
                "71\03\09000\0DU1234568\067890\0GOOGL\0STK\0\00.0\0\0\0SMART\0USD\0GOOGL\0GOOGL\0200.0\02500.00\0MODEL1\0".to_string(),
                "72\01\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_account_summary() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestAccountSummary,
            vec![
                "63\01\09000\0DU1234567\0NetLiquidation\025000.00\0USD\0".to_string(),
                "63\01\09000\0DU1234567\0TotalCashValue\015000.00\0USD\0".to_string(),
                "64\01\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_pnl() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(OutgoingMessages::RequestPnL, vec!["94\09000\0250.50\01500.00\0750.00\0".to_string()]);

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_pnl_single() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Response format: message_type\0request_id\0position\0daily_pnl\0unrealized_pnl\0realized_pnl\0value\0
        gateway.add_interaction(
            OutgoingMessages::RequestPnLSingle,
            vec!["95\09000\0100.0\0150.25\0500.00\0250.00\01000.00\0".to_string()],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_family_codes() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Response format: message_type\0count\0account_id\0family_code\0...
        gateway.add_interaction(
            OutgoingMessages::RequestFamilyCodes,
            vec!["78\02\0DU1234567\0FAM001\0DU1234568\0FAM002\0".to_string()],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_account_updates() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestAccountData,
            vec![
                "6\02\0NetLiquidation\025000.00\0USD\0DU1234567\0".to_string(),
                // PortfolioValue v4: type(7), version(4), symbol, sec_type, expiry, strike, right,
                // currency, local_symbol, position, market_price, market_value, avg_cost, unrealized_pnl, realized_pnl, account
                // (NO contract_id, multiplier, primary_exchange, trading_class in v4!)
                "7\04\0AAPL\0STK\0\00.0\0\0USD\0AAPL\0500.0\0151.50\075750.00\0150.25\0375.00\0125.00\0DU1234567\0".to_string(),
                // AccountUpdateTime: type(8), version(ignored), timestamp
                "8\01\020240122 15:30:00\0".to_string(),
                "54\01\0DU1234567\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_account_updates_multi() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestAccountUpdatesMulti,
            vec![
                // AccountUpdateMulti: type(73), version(1), request_id, account, model_code, key, value, currency
                "73\01\09000\0DU1234567\0\0CashBalance\094629.71\0USD\0".to_string(),
                "73\01\09000\0DU1234567\0\0Currency\0USD\0USD\0".to_string(),
                "73\01\09000\0DU1234567\0\0StockMarketValue\00.00\0BASE\0".to_string(),
                // AccountUpdateMultiEnd: type(74), version(1), request_id
                "74\01\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_contract_details() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestContractData,
            vec![
                // ContractData: type(10), symbol, sec_type, last_trade_date, strike, right, exchange, currency, 
                // local_symbol, market_name, trading_class, contract_id, min_tick, multiplier,
                // order_types, valid_exchanges, price_magnifier, under_contract_id, long_name, primary_exchange,
                // contract_month, industry, category, subcategory, time_zone_id, trading_hours, liquid_hours,
                // ev_rule, ev_multiplier, sec_id_list_count, agg_group, under_symbol, under_security_type,
                // market_rule_ids, real_expiration_date, stock_type, min_size, size_increment, suggested_size_increment
                "10\09000\0AAPL\0STK\0\00.0\0\0NASDAQ\0USD\0AAPL\0NMS\0AAPL\0265598\00.01\0\0ACTIVETIM,AD,ADJUST,ALERT,ALGO,ALLOC,AON,AVGCOST,BASKET,BENCHPX,CASHQTY,COND,CONDORDER,DARKONLY,DARKPOLL,DAY,DEACT,DEACTDIS,DEACTPX,DIS,FOK,GAT,GTC,GTD,GTT,HID,IBKRATS,ICE,IMB,IOC,LIT,LMT,LOC,MIDPX,MIT,MKT,MOC,MTL,NGCOMB,NODARK,NONALGO,OCA,OPG,OPGREROUT,PEGBENCH,PEGMID,PEGMKT,PEGPRI,PEGSTK,POSTONLY,PREOPGRTH,PRICEIMP,REL,RPI,RTH,SCALE,SCALEODD,SCALERST,SIZEPRIO,SNAPMID,SNAPMKT,SNAPREL,STP,STPLMT,SWEEP,TRAIL,TRAILLIT,TRAILLMT,TRAILMIT,WHATIF\0SMART,AMEX,NYSE,CBOE,PHLX,ISE,CHX,ARCA,ISLAND,DRCTEDGE,BEX,BATS,EDGEA,CSFBALGO,JEFFALGO,BYX,IEX,EDGX,FOXRIVER,PEARL,NYSENAT,LTSE,MEMX,TPLUS1,PSX\00\00\0Apple Inc\0NASDAQ\0\0Technology\0Computers\0Computers\0US/Eastern\020240122:0930-1600;20240123:0930-1600\020240122:0930-1600;20240123:0930-1600\0\00\00\00\0\0\0Consolidated\0\0NMS\01\01\01\0".to_string(),
                // ContractDataEnd: type(52), version(1), request_id
                "52\01\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_matching_symbols() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::BOND_ISSUERID);

        gateway.add_interaction(
            OutgoingMessages::RequestMatchingSymbols,
            vec![
                // SymbolSamples: type(79), request_id, count, 
                // (contract_id, symbol, security_type, primary_exchange, currency, deriv_sec_types_count, [deriv_types], description, issuer_id)*
                // Format based on the decoder test
                "79\09000\02\0265598\0AAPL\0STK\0NASDAQ\0USD\02\0OPT\0WAR\0Apple Inc.\0AAPL123\0276821\0MSFT\0STK\0NASDAQ\0USD\01\0OPT\0Microsoft Corporation\0MSFT456\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_market_rule() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::MARKET_RULES);

        gateway.add_interaction(
            OutgoingMessages::RequestMarketRule,
            vec![
                // MarketRule: type(93), market_rule_id, price_increments_count, (low_edge, increment)*
                // Example with 3 price increments for market rule ID 26
                "93\026\03\00.0\00.01\0100.0\00.05\01000.0\00.10\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_calculate_option_price() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::PRICE_BASED_VOLATILITY);

        gateway.add_interaction(
            OutgoingMessages::ReqCalcImpliedVolat,
            vec![
                // TickOptionComputation: type(21), request_id, tick_type, tick_attribute,
                // implied_volatility, delta, option_price, pv_dividend, gamma, vega, theta, underlying_price
                // tick_type=13 (ModelOption), tick_attribute=0
                "21\09000\013\00\00.25\00.5\012.75\00.0\00.05\00.02\0-0.01\0100.0\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_calculate_implied_volatility() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::PRICE_BASED_VOLATILITY);

        gateway.add_interaction(
            OutgoingMessages::ReqCalcImpliedVolat,
            vec![
                // TickOptionComputation: type(21), request_id, tick_type, tick_attribute,
                // implied_volatility, delta, option_price, pv_dividend, gamma, vega, theta, underlying_price
                // tick_type=13 (ModelOption), tick_attribute=1 (price-based)
                // When calculating IV from price, we get back the computed IV (0.35 in this example)
                "21\09000\013\01\00.35\00.45\015.50\00.0\00.04\00.03\0-0.02\0105.0\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_option_chain() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::SEC_DEF_OPT_PARAMS_REQ);

        gateway.add_interaction(
            OutgoingMessages::RequestSecurityDefinitionOptionalParameters,
            vec![
                // SecurityDefinitionOptionParameter: type(75), request_id, exchange, underlying_contract_id,
                // trading_class, multiplier, expirations_count, expirations, strikes_count, strikes
                "75\09000\0SMART\0265598\0AAPL\0100\03\020250117\020250221\020250321\05\090.0\095.0\0100.0\0105.0\0110.0\0".to_string(),
                // Multiple exchanges can be returned
                "75\09000\0CBOE\0265598\0AAPL\0100\02\020250117\020250221\04\095.0\0100.0\0105.0\0110.0\0".to_string(),
                // SecurityDefinitionOptionParameterEnd: type(76), request_id
                "76\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_open_orders() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestOpenOrders,
            vec![
                // OpenOrder message - Order 1: Copy exact format from place_order test
                "5\01001\0265598\0AAPL\0STK\0\00\0?\0\0SMART\0USD\0AAPL\0NMS\0BUY\0100\0MKT\00.0\00.0\0DAY\0\0DU1236109\0\00\0\0100\01377295418\00\00\00\0\01377295418.0/DU1236109/100\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0PreSubmitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OpenOrder message - Order 2: Same format but changed order_id, symbol, action, quantity, order_type, limit_price
                "5\01002\0276821\0MSFT\0STK\0\00\0?\0\0SMART\0USD\0MSFT\0NMS\0SELL\050\0LMT\0350.0\00.0\0DAY\0\0DU1236109\0\00\0\0100\01377295419\00\00\00\0\01377295419.0/DU1236109/100\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0Submitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OpenOrderEnd message: type(53), version(1)
                "53\01\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_all_open_orders() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestAllOpenOrders,
            vec![
                // OpenOrder message - Order 1: TSLA order from different client
                "5\02001\076792\0TSLA\0STK\0\00\0?\0\0SMART\0USD\0TSLA\0NMS\0BUY\010\0LMT\0420.0\00.0\0GTC\0\0DU1236110\0\00\0\0101\01377295500\00\00\00\0\01377295500.0/DU1236110/101\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0Submitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OpenOrder message - Order 2: AMZN order from different client  
                "5\02002\03691\0AMZN\0STK\0\00\0?\0\0SMART\0USD\0AMZN\0NMS\0SELL\05\0MKT\00.0\00.0\0DAY\0\0DU1236111\0\00\0\0102\01377295501\00\00\00\0\01377295501.0/DU1236111/102\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0PreSubmitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OpenOrder message - Order 3: GOOGL order from current client
                "5\01003\026578\0GOOGL\0STK\0\00\0?\0\0SMART\0USD\0GOOGL\0NMS\0BUY\020\0LMT\02800.0\00.0\0DAY\0\0DU1236109\0\00\0\0100\01377295502\00\00\00\0\01377295502.0/DU1236109/100\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0Submitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OpenOrderEnd message
                "53\01\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_auto_open_orders() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestAutoOpenOrders,
            vec![
                // OrderStatus message - Order 3001 status update
                "3\03001\0PreSubmitted\00\0100\00\0123456\00\00\0100\0\00\0".to_string(),
                // OpenOrder message - Order 3001: FB order from TWS
                "5\03001\013407\0FB\0STK\0\00\0?\0\0SMART\0USD\0FB\0NMS\0BUY\050\0MKT\00.0\00.0\0DAY\0\0TWS\0\00\0\00\01377295600\00\00\00\0\01377295600.0/TWS/0\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0PreSubmitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OrderStatus message - Order 3001 submitted
                "3\03001\0Submitted\00\0100\00\0123456\00\00\0100\0\00\0".to_string(),
                // OpenOrderEnd message
                "53\01\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_completed_orders() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::COMPLETED_ORDERS);

        // Real CompletedOrder message captured from IB Gateway
        // This is a cancelled ES futures order
        let msg1 = "101\0637533641\0ES\0FUT\020250919\00\0?\050\0CME\0USD\0ESU5\0ES\0BUY\01\0LMT\01000.0\00.0\0GTC\0\0DU1236109\0\00\0\0616088517\00\00\00\0\0\0\0\0\0\0\0\0\0\00\0\0-1\0\0\0\0\0\02147483647\00\00\0\03\00\0\00\0None\0\00\00\00\0\00\00\0\0\0\00\00\00\02147483647\02147483647\0\0\0\0IB\00\00\0\00\0Cancelled\00\00\00\01001.0\01.7976931348623157E308\00\01\00\0\00\02147483647\00\0Not an insider or substantial shareholder\00\00\09223372036854775807\020250810 09:07:39 America/Los_Angeles\0Cancelled by Trader\0\0\0\0\0\0".to_string();

        // Create a second message for a filled stock order (AAPL)
        let msg2 = "101\0265598\0AAPL\0STK\0\00\0\0\0SMART\0USD\0AAPL\0NMS\0BUY\0100\0MKT\00.0\00.0\0DAY\0\0DU1236109\0\00\0\01377295418\00\00\00\0\0\0\0\0\0\0\0\0\0\00\0\0-1\0\0\0\0\0\02147483647\00\00\0\03\00\0\00\0None\0\00\00\00\0\00\00\0\0\0\00\00\00\02147483647\02147483647\0\0\0\0IB\00\00\0\00\0Filled\0100\00\00\0150.25\01.7976931348623157E308\00\01\00\0\00\02147483647\00\0Not an insider or substantial shareholder\00\00\09223372036854775807\020231122 10:30:00 America/Los_Angeles\0Filled\0\0\0\0\0\0".to_string();

        gateway.add_interaction(
            OutgoingMessages::RequestCompletedOrders,
            vec![
                msg1,
                msg2,
                // CompletedOrdersEnd message - captured from real IB Gateway
                "102\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_place_order() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::PlaceOrder,
            vec![
                // Real OrderStatus message captured from TWS
                "3\01001\0PreSubmitted\00\0100\00\0123456\00\00\0100\0\00\0".to_string(),
                // Another OrderStatus showing submitted
                "3\01001\0Submitted\00\0100\00\0123456\00\00\0100\0\00\0".to_string(),
                // OrderStatus update when filled
                "3\01001\0Filled\0100\00\0150.25\0123456\00\0150.25\0100\0\00\0".to_string(),
                // OpenOrder message - captured from real TWS, with order_id changed to 1001
                "5\01001\0265598\0AAPL\0STK\0\00\0?\0\0SMART\0USD\0AAPL\0NMS\0BUY\0100\0LMT\01.0\00.0\0DAY\0\0DU1236109\0\00\0\0100\01377295418\00\00\00\0\01377295418.0/DU1236109/100\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0PreSubmitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // ExecutionData message - complete format with all required fields
                // Fields: type(11), request_id, order_id, contract_id, symbol, sec_type, 
                // last_trade_date, strike, right, multiplier, exchange, currency, local_symbol, trading_class,
                // execution_id, time, account, exchange, side, shares, price, perm_id, client_id,
                // liquidation, cum_qty, avg_price, order_ref, ev_rule, ev_multiplier, model_code, last_liquidity
                "11\0-1\01001\0265598\0AAPL\0STK\0\00.0\0\0\0SMART\0USD\0AAPL\0AAPL\0000e1a2b.67890abc.01.01\020240125 10:30:00\0DU1234567\0SMART\0BOT\0100\0150.25\0123456\0100\00\0100\0150.25\0\0\00.0\0\00\0".to_string(),
                // CommissionReport message
                // type(59), version(1), execution_id, commission, currency, realized_pnl, yield, yield_redemption_date
                "59\01\0000e1a2b.67890abc.01.01\01.25\0USD\00.0\00.0\00\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_cancel_order() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add interaction for cancel_order
        gateway.add_interaction(
            OutgoingMessages::CancelOrder,
            vec![
                // OrderStatus message showing order was cancelled
                // Fields: type(3), order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id, last_fill_price, client_id, why_held, mkt_cap_price
                "3\01001\0Cancelled\00\0100\00.0\0123456\00\00.0\0100\0\00\0".to_string(),
                // Error message confirming cancellation (optional but common)
                // Fields: type(4), version(2), id(order_id), error_code(202), error_string
                "4\02\01001\0202\0Order Cancelled - reason:User requested order cancellation\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_global_cancel() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add interaction for global_cancel - simulates cancelling two open orders
        gateway.add_interaction(
            OutgoingMessages::RequestGlobalCancel,
            vec![
                // OrderStatus for first cancelled order
                // Fields: type(3), order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id, last_fill_price, client_id, why_held, mkt_cap_price
                "3\01033\0Cancelled\00\0100\00\0137729541\00\00\0100\0\00\0".to_string(),
                // Error message for first order
                // Fields: type(4), version(2), id(order_id), error_code(202), error_string, advanced_order_reject_json
                "4\02\01033\0202\0Order Canceled - reason:\0\0".to_string(),
                // OrderStatus for second cancelled order
                "3\01034\0Cancelled\00\050\00\0137729542\00\00\0100\0\00\0".to_string(),
                // Error message for second order
                "4\02\01034\0202\0Order Canceled - reason:\0\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_executions() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add interaction for RequestExecutions message
        gateway.add_interaction(
            OutgoingMessages::RequestExecutions,
            vec![
                // ExecutionData message - stock execution (AAPL)
                // Fields: type(11), request_id, order_id, contract_id, symbol, sec_type, 
                // last_trade_date, strike, right, multiplier, exchange, currency, local_symbol, trading_class,
                // execution_id, time, account, exchange, side, shares, price, perm_id, client_id,
                // liquidation, cum_qty, avg_price, order_ref, ev_rule, ev_multiplier, model_code, last_liquidity
                "11\09000\01001\0265598\0AAPL\0STK\0\00.0\0\0\0SMART\0USD\0AAPL\0AAPL\0000e1a2b.67890abc.01.01\020240125 10:30:00\0DU1234567\0SMART\0BOT\0100\0150.25\0123456\0100\00\0100\0150.25\0\0\00.0\0\00\0".to_string(),
                // CommissionReport message for first execution
                // Fields: type(59), version(1), execution_id, commission, currency, realized_pnl, yield, yield_redemption_date
                "59\01\0000e1a2b.67890abc.01.01\01.25\0USD\00.0\00.0\00\0".to_string(),
                // ExecutionData message - futures execution (ES)
                "11\09000\01002\0637533641\0ES\0FUT\020250919\00.0\0\050\0CME\0USD\0ESU5\0ES\0000e1a2b.67890def.02.01\020240125 10:31:00\0DU1234567\0CME\0SLD\05\05050.25\0123457\0100\00\05\05050.25\0\0\00.0\0\00\0".to_string(),
                // CommissionReport message for second execution
                "59\01\0000e1a2b.67890def.02.01\02.50\0USD\0125.50\00.0\00\0".to_string(),
                // ExecutionData message - options execution (SPY)
                "11\09000\01003\0123456789\0SPY\0OPT\020240126\0450.0\0C\0100\0CBOE\0USD\0SPY240126C00450000\0SPY\0000e1a2b.67890ghi.03.01\020240125 10:32:00\0DU1234567\0CBOE\0BOT\010\02.50\0123458\0100\00\010\02.50\0\0\00.0\0\00\0".to_string(),
                // CommissionReport message for third execution
                "59\01\0000e1a2b.67890ghi.03.01\00.65\0USD\0250.00\00.0\00\0".to_string(),
                // ExecutionDataEnd message
                "55\01\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_exercise_options() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add interaction for ExerciseOptions message
        // Using order_id 90 to match the next_order_id from client
        gateway.add_interaction(
            OutgoingMessages::ExerciseOptions,
            vec![
                // OrderStatus message - Option exercise submitted
                // Fields: type(3), order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id, last_fill_price, client_id, why_held, mkt_cap_price
                "3\090\0PreSubmitted\00\010\00.0\0123456\00\00.0\0100\0\00\0".to_string(),
                // OpenOrder message - Option exercise order
                // Fields: Similar to place_order but for an option contract being exercised
                // contract_id=123456789 for SPY option, security_type=OPT, right=C (Call), strike=450.0
                "5\090\0123456789\0SPY\0OPT\020240126\0450.0\0C\0100\0CBOE\0USD\0SPY240126C00450000\0SPY\0BUY\010\0EXERCISE\00.0\00.0\0DAY\0\0DU1234567\0\00\0\0100\01377295700\00\00\00\0\01377295700.0/DU1234567/100\0\0\0\0\0\0\0\0\0\00\0\0-1\00\0\0\0\0\0\02147483647\00\00\00\0\03\00\00\0\00\00\0\00\0None\0\00\0\0\0\0?\00\00\0\00\00\0\0\0\0\0\00\00\00\02147483647\02147483647\0\0\00\0\0IB\00\00\0\00\00\0PreSubmitted\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\0\0\0\0\0\00\00\00\0None\01.7976931348623157E308\02.0\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\01.7976931348623157E308\00\0\0\0\00\01\00\00\00\0\0\00\0\0\0\0\0\0".to_string(),
                // OrderStatus message - Option exercise in progress
                "3\090\0Submitted\00\010\00.0\0123456\00\00.0\0100\0\00\0".to_string(),
                // OrderStatus message - Option exercise filled
                "3\090\0Filled\010\00\00.0\0123456\00\00.0\0100\0\00\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    // === Market Data Setup Functions ===

    pub fn setup_market_data() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestMarketData,
            vec![
                // TickPrice - Bid price
                // type(1), version(6), request_id, tick_type(1=bid), price, size(deprecated), attrib_mask
                "1\06\09000\01\0150.50\00\01\0".to_string(),
                // TickSize - Bid size
                // type(2), version(6), request_id, tick_type(0=bid_size), size
                "2\06\09000\00\0100\0".to_string(),
                // TickPrice - Ask price
                // type(1), version(6), request_id, tick_type(2=ask), price, size(deprecated), attrib_mask
                "1\06\09000\02\0151.00\00\00\0".to_string(),
                // TickSize - Ask size
                // type(2), version(6), request_id, tick_type(3=ask_size), size
                "2\06\09000\03\0200\0".to_string(),
                // TickPrice - Last price
                // type(1), version(6), request_id, tick_type(4=last), price, size(deprecated), attrib_mask
                "1\06\09000\04\0150.75\00\00\0".to_string(),
                // TickSize - Last size
                // type(2), version(6), request_id, tick_type(5=last_size), size
                "2\06\09000\05\050\0".to_string(),
                // TickString - Last timestamp
                // type(46), version(6), request_id, tick_type(45=last_timestamp), value
                "46\06\09000\045\01705500000\0".to_string(),
                // TickGeneric - Volume
                // type(45), version(6), request_id, tick_type(8=volume), value
                "45\06\09000\08\01500000\0".to_string(),
                // TickSnapshotEnd - for snapshot requests
                // type(17), version(1), request_id
                "17\01\09000\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_realtime_bars() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestRealTimeBars,
            vec![
                // RealTimeBars message - 5 second bar
                // type(50), version(3), request_id, time, open, high, low, close, volume, wap, count
                "50\03\09000\01705500000\0150.25\0150.75\0150.00\0150.50\01000\0150.40\025\0".to_string(),
                // Another bar after 5 seconds
                "50\03\09000\01705500005\0150.50\0151.00\0150.40\0150.90\01200\0150.70\030\0".to_string(),
                // Third bar
                "50\03\09000\01705500010\0150.90\0151.25\0150.85\0151.20\0800\0151.05\020\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_tick_by_tick_last() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::TICK_BY_TICK);

        gateway.add_interaction(
            OutgoingMessages::RequestTickByTickData,
            vec![
                // TickByTick message - Last trade
                // type(99), request_id, tick_type(1=Last, 2=AllLast), time, price, size,
                // tick_attrib_last(mask), exchange, special_conditions
                "99\09000\01\01705500000\0150.75\0100\00\0NASDAQ\0\0".to_string(),
                // Another Last trade
                "99\09000\01\01705500001\0150.80\050\02\0NYSE\0\0".to_string(),
                // Third Last trade
                "99\09000\01\01705500002\0150.70\0150\00\0NASDAQ\0\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_tick_by_tick_all_last() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::TICK_BY_TICK);

        gateway.add_interaction(
            OutgoingMessages::RequestTickByTickData,
            vec![
                // TickByTick message - AllLast trade (includes all trades)
                // type(99), request_id, tick_type(2=AllLast), time, price, size,
                // tick_attrib_last(mask), exchange, special_conditions
                "99\09000\02\01705500000\0150.75\0100\00\0NASDAQ\0\0".to_string(),
                // Another AllLast trade (unreported trade)
                "99\09000\02\01705500001\0150.80\050\02\0DARK\0ISO\0".to_string(),
                // Third AllLast trade
                "99\09000\02\01705500002\0150.70\0150\00\0NYSE\0\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_tick_by_tick_bid_ask() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::TICK_BY_TICK);

        gateway.add_interaction(
            OutgoingMessages::RequestTickByTickData,
            vec![
                // TickByTick message - BidAsk
                // type(99), request_id, tick_type(3=BidAsk), time, bid_price, ask_price,
                // bid_size, ask_size, tick_attrib_bid_ask(mask)
                "99\09000\03\01705500000\0150.50\0150.55\0100\0200\00\0".to_string(),
                // BidAsk update - bid past low
                "99\09000\03\01705500001\0150.45\0150.55\0150\0200\01\0".to_string(),
                // BidAsk update - ask past high
                "99\09000\03\01705500002\0150.45\0150.60\0150\0100\02\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_tick_by_tick_midpoint() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::TICK_BY_TICK);

        gateway.add_interaction(
            OutgoingMessages::RequestTickByTickData,
            vec![
                // TickByTick message - MidPoint
                // type(99), request_id, tick_type(4=MidPoint), time, midpoint
                "99\09000\04\01705500000\0150.525\0".to_string(),
                // MidPoint update
                "99\09000\04\01705500001\0150.50\0".to_string(),
                // Another MidPoint update
                "99\09000\04\01705500002\0150.525\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_market_depth() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        gateway.add_interaction(
            OutgoingMessages::RequestMarketDepth,
            vec![
                // MarketDepth message type 12 version 1 (for is_smart_depth=false)
                // type(12), version(1), request_id(9000), position(0),
                // operation(0=insert), side(1=bid), price, size
                "12\01\09000\00\00\01\0150.50\0100\0".to_string(),
                // MarketDepth v1 - Ask insert at position 0
                "12\01\09000\00\00\00\0150.55\0200\0".to_string(),
                // MarketDepth v1 - Bid update at position 0
                "12\01\09000\00\01\01\0150.49\0150\0".to_string(),
                // MarketDepth v1 - Ask delete at position 0
                "12\01\09000\00\02\00\0150.55\00\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_market_depth_exchanges() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::SERVICE_DATA_TYPE);

        gateway.add_interaction(
            OutgoingMessages::RequestMktDepthExchanges,
            vec![
                // MktDepthExchanges message
                // type(80), number_of_descriptions(3), then for each:
                // exchange, sec_type, listing_exchange, service_data_type, agg_group
                "80\03\0ISLAND\0STK\0NASDAQ\0Deep2\01\0NYSE\0STK\0NYSE\0Deep\02\0ARCA\0STK\0NYSE\0Deep\02\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_switch_market_data_type() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::REQ_MARKET_DATA_TYPE);

        // Note: switch_market_data_type doesn't return a response, it's a fire-and-forget request
        // We just need to ensure the gateway accepts the request without error
        gateway.add_interaction(
            OutgoingMessages::RequestMarketDataType,
            vec![], // No response expected
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    // === Historical Data Test Setup Functions ===

    pub fn setup_head_timestamp() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::PEGBEST_PEGMID_OFFSETS);

        // Add response for head timestamp request
        gateway.add_interaction(
            OutgoingMessages::RequestHeadTimestamp,
            vec![
                "88\09000\01705311000\0".to_string(), // HeadTimestamp message: type=88, request_id=9000, timestamp=2024-01-15 09:30:00 UTC
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_historical_data() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for historical data request
        gateway.add_interaction(
            OutgoingMessages::RequestHistoricalData,
            vec![
                // Message type=17, request_id=9000, start_date, end_date, bar_count=3, then bars
                "17\09000\020240122  09:30:00\020240122  09:45:00\03\01705398600\0150.25\0150.75\0150.00\0150.50\01000\0150.40\025\01705398900\0150.50\0151.00\0150.40\0150.90\01200\0150.70\030\01705399200\0150.90\0151.25\0150.85\0151.20\01500\0151.05\035\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_historical_schedules() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for historical schedules request
        gateway.add_interaction(
            OutgoingMessages::RequestHistoricalData,
            vec![
                // Message type=106 (HistoricalSchedule), request_id=9000, start, end, timezone, session_count=1, then session data
                "106\09000\020240122-09:30:00\020240122-16:00:00\0US/Eastern\01\020240122-09:30:00\020240122-16:00:00\020240122\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_historical_ticks_bid_ask() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for historical ticks bid/ask request
        gateway.add_interaction(
            OutgoingMessages::RequestHistoricalTicks,
            vec![
                // Message type=97 (HistoricalTickBidAsk), request_id=9000, number_of_ticks=3, then for each tick: timestamp, mask, priceBid, priceAsk, sizeBid, sizeAsk, then done flag
                "97\09000\03\01705920600\00\0150.25\0150.50\0100\0200\01705920605\00\0150.30\0150.55\0150\0250\01705920610\00\0150.35\0150.60\0200\0300\01\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_historical_ticks_mid_point() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for historical ticks midpoint request
        gateway.add_interaction(
            OutgoingMessages::RequestHistoricalTicks,
            vec![
                // Message type=96 (HistoricalTick/Midpoint), request_id=9000, number_of_ticks=3, then for each tick: timestamp, skip_field, price, size (always 0), then done flag
                "96\09000\03\01705920600\00\0150.375\00\01705920605\00\0150.425\00\01705920610\00\0150.475\00\01\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_historical_ticks_trade() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for historical ticks trade request
        gateway.add_interaction(
            OutgoingMessages::RequestHistoricalTicks,
            vec![
                // Message type=98 (HistoricalTickLast/Trade), request_id=9000, number_of_ticks=3, then for each tick: timestamp, mask, price, size, exchange, specialConditions, then done flag
                "98\09000\03\01705920600\00\0150.50\0100\0NASDAQ\0T\01705920605\00\0150.55\0200\0NYSE\0\01705920610\00\0150.60\0150\0NASDAQ\0\01\0"
                    .to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_histogram_data() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for histogram data request
        gateway.add_interaction(
            OutgoingMessages::RequestHistogramData,
            vec![
                // Message type=89, request_id=9000, count=3, then for each entry: price, size
                "89\09000\03\0150.00\01000\0150.50\01500\0151.00\0800\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    // === News Test Setup Functions ===

    pub fn setup_news_providers() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::REQ_NEWS_PROVIDERS);

        // Add response for news providers request
        gateway.add_interaction(
            OutgoingMessages::RequestNewsProviders,
            vec![
                // NewsProviders message: type=85, num_providers=3, then for each provider: code, name
                "85\03\0BRFG\0Briefing.com General Market Columns\0BRFUPDN\0Briefing.com Analyst Actions\0DJ-RT\0Dow Jones Real-Time News\0"
                    .to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_news_bulletins() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for news bulletins request
        gateway.add_interaction(
            OutgoingMessages::RequestNewsBulletins,
            vec![
                // NewsBulletins message: type=14, version=1, message_id, message_type, message, exchange
                "14\01\0123\01\0Important market announcement\0NYSE\0".to_string(),
                "14\01\0124\02\0Trading halt on symbol XYZ\0NASDAQ\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_historical_news() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::REQ_HISTORICAL_NEWS);

        // Add response for historical news request
        gateway.add_interaction(
            OutgoingMessages::RequestHistoricalNews,
            vec![
                // HistoricalNews message: type=86, request_id=9000, time, provider_code, article_id, headline
                "86\09000\02024-01-15 14:30:00.000\0DJ-RT\0DJ001234\0Market hits new highs amid positive earnings\0".to_string(),
                "86\09000\02024-01-15 14:25:00.000\0BRFG\0BRF5678\0Federal Reserve announces policy decision\0".to_string(),
                // HistoricalNewsEnd message: type=87, request_id=9000, has_more=false
                "87\09000\00\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_news_article() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::REQ_NEWS_ARTICLE);

        // Add response for news article request
        gateway.add_interaction(
            OutgoingMessages::RequestNewsArticle,
            vec![
                // NewsArticle message: type=83, request_id=9000, article_type=0 (text), article_text
                "83\09000\00\0This is the full text of the news article. It contains detailed information about the market event described in the headline.\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_scanner_parameters() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::IPO_PRICES);

        // Add response for scanner parameters request
        gateway.add_interaction(
            OutgoingMessages::RequestScannerParameters,
            vec![
                // ScannerParameters message: type=19, version=1, xml_content
                "19\01\0<ScanParameterResponse><InstrumentList><Instrument>STK</Instrument><Instrument>OPT</Instrument></InstrumentList><LocationTree><Location>US</Location></LocationTree><ScanTypeList><ScanType>TOP_PERC_GAIN</ScanType></ScanTypeList></ScanParameterResponse>\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_scanner_subscription() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::SCANNER_GENERIC_OPTS);

        // Add response for scanner subscription request
        gateway.add_interaction(
            OutgoingMessages::RequestScannerSubscription,
            vec![
                // ScannerData message: type=20, version, request_id=9000, number_of_items=2
                "20\03\09000\02\0".to_string() +
                // rank, contract_id, symbol, sec_type, expiry, strike, right, exchange, currency, local_symbol, market_name, trading_class
                "1\01234\0AAPL\0STK\0\00.0\0\0SMART\0USD\0AAPL\0NMS\0AAPL\0" +
                // distance, benchmark, projection, legs
                "\0\0\0\0" +
                // rank 2
                "2\05678\0GOOGL\0STK\0\00.0\0\0SMART\0USD\0GOOGL\0NMS\0GOOGL\0" +
                "\0\0\0\0",
                // ScannerDataEnd
                "20\03\09000\0-1\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_wsh_metadata() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::WSHE_CALENDAR);

        // Add response for WSH metadata request
        gateway.add_interaction(
            OutgoingMessages::RequestWshMetaData,
            vec![
                // WshMetaData message: type=104, request_id=9000, data_json
                "104\09000\0{\"dataJson\":\"sample_metadata\"}\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_wsh_event_data() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::WSHE_CALENDAR);

        // Add response for WSH event data request
        gateway.add_interaction(
            OutgoingMessages::RequestWshEventData,
            vec![
                // WshEventData message: type=105, request_id=9000, data_json, end_flag
                "105\09000\0{\"dataJson\":\"event1\"}\0".to_string(),
                "105\09000\0{\"dataJson\":\"event2\"}\0".to_string(),
                "105\09000\0\01\0".to_string(), // end flag = true
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_contract_news() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::REQ_NEWS_ARTICLE);

        // Add response for contract news request (via market data with news ticks)
        gateway.add_interaction(
            OutgoingMessages::RequestMarketData,
            vec![
                // TickNews message: type=84, request_id=9000, time, provider_code, article_id, headline, extra_data
                "84\09000\01705920600000\0DJ-RT\0DJ001234\0Stock rises on earnings beat\0extraData1\0".to_string(),
                "84\09000\01705920700000\0BRFG\0BRF5678\0Company announces expansion\0extraData2\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_broad_tape_news() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::REQ_NEWS_ARTICLE);

        // Add response for broad tape news request (via market data with news ticks)
        gateway.add_interaction(
            OutgoingMessages::RequestMarketData,
            vec![
                // TickNews message: type=84, request_id=9000, time, provider_code, article_id, headline, extra_data
                "84\09000\01705920600000\0BRFG\0BRF001\0Market update: Tech sector rallies\0extraData1\0".to_string(),
                "84\09000\01705920700000\0BRFG\0BRF002\0Fed minutes released\0extraData2\0".to_string(),
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }

    pub fn setup_wsh_event_data_by_filter() -> MockGateway {
        let mut gateway = MockGateway::new(server_versions::WSH_EVENT_DATA_FILTERS);

        // Add response for WSH event data by filter request
        gateway.add_interaction(
            OutgoingMessages::RequestWshEventData,
            vec![
                // WshEventData message: type=105, request_id=9000, data_json, end_flag
                "105\09000\0{\"dataJson\":\"filtered_event1\"}\0".to_string(),
                "105\09000\0{\"dataJson\":\"filtered_event2\"}\0".to_string(),
                "105\09000\0\01\0".to_string(), // end flag = true
            ],
        );

        gateway.start().expect("Failed to start mock gateway");
        gateway
    }
}
