//! Meshtastic URL decoder module.
//! Provides functions to decode Meshtastic channel configuration URLs and node info URLs.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use meshtastic_protobufs::meshtastic::{ChannelSet, NodeInfo as PbNodeInfo};
use prost::Message;

use crate::errors::DecodeError;
use crate::models::{
    MeshtasticConfig, NodeInfo, MESHTASTIC_CHANNEL_URL_BASE, MESHTASTIC_NODE_URL_BASE,
};

/// Result of decoding a Meshtastic URL.
/// Can be either channel configuration or node information.
#[derive(Debug, Clone)]
pub enum DecodeResult {
    Channel(MeshtasticConfig),
    Node(NodeInfo),
}

/// Decodes a Meshtastic URL into either channel configuration or node information.
///
/// Automatically detects whether the URL contains channel configuration or node info
/// by attempting to decode as channel first, then as node info.
///
/// # Arguments
/// * `url` - A Meshtastic URL string (either full URL or just the hash part)
///   - Channel URLs: `https://meshtastic.org/e/#<hash>`
///   - Node URLs: `https://meshtastic.org/v/#<hash>`
///   - Short form: `#<hash>`
///
/// # Returns
/// * `Ok(DecodeResult::Channel(...))` - Channel configuration decoded successfully
/// * `Ok(DecodeResult::Node(...))` - Node information decoded successfully
/// * `Err(DecodeError)` - If the URL is invalid or cannot be decoded
///
/// # Example
/// ```
/// use meshurl::decode_url;
///
/// let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
/// match decode_url(url).unwrap() {
///     meshurl::DecodeResult::Channel(config) => {
///         for channel in config.channels {
///             println!("Channel: {}", channel.name);
///         }
///     }
///     meshurl::DecodeResult::Node(node) => {
///         println!("Node: {} ({})", node.long_name, node.short_name);
///     }
/// }
/// ```
pub fn decode_url(url: &str) -> Result<DecodeResult, DecodeError> {
    let hash_part = extract_hash(url)?;
    let decoded = decode_base64(hash_part)?;

    if let Ok(config) = try_decode_as_channel(&decoded) {
        return Ok(DecodeResult::Channel(config));
    }

    if let Ok(node) = try_decode_as_node(&decoded) {
        return Ok(DecodeResult::Node(node));
    }

    Err(DecodeError::InvalidUrl(
        "Unable to decode URL: not a valid Meshtastic channel or node URL".to_string(),
    ))
}

fn try_decode_as_channel(data: &[u8]) -> Result<MeshtasticConfig, DecodeError> {
    let channel_set = ChannelSet::decode(data)
        .map_err(|_| DecodeError::InvalidUrl("Not a valid channel configuration".to_string()))?;
    Ok(MeshtasticConfig::from_channel_set(&channel_set))
}

fn try_decode_as_node(data: &[u8]) -> Result<NodeInfo, DecodeError> {
    let node_pb = PbNodeInfo::decode(data)
        .map_err(|_| DecodeError::InvalidUrl("Not a valid node info".to_string()))?;
    Ok(NodeInfo::from_pb(&node_pb))
}

/// Extracts the base64 hash part from a Meshtastic URL.
/// Supports multiple URL formats:
/// - https://meshtastic.org/e/#<hash> (channel)
/// - https://meshtastic.org/v/#<hash> (node)
/// - meshtastic.org/e/#<hash>
/// - meshtastic.org/v/#<hash>
/// - #<hash>
/// - text#<hash>
/// - <base64> (raw base64 without prefix)
fn extract_hash(url: &str) -> Result<&str, DecodeError> {
    url.strip_prefix(MESHTASTIC_CHANNEL_URL_BASE)
        .or_else(|| url.strip_prefix("meshtastic.org/e/#"))
        .or_else(|| url.strip_prefix(MESHTASTIC_NODE_URL_BASE))
        .or_else(|| url.strip_prefix("meshtastic.org/v/#"))
        .or_else(|| {
            url.contains('#')
                .then(|| url.rsplit('#').next().unwrap_or(url))
        })
        .or_else(|| {
            if !url.starts_with("https://") && !url.starts_with("meshtastic.org") && !url.is_empty()
            {
                Some(url)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            DecodeError::InvalidUrl(format!(
                "Expected format: {} or {} <base64>",
                MESHTASTIC_CHANNEL_URL_BASE, MESHTASTIC_NODE_URL_BASE
            ))
        })
}

/// Decodes a base64-encoded string into bytes.
fn decode_base64(hash: &str) -> Result<Vec<u8>, DecodeError> {
    URL_SAFE_NO_PAD
        .decode(hash)
        .map_err(|e| DecodeError::Base64Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hash_channel_url() {
        let url = "https://meshtastic.org/e/#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w3FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = extract_hash(url);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w3FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ"));
    }

    #[test]
    fn test_extract_hash_node_url() {
        let url =
            "https://meshtastic.org/v/#EhgSEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJkaDA1Q89kZFRDn_voYAA";
        let result = extract_hash(url);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "EhgSEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJkaDA1Q89kZFRDn_voYAA"
        );
    }

    #[test]
    fn test_extract_hash_short_url() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w3FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = extract_hash(url);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_hash_without_hash() {
        let url = "CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w3FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = extract_hash(url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), url);
    }

    #[test]
    fn test_decode_invalid_url() {
        let url = "not_a_valid_url";
        let result = decode_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_valid_channel_url() {
        let url = "#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let result = decode_url(url);
        assert!(result.is_ok());

        match result.unwrap() {
            DecodeResult::Channel(config) => {
                assert_eq!(config.channels.len(), 3);
                assert_eq!(config.channels[0].name, "");
                assert!(config.lora.is_some());
            }
            DecodeResult::Node(_) => panic!("Expected Channel, got Node"),
        }
    }

    #[test]
    fn test_decode_node_url() {
        let url = "#CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";
        let result = decode_url(url);
        assert!(result.is_ok());

        match result.unwrap() {
            DecodeResult::Node(node) => {
                assert_eq!(node.num, 1);
                assert_eq!(node.long_name, "Galicia Calidade");
                assert_eq!(node.short_name, "🐙");
                assert_eq!(node.role.to_string(), "Client");
                assert!(node.public_key.is_none());
                assert!(!node.is_unmessagable);
            }
            DecodeResult::Channel(_) => panic!("Expected Node, got Channel"),
        }
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
        let result = extract_hash("https://meshtastic.org/e/#");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_extract_hash_various_formats() {
        assert!(extract_hash("#abc").is_ok());
        assert!(extract_hash("https://meshtastic.org/e/#abc").is_ok());
        assert!(extract_hash("https://meshtastic.org/v/#abc").is_ok());
        assert!(extract_hash("").is_err());
        assert!(extract_hash("abc").is_ok());
        assert!(extract_hash("https://example.com").is_err());
        assert!(extract_hash("some/text#abc").is_ok());
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
    fn test_extract_hash_no_hash_returns_error() {
        assert!(extract_hash("https://example.com").is_err());
        assert!(decode_url("meshtastic.org/e/#abc").is_err());
    }
}
