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
                        for response in interaction.responses.iter() {
                            self.write_message(&mut stream, response.clone())?;
                        }
                        self.current_interaction += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

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
            assert_eq!(message, "71\02\0100\0\0");

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
}
