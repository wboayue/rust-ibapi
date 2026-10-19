//! Text-wire field encoding: `ToField` and its impls for the field value
//! types.

use std::sync::LazyLock;
use time::{
    format_description::{self, BorrowedFormatItem},
    Date,
};

#[cfg(test)]
#[path = "to_field_tests.rs"]
mod tests;

pub(crate) trait ToField {
    fn to_field(&self) -> String;
}

impl ToField for bool {
    fn to_field(&self) -> String {
        if *self {
            String::from("1")
        } else {
            String::from("0")
        }
    }
}

impl ToField for String {
    fn to_field(&self) -> String {
        self.clone()
    }
}

impl ToField for Option<String> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for &str {
    fn to_field(&self) -> String {
        <&str>::clone(self).to_string()
    }
}

impl ToField for Option<&str> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for usize {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for i32 {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<i32> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

impl ToField for f64 {
    fn to_field(&self) -> String {
        self.to_string()
    }
}

impl ToField for Option<f64> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

fn date_format() -> Vec<BorrowedFormatItem<'static>> {
    format_description::parse_borrowed::<2>("[year][month][day]").unwrap()
}

static DATE_FORMAT: LazyLock<Vec<BorrowedFormatItem<'static>>> = LazyLock::new(date_format);

impl ToField for Date {
    fn to_field(&self) -> String {
        self.format(&DATE_FORMAT).unwrap()
    }
}

impl ToField for Option<Date> {
    fn to_field(&self) -> String {
        encode_option_field(self)
    }
}

pub(crate) fn encode_option_field<T: ToField>(val: &Option<T>) -> String {
    match val {
        Some(val) => val.to_field(),
        None => String::from(""),
    }
}
