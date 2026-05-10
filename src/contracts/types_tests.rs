use super::*;

#[test]
fn symbol_partial_eq_str_round_trip() {
    let s = Symbol::from("AAPL");

    assert_eq!(s, *"AAPL");
    assert_eq!(s, "AAPL");
    assert_eq!(*"AAPL", s);
    assert_eq!("AAPL", s);

    assert_ne!(s, "MSFT");
    assert_ne!("MSFT", s);
}

#[test]
fn exchange_partial_eq_str_round_trip() {
    let e = Exchange::from("NASDAQ");

    assert_eq!(e, *"NASDAQ");
    assert_eq!(e, "NASDAQ");
    assert_eq!(*"NASDAQ", e);
    assert_eq!("NASDAQ", e);

    assert_ne!(e, "NYSE");
    assert_ne!("NYSE", e);
}

#[test]
fn currency_partial_eq_str_round_trip() {
    let c = Currency::from("USD");

    assert_eq!(c, *"USD");
    assert_eq!(c, "USD");
    assert_eq!(*"USD", c);
    assert_eq!("USD", c);

    assert_ne!(c, "EUR");
    assert_ne!("EUR", c);
}
