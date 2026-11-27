use super::*;
use std::fmt::Display;

// The conversion set docs/rules/style/id-parameters.md requires of every id newtype.
fn integer_id<T: From<i32> + for<'a> From<&'a i32> + Into<i32> + Display>() {}
fn string_id<T: From<String> + for<'a> From<&'a String> + for<'a> From<&'a str> + Into<String> + Display>() {}

#[test]
fn id_newtypes_convert_both_ways() {
    integer_id::<OrderId>();
    integer_id::<ContractId>();
    string_id::<AccountId>();
    string_id::<ModelCode>();
}
