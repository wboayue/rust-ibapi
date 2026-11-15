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
///
/// Both forms distinguish the two ways a wire value can be unusable: empty
/// input is `"empty $name"` (no value at all) and an unrecognized non-empty
/// value is `"unknown $name"`. `parse_required` reaches the first only when a
/// caller parses directly — it intercepts empty with its own `"missing
/// {label}"`, which names the field rather than the type.
///
/// The `fallback $variant` form is for open enums on inbound stream paths,
/// where an unrecognized value must not terminate the subscription: a
/// non-empty string that `from_wire` does not recognize parses as
/// `$name::$variant(raw)` instead of `Error::Parse`. Empty input is still
/// rejected — absence of a value stays an error (see
/// `docs/rules/wire/enum-typing.md`); only an unrecognized *value* is
/// preserved. `as_str` then borrows from `self`, so such enums implement
/// `as_str(&self) -> &str`.
///
/// The `fallback $variant, serde` form also generates serde and utoipa impls
/// that use the wire string in both directions, so `Unknown("X")` round-trips
/// as `"X"` rather than the externally tagged `{"Unknown":"X"}` a derive gives
/// the one payload variant, and JSON stays stable against Rust-side renames.
/// Deserialize goes through `FromStr`, so it rejects `""`; serde is not on the
/// wire path (decoding goes through `parse_required` / `parse_optional`). The
/// utoipa schema is exactly a string, because the derive would mirror serde's
/// default representation, which is not what these impls emit.
macro_rules! impl_wire_enum {
    ($name:ident) => {
        impl_wire_enum!(@common $name);
        impl ::std::str::FromStr for $name {
            type Err = $crate::Error;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                if s.is_empty() {
                    return Err($crate::Error::Parse(0, s.to_string(), concat!("empty ", stringify!($name)).into()));
                }
                Self::from_wire(s).ok_or_else(|| $crate::Error::Parse(0, s.to_string(), concat!("unknown ", stringify!($name)).into()))
            }
        }
    };
    ($name:ident, fallback $variant:ident) => {
        impl_wire_enum!(@common $name);
        impl ::std::str::FromStr for $name {
            type Err = $crate::Error;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                if s.is_empty() {
                    return Err($crate::Error::Parse(0, s.to_string(), concat!("empty ", stringify!($name)).into()));
                }
                Ok(Self::from_wire(s).unwrap_or_else(|| $name::$variant(s.to_string())))
            }
        }
    };
    ($name:ident, fallback $variant:ident, serde) => {
        impl_wire_enum!($name, fallback $variant);

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                <String as ::serde::Deserialize>::deserialize(deserializer)?
                    .parse()
                    .map_err(::serde::de::Error::custom)
            }
        }

        #[cfg(feature = "utoipa")]
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema> {
                <String as ::utoipa::PartialSchema>::schema()
            }
        }

        #[cfg(feature = "utoipa")]
        impl ::utoipa::ToSchema for $name {}
    };
    (@common $name:ident) => {
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl $crate::ToField for $name {
            fn to_field(&self) -> String {
                self.to_string()
            }
        }
    };
}

/// Generates both `From<i32> for $name` and `From<$name> for i32` for an
/// integer-coded wire enum from one `Variant = code` table.
///
/// The enum must carry an `Unknown(i32)` variant: `From<i32>` is total,
/// returning `Unknown(code)` for a code outside the table, and
/// `From<$name> for i32` hands that code back. One table means the two
/// directions cannot disagree; a variant missing from it is a
/// non-exhaustive-match error, and a duplicated code an unreachable-pattern
/// warning. See `docs/rules/wire/enum-typing.md`.
macro_rules! impl_wire_code {
    ($name:ident { $($variant:ident = $code:literal),+ $(,)? }) => {
        impl From<$name> for i32 {
            fn from(value: $name) -> i32 {
                match value {
                    $($name::$variant => $code,)+
                    $name::Unknown(code) => code,
                }
            }
        }

        impl From<i32> for $name {
            fn from(value: i32) -> Self {
                match value {
                    $($code => $name::$variant,)+
                    code => $name::Unknown(code),
                }
            }
        }
    };
}

#[cfg(test)]
#[path = "macros_tests.rs"]
mod tests;
