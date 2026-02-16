use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Skips the test if US equity markets are closed.
///
/// ```ignore
/// #[test]
/// #[require_market_open]
/// fn my_test() { /* ... */ }
/// ```
#[proc_macro_attribute]
pub fn require_market_open(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);

    let check = syn::parse_quote! {
        if !ibapi_test::us_market_open() {
            eprintln!("SKIPPED: US equity market is closed");
            return;
        }
    };

    func.block.stmts.insert(0, check);

    quote! { #func }.into()
}
