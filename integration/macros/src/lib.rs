use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use time::Weekday;
use time_tz::{timezones::db::america::NEW_YORK, OffsetDateTimeExt};

fn us_market_open() -> bool {
    let now = time::OffsetDateTime::now_utc().to_timezone(NEW_YORK);
    let day = now.weekday();
    if day == Weekday::Saturday || day == Weekday::Sunday {
        return false;
    }
    let open = time::Time::from_hms(9, 30, 0).unwrap();
    let close = time::Time::from_hms(16, 0, 0).unwrap();
    now.time() >= open && now.time() < close
}

/// Marks the test as `#[ignore]` if US equity markets are closed (outside
/// Mon-Fri 9:30-16:00 Eastern). The check runs at compile time.
///
/// ```ignore
/// #[test]
/// #[require_market_open]
/// fn my_test() { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn require_market_open(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    if us_market_open() {
        quote! { #func }.into()
    } else {
        quote! {
            #[ignore = "US equity market is closed"]
            #func
        }
        .into()
    }
}
