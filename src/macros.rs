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
/// Every form distinguishes the two ways a wire value can be unusable: empty
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
/// The `$name { .. }` and `fallback $variant` forms also generate serde and utoipa impls that use the wire string
/// in both directions, so JSON matches what TWS sends and stays stable against
/// Rust-side renames, and an open enum's `Unknown("X")` round-trips as `"X"`
/// rather than the externally tagged `{"Unknown":"X"}` a derive gives the one
/// payload variant. Deserialize goes through `FromStr`, so it rejects `""`, and
/// a closed enum rejects a string it does not list; serde is not on the wire
/// path (decoding goes through `parse_required` / `parse_optional`). The
/// utoipa schema is a string, because the derive would mirror serde's default
/// representation, which is not what these impls emit. An enum built
/// with this macro must not derive `Serialize`, `Deserialize` or `ToSchema`;
/// the impls would conflict.
///
/// The `$name, integer { .. }` form is for a closed enum whose wire value is an
/// integer code but whose `as_str` table spells it as text (`FaDataType`,
/// `ServerLogLevel`): serde and the utoipa schema use the integer, read back
/// through the same `from_wire` table, so a code it does not list is an
/// error. Both directions read that one table.
///
/// A closed form lists every variant (`impl_wire_enum!(Action { Buy, Sell, .. })`),
/// and its schema enumerates their serialized values; see
/// `impl_wire_string_serde!`'s `@closed_schema` arm. The schema describes the
/// serialized form only, so an alias `FromStr` also accepts is not in it. An
/// open enum's schema stays unconstrained, since it accepts values it does not
/// list.
macro_rules! impl_wire_enum {
    ($name:ident { $($v:ident),+ $(,)? }) => {
        impl_wire_enum!(@closed $name);
        impl_wire_string_serde!($name { $($v),+ });
    };
    ($name:ident as $schema:literal { $($v:ident),+ $(,)? }) => {
        impl_wire_enum!(@closed $name);
        impl_wire_string_serde!($name as $schema { $($v),+ });
    };
    ($name:ident, integer { $($v:ident),+ $(,)? }) => {
        impl_wire_enum!(@closed $name);

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                let code = self.as_str().parse::<i32>().map_err(::serde::ser::Error::custom)?;
                serializer.serialize_i32(code)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                let code = <i32 as ::serde::Deserialize>::deserialize(deserializer)?;
                code.to_string().parse().map_err(::serde::de::Error::custom)
            }
        }

        impl_wire_string_serde!(@closed_schema $name, i32, stringify!($name), [$($v),+]);
    };
    (@closed $name:ident) => {
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
        impl_wire_string_serde!($name);
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

/// Serde and utoipa impls for a wire-string enum: Serialize writes `Display`,
/// Deserialize goes through `FromStr`, and the utoipa schema is a string.
/// `impl_wire_enum!` invokes it for every string-form enum it builds; an enum with
/// hand-written `Display` / `FromStr` that round-trip the wire string
/// (historical `WhatToShow`, `BarSize`) invokes it directly.
///
/// `$name` alone is the open form: the schema is any string. `$name { .. }`
/// is the closed form, listing every variant: the schema's `enum` holds each
/// one's serialized value. The list is checked by an exhaustive `match`, so a
/// variant missing from it fails to compile. The schema's OpenAPI component
/// name is the type name; `$name as "Name" { .. }` overrides it, for two types
/// that share a name (`WhatToShow`) but not a code list. The override form
/// exists on `impl_wire_enum!` too.
macro_rules! impl_wire_string_serde {
    ($name:ident) => {
        impl_wire_string_serde!(@serde $name);

        #[cfg(feature = "utoipa")]
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema> {
                <String as ::utoipa::PartialSchema>::schema()
            }
        }

        #[cfg(feature = "utoipa")]
        impl ::utoipa::ToSchema for $name {}
    };
    ($name:ident { $($v:ident),+ $(,)? }) => {
        impl_wire_string_serde!(@serde $name);
        impl_wire_string_serde!(@closed_schema $name, String, stringify!($name), [$($v),+]);
    };
    ($name:ident as $schema:literal { $($v:ident),+ $(,)? }) => {
        impl_wire_string_serde!(@serde $name);
        impl_wire_string_serde!(@closed_schema $name, String, $schema, [$($v),+]);
    };
    (@serde $name:ident) => {
        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                <String as ::serde::Deserialize>::deserialize(deserializer)?
                    .parse()
                    .map_err(::serde::de::Error::custom)
            }
        }
    };
    // `$base`'s schema (`String` or `i32`) with `enum` set to every listed
    // variant's serialized value, so the schema cannot disagree with serde.
    // `$schema` is the OpenAPI component name.
    (@closed_schema $name:ident, $base:ty, $schema:expr, [$($v:ident),+]) => {
        const _: () = {
            // Never called: fails to compile when the list misses a variant.
            #[allow(dead_code)]
            fn every_variant_listed(value: $name) {
                match value {
                    $($name::$v => {})+
                }
            }
        };

        #[cfg(feature = "utoipa")]
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema> {
                let mut schema = <$base as ::utoipa::PartialSchema>::schema();
                let ::utoipa::openapi::RefOr::T(::utoipa::openapi::schema::Schema::Object(object)) = &mut schema else {
                    unreachable!(concat!(stringify!($base), " schema is an inline object"));
                };
                object.enum_values = Some(vec![$(
                    ::serde_json::to_value($name::$v).expect(concat!(stringify!($name), " serializes")),
                )+]);
                schema
            }
        }

        #[cfg(feature = "utoipa")]
        impl ::utoipa::ToSchema for $name {
            fn name() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed($schema)
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
///
/// It also generates serde and utoipa impls that use the wire code, so the
/// enum serializes as a bare integer (`2`, and `Unknown(7)` as `7`) instead of
/// the derived variant name or the tagged `{"Unknown":7}`, and a field typed
/// from an `i32` keeps its JSON form. Deserialize is total, like `From<i32>`.
/// The utoipa schema is exactly an integer, for the reason given on
/// `impl_wire_enum!`. `TickType`, whose table lives in its own macro, takes
/// the serde half through the `@serde $name` form.
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

        impl_wire_code!(@serde $name);
    };
    (@serde $name:ident) => {
        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                serializer.serialize_i32(i32::from(*self))
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {
                <i32 as ::serde::Deserialize>::deserialize(deserializer).map($name::from)
            }
        }

        #[cfg(feature = "utoipa")]
        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema> {
                <i32 as ::utoipa::PartialSchema>::schema()
            }
        }

        #[cfg(feature = "utoipa")]
        impl ::utoipa::ToSchema for $name {}
    };
}

#[cfg(test)]
#[path = "macros_tests.rs"]
mod tests;
