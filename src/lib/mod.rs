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
pub mod protobufs;
pub mod regions;

pub use decoder::{DecodeResult, decode_url};
pub use encoder::{
    ModemPreset, RegionCode, encode_url, encode_url_short, modem_preset_from_str,
    region_code_from_str,
};
pub use errors::{DecodeError, EncodeError};
pub use models::{
    CUSTOM_MODEM_NAME, ChannelInfo, ChannelRole, DEFAULT_PSK, LoRaInfo, MAX_CHANNEL_NAME_BYTES,
    MESHTASTIC_CHANNEL_URL_BASE, MESHTASTIC_NODE_URL_BASE, MODEM_PRESETS, MeshtasticConfig,
    MeshtasticDisplay, ModemConfig, NodeInfo, POSITION_OPTIONS, PskMode, PskType, REGION_CODES,
    generate_random_psk, get_preset_params, hash_phrase_to_psk, validate_channel_name,
};
pub use regions::{
    default_preset_for_region, presets_for_region, region_supports_preset, region_swap_for_preset,
};
