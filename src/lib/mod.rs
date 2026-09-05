//! Meshtastic URL encoder and decoder library.
//!
//! Provides functionality to encode and decode Meshtastic channel configuration URLs.
//!
//! # Quick Start
//!
//! ```rust
//! use meshurl::{decode_url, DecodeResult};
//!
//! // Decode a URL (supports both channel and node URLs)
//! let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
//! match decode_url(url).expect("valid URL") {
//!     DecodeResult::Channel(config) => {
//!         for channel in config.channels {
//!             println!("Channel: {}", channel.name);
//!         }
//!     }
//!     DecodeResult::Node(node) => {
//!         println!("Node: {} ({})", node.long_name, node.short_name);
//!     }
//! }
//!
//! // Encode a configuration
//! // (create a MeshtasticConfig and encode it)
//! ```

pub mod decoder;
pub mod encoder;
pub mod errors;
pub mod models;

pub use decoder::{DecodeResult, decode_url};
pub use encoder::{
    ModemPreset, RegionCode, encode_url, encode_url_short, modem_preset_from_str,
    region_code_from_str,
};
pub use errors::{DecodeError, EncodeError};
pub use models::{
    ChannelInfo, ChannelRole, DEFAULT_PSK, LoRaInfo, MESHTASTIC_CHANNEL_URL_BASE,
    MESHTASTIC_NODE_URL_BASE, MODEM_PRESETS, MeshtasticConfig, MeshtasticDisplay, NodeInfo,
    POSITION_OPTIONS, PskMode, PskType, REGION_CODES, generate_random_psk, get_preset_params,
    hash_phrase_to_psk,
};
