//! Crate-internal macros for shape-identical trait impls.
//!
//! Reachable crate-wide via `#[macro_use] mod macros;` in `lib.rs`. Not
//! exported to the public API.

/// Mirrors std `String`'s `PartialEq` ergonomics on a string-newtype:
/// `wrapper == "literal"` and `"literal" == wrapper` both work.
macro_rules! impl_str_partial_eq {
    ($t:ty) => {
        impl PartialEq<str> for $t {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
        impl PartialEq<&str> for $t {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
        impl PartialEq<$t> for str {
            fn eq(&self, other: &$t) -> bool {
                self == other.0
            }
        }
        impl PartialEq<$t> for &str {
            fn eq(&self, other: &$t) -> bool {
                *self == other.0
            }
        }
    };
}

/// Generate `Display` / `FromStr<Err = Error>` / `ToField` impls from
/// hand-written `as_str(&self) -> &'static str` + `from_wire(&str) -> Option<Self>`
/// methods. The data tables stay in normal Rust (visible to goto-def); only
/// the boilerplate plumbing — `Display` via `as_str`, `FromStr` via `from_wire`
/// with canonical `Error::Parse`, `ToField` via `Display` — runs through the
/// macro. Orphan rule blocks a blanket `impl<T: WireEnum> Display`, so a
/// macro is the only viable shape.
macro_rules! impl_wire_enum {
    ($name:ident) => {
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl ::std::str::FromStr for $name {
            type Err = $crate::Error;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                Self::from_wire(s).ok_or_else(|| $crate::Error::Parse(0, s.to_string(), concat!("unknown ", stringify!($name)).into()))
            }
        }
        impl $crate::ToField for $name {
            fn to_field(&self) -> String {
                self.to_string()
            }
        }
    };
}
