// prost output: lints off for the generated code only, so the hand-written
// converters below are still linted.
#[allow(clippy::all, dead_code)]
mod generated {
    include!("protobuf.rs");
}
pub(crate) use generated::*;

pub(crate) mod decoders;

pub(crate) mod payload;

pub(crate) mod encoders;
