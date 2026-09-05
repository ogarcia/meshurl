//! Meshtastic protobuf types, generated at build time from `proto/`.
//!
//! See `proto/README.md` for which definitions are vendored and why.

#![allow(clippy::all, missing_docs)]

include!(concat!(env!("OUT_DIR"), "/meshtastic.rs"));
