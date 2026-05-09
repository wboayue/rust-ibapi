use super::*;
use time::macros::date;

#[test]
fn bool_to_field_renders_one_or_zero() {
    assert_eq!(true.to_field(), "1");
    assert_eq!(false.to_field(), "0");
}

#[test]
fn string_to_field_clones_value() {
    let s = String::from("AAPL");
    assert_eq!(s.to_field(), "AAPL");
}

#[test]
fn str_to_field_round_trips() {
    let s: &str = "MSFT";
    assert_eq!(s.to_field(), "MSFT");
}

#[test]
fn usize_to_field_decimal() {
    let n: usize = 42;
    assert_eq!(n.to_field(), "42");
}

#[test]
fn i32_to_field_handles_negatives() {
    let n: i32 = -7;
    assert_eq!(n.to_field(), "-7");
    assert_eq!(0_i32.to_field(), "0");
}

#[test]
fn f64_to_field_decimal() {
    let n: f64 = 1.5;
    assert_eq!(n.to_field(), "1.5");
}

#[test]
fn option_string_some_emits_value_none_emits_empty() {
    assert_eq!(Some(String::from("X")).to_field(), "X");
    let none: Option<String> = None;
    assert_eq!(none.to_field(), "");
}

#[test]
fn option_str_some_emits_value_none_emits_empty() {
    assert_eq!(Some("Y").to_field(), "Y");
    let none: Option<&str> = None;
    assert_eq!(none.to_field(), "");
}

#[test]
fn option_i32_some_emits_value_none_emits_empty() {
    assert_eq!(Some(123_i32).to_field(), "123");
    let none: Option<i32> = None;
    assert_eq!(none.to_field(), "");
}

#[test]
fn option_f64_some_emits_value_none_emits_empty() {
    assert_eq!(Some(2.5_f64).to_field(), "2.5");
    let none: Option<f64> = None;
    assert_eq!(none.to_field(), "");
}

#[test]
fn date_to_field_uses_yyyymmdd() {
    let d = date!(2025 - 03 - 14);
    assert_eq!(d.to_field(), "20250314");
}

#[test]
fn option_date_some_emits_yyyymmdd_none_emits_empty() {
    assert_eq!(Some(date!(2024 - 12 - 31)).to_field(), "20241231");
    let none: Option<Date> = None;
    assert_eq!(none.to_field(), "");
}

#[test]
fn encode_option_field_delegates_to_inner() {
    let some = Some(true);
    assert_eq!(encode_option_field(&some), "1");
    let none: Option<bool> = None;
    assert_eq!(encode_option_field(&none), "");
}
