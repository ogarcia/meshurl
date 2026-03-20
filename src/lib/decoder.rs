//! Meshtastic URL decoder module.
//! Provides functions to decode Meshtastic channel configuration URLs.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use meshtastic_protobufs::meshtastic::ChannelSet;
use prost::Message;

use crate::errors::DecodeError;
use crate::models::{MeshtasticConfig, MESHTASTIC_URL_BASE};

/// Decodes a Meshtastic URL into a MeshtasticConfig.
///
/// # Arguments
/// * `url` - A Meshtastic URL string (either full URL or just the hash part)
///
/// # Returns
/// * `Ok(MeshtasticConfig)` - The decoded configuration
/// * `Err(DecodeError)` - If the URL is invalid or cannot be decoded
///
/// # Example
/// ```
/// use meshurl::decode_url;
///
/// let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
/// let config = decode_url(url).expect("valid URL");
/// for channel in config.channels {
///     println!("Channel: {}", channel.name);
/// }
/// ```
pub fn decode_url(url: &str) -> Result<MeshtasticConfig, DecodeError> {
    let hash_part = extract_hash(url)?;
    let decoded = decode_base64(hash_part)?;
    let channel_set = decode_protobuf(&decoded)?;
    Ok(MeshtasticConfig::from_channel_set(&channel_set))
}

/// Extracts the base64 hash part from a Meshtastic URL.
/// Supports multiple URL formats:
/// - https://meshtastic.org/e/#<hash>
/// - meshtastic.org/e/#<hash>
/// - #<hash>
fn extract_hash(url: &str) -> Result<&str, DecodeError> {
    url.strip_prefix(MESHTASTIC_URL_BASE)
        .or_else(|| url.strip_prefix("meshtastic.org/e/#"))
        .or_else(|| url.strip_prefix("#"))
        .ok_or_else(|| {
            DecodeError::InvalidUrl(format!("Expected format: {}<base64>", MESHTASTIC_URL_BASE))
        })
}

/// Decodes a base64-encoded string into bytes.
fn decode_base64(hash: &str) -> Result<Vec<u8>, DecodeError> {
    URL_SAFE_NO_PAD
        .decode(hash)
        .map_err(|e| DecodeError::Base64Decode(e.to_string()))
}

/// Decodes protobuf bytes into a ChannelSet.
fn decode_protobuf(data: &[u8]) -> Result<ChannelSet, DecodeError> {
    ChannelSet::decode(data).map_err(|e| DecodeError::ProtobufDecode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hash_full_url() {
        let url = "https://meshtastic.org/e/#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = extract_hash(url);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ"));
    }

    #[test]
    fn test_extract_hash_short_url() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = extract_hash(url);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_hash_without_hash() {
        let url = "invalid_url";
        let result = extract_hash(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_invalid_url() {
        let url = "not_a_valid_url";
        let result = decode_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_valid_url_default_channel() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = decode_url(url);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.channels.len(), 3);
        assert_eq!(config.channels[0].name, "");
        assert!(config.lora.is_some());
    }

    #[test]
    fn test_decode_url_with_custom_psk() {
        let url = "#CgwSAQEaB0dhbGljaWE";
        let result = decode_url(url);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.channels.len(), 1);
        assert_eq!(config.channels[0].name, "Galicia");
        assert!(!config.channels[0].psk.is_empty());
    }

    #[test]
    fn test_decode_base64_invalid() {
        let result = decode_base64("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_base64_valid() {
        let result = decode_base64("CgsSAQ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_url_empty_hash() {
        let result = extract_hash("#");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_decode_url_just_prefix() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = decode_url(url);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_url_with_short_channel_name() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = decode_url(url);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.channels.is_empty());
    }

    #[test]
    fn test_decode_url_multiple_channels() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = decode_url(url);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.channels.len() >= 1);
    }

    #[test]
    fn test_decode_url_unicode_channel_name() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = decode_url(url);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.channels.is_empty());
    }

    #[test]
    fn test_extract_hash_various_formats() {
        assert!(extract_hash("#abc").is_ok());
        assert!(extract_hash("https://meshtastic.org/e/#abc").is_ok());
        assert!(extract_hash("").is_err());
        assert!(extract_hash("invalid").is_err());
    }

    #[test]
    fn test_decode_base64_empty() {
        let result = decode_base64("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_decode_base64_padding() {
        assert!(decode_base64("YWJj").is_ok());
        assert!(decode_base64("YWJjZA").is_ok());
        assert!(decode_base64("YWJjZGVm").is_ok());
        assert!(decode_base64("YWJjZGVmZw").is_ok());
    }

    #[test]
    fn test_decode_base64_special_chars() {
        assert!(decode_base64("YWJj").is_ok());
        assert!(decode_base64("AQIDBA").is_ok());
        assert!(decode_base64("YWJjZGVmZw").is_ok());
    }

    #[test]
    fn test_extract_hash_no_hash_returns_error() {
        assert!(extract_hash("https://example.com").is_err());
        assert!(extract_hash("just text").is_err());
    }
}
