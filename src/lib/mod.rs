//! Meshtastic URL encoder and decoder library.
//!
//! Provides functionality to encode and decode Meshtastic channel configuration URLs.
//!
//! # Quick Start
//!
//! ```rust
//! use meshurl::{decode_url, encode_url};
//!
//! // Decode a URL
//! let config = decode_url("https://meshtastic.org/e/#CgMSAQ...").unwrap();
//! for channel in config.channels {
//!     println!("Channel: {}", channel.name);
//! }
//!
//! // Encode a configuration
//! // (create a MeshtasticConfig and encode it)
//! ```

pub mod decoder;
pub mod encoder;
pub mod errors;
pub mod models;

pub use decoder::decode_url;
pub use encoder::{
    encode_url, encode_url_short, modem_preset_from_str, region_code_from_str, ModemPreset,
    RegionCode,
};
pub use errors::{DecodeError, EncodeError};
pub use models::{
    get_preset_params, ChannelInfo, ChannelRole, LoRaInfo, MeshtasticConfig, PskMode, PskType,
    DEFAULT_PSK, POSITION_OPTIONS,
};
