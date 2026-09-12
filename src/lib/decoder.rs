//! Meshtastic URL decoder module.
//! Provides functions to decode Meshtastic channel configuration URLs and node info URLs.

use crate::protobufs::{ChannelSet, SharedContact};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use prost::Message;

use crate::errors::DecodeError;
use crate::models::{
    MESHTASTIC_CHANNEL_ADD_URL_BASE, MESHTASTIC_CHANNEL_URL_BASE, MESHTASTIC_NODE_URL_BASE,
    MeshtasticConfig, NodeInfo,
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
    let parts = extract_hash(url)?;
    let decoded = decode_base64(parts.payload)?;

    if decoded.is_empty() {
        return Err(DecodeError::InvalidUrl(
            "URL carries no configuration".to_string(),
        ));
    }

    let result = match parts.kind {
        // The prefix states what the payload is, so trust it and report a
        // mismatch instead of quietly decoding it as the other kind.
        UrlKind::Channel => try_decode_as_channel(&decoded).map(DecodeResult::Channel),
        UrlKind::Node => try_decode_as_node(&decoded).map(DecodeResult::Node),
        UrlKind::Unknown => decode_by_shape(&decoded),
    };

    // `add=true` is part of the URL, not of the payload, so it is carried over
    // once the channels themselves are decoded.
    match result {
        Ok(DecodeResult::Channel(mut config)) => {
            config.add_only = parts.add_only;
            Ok(DecodeResult::Channel(config))
        }
        other => other,
    }
}

/// Identifies a payload that arrived without a prefix to go by.
///
/// Protobuf skips fields it does not know, so the two message types accept
/// many of the same byte strings; this is a guess, which is why it is only
/// reached when the URL itself says nothing.
fn decode_by_shape(decoded: &[u8]) -> Result<DecodeResult, DecodeError> {
    if let Ok(config) = try_decode_as_channel(decoded) {
        return Ok(DecodeResult::Channel(config));
    }

    if let Ok(node) = try_decode_as_node(decoded) {
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

/// A `/v/` URL carries a `SharedContact`, the message a device hands another
/// when a contact is shared, and not the `NodeInfo` of the node database. The
/// two agree on the node number and the user, which is why a plain contact
/// decoded as either, but `should_ignore` and `manually_verified` land on
/// `NodeInfo`'s position and SNR, and a URL setting them was refused.
fn try_decode_as_node(data: &[u8]) -> Result<NodeInfo, DecodeError> {
    let contact = SharedContact::decode(data)
        .map_err(|_| DecodeError::InvalidUrl("Not a valid shared contact".to_string()))?;
    Ok(NodeInfo::from_pb(&contact))
}

/// What kind of payload a URL announces through its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UrlKind {
    /// An `/e/` URL: channel configuration.
    Channel,
    /// A `/v/` URL: node information.
    Node,
    /// A bare hash or base64 string, which says nothing about its contents.
    Unknown,
}

/// What a URL turned out to be, and the payload to decode.
struct UrlParts<'a> {
    kind: UrlKind,
    payload: &'a str,
    /// Whether the URL asked to add its channels rather than replace them.
    add_only: bool,
}

/// Extracts the base64 hash part from a Meshtastic URL, along with the kind of
/// payload the URL claims to carry.
///
/// Supports multiple URL formats:
/// - https://meshtastic.org/e/#<hash> (channel)
/// - https://meshtastic.org/e/?add=true#<hash> (channel, added not replaced)
/// - https://meshtastic.org/v/#<hash> (node)
/// - meshtastic.org/e/#<hash>
/// - meshtastic.org/v/#<hash>
/// - #<hash>
/// - text#<hash>
/// - <base64> (raw base64 without prefix)
fn extract_hash(url: &str) -> Result<UrlParts<'_>, DecodeError> {
    const CHANNEL_PREFIXES: &[&str] = &[MESHTASTIC_CHANNEL_URL_BASE, "meshtastic.org/e/#"];
    const CHANNEL_ADD_PREFIXES: &[&str] = &[
        MESHTASTIC_CHANNEL_ADD_URL_BASE,
        "meshtastic.org/e/?add=true#",
    ];
    const NODE_PREFIXES: &[&str] = &[MESHTASTIC_NODE_URL_BASE, "meshtastic.org/v/#"];

    // Before the plain form, whose prefix is not a prefix of this one.
    for prefix in CHANNEL_ADD_PREFIXES {
        if let Some(payload) = url.strip_prefix(prefix) {
            return Ok(UrlParts {
                kind: UrlKind::Channel,
                payload,
                add_only: true,
            });
        }
    }

    for prefix in CHANNEL_PREFIXES {
        if let Some(payload) = url.strip_prefix(prefix) {
            return Ok(UrlParts {
                kind: UrlKind::Channel,
                payload,
                add_only: false,
            });
        }
    }

    for prefix in NODE_PREFIXES {
        if let Some(payload) = url.strip_prefix(prefix) {
            return Ok(UrlParts {
                kind: UrlKind::Node,
                payload,
                add_only: false,
            });
        }
    }

    if url.contains('#') {
        return Ok(UrlParts {
            kind: UrlKind::Unknown,
            payload: url.rsplit('#').next().unwrap_or(url),
            add_only: false,
        });
    }

    if !url.starts_with("https://") && !url.starts_with("meshtastic.org") && !url.is_empty() {
        return Ok(UrlParts {
            kind: UrlKind::Unknown,
            payload: url,
            add_only: false,
        });
    }

    Err(DecodeError::InvalidUrl(format!(
        "Expected format: {} or {} <base64>",
        MESHTASTIC_CHANNEL_URL_BASE, MESHTASTIC_NODE_URL_BASE
    )))
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
    use crate::protobufs::User;

    /// A one channel `ChannelSet`, and a `NodeInfo`, as the payload of a URL.
    const CHANNEL_PAYLOAD: &str = "CgsSAQEoATABOgIIDQ";
    const NODE_PAYLOAD: &str = "CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";

    #[test]
    fn an_add_url_is_a_channel_url() {
        // It used to fall through to the shape guessing, because the query
        // string meant the `/e/` prefix no longer matched.
        let url = format!("{}{}", MESHTASTIC_CHANNEL_ADD_URL_BASE, CHANNEL_PAYLOAD);

        let parts = extract_hash(&url).expect("an add URL parses");

        assert_eq!(parts.kind, UrlKind::Channel);
        assert!(parts.add_only, "and it says so");
        assert_eq!(parts.payload, CHANNEL_PAYLOAD);
    }

    #[test]
    fn an_add_url_without_the_scheme_is_recognised_too() {
        let url = format!("meshtastic.org/e/?add=true#{}", CHANNEL_PAYLOAD);

        let parts = extract_hash(&url).expect("an add URL parses");

        assert_eq!(parts.kind, UrlKind::Channel);
        assert!(parts.add_only);
    }

    #[test]
    fn decoding_an_add_url_keeps_the_flag() {
        let url = format!("{}{}", MESHTASTIC_CHANNEL_ADD_URL_BASE, CHANNEL_PAYLOAD);

        match decode_url(&url).expect("the URL decodes") {
            DecodeResult::Channel(config) => {
                assert!(config.add_only, "the configuration carries it");
                assert!(!config.channels.is_empty(), "and the channels decoded");
            }
            DecodeResult::Node(_) => panic!("expected a channel URL"),
        }
    }

    #[test]
    fn a_plain_url_is_not_an_add_url() {
        match decode_url(&format!(
            "{}{}",
            MESHTASTIC_CHANNEL_URL_BASE, CHANNEL_PAYLOAD
        ))
        .expect("the URL decodes")
        {
            DecodeResult::Channel(config) => assert!(!config.add_only),
            DecodeResult::Node(_) => panic!("expected a channel URL"),
        }
    }

    #[test]
    fn a_node_url_is_never_an_add_url() {
        // `add=true` is about a channel table, which node URLs do not carry.
        let url = format!("{}{}", MESHTASTIC_NODE_URL_BASE, NODE_PAYLOAD);

        let parts = extract_hash(&url).expect("a node URL parses");

        assert!(!parts.add_only);
    }

    #[test]
    fn test_extract_hash_channel_url() {
        let url = "https://meshtastic.org/e/#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w3FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";
        let parts = extract_hash(url).expect("a channel URL parses");
        assert_eq!(parts.kind, UrlKind::Channel);
        assert!(!parts.add_only, "a plain URL replaces the channels");
        assert!(parts.payload.starts_with("CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w3FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ"));
    }

    #[test]
    fn test_extract_hash_node_url() {
        let url =
            "https://meshtastic.org/v/#EhgSEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJkaDA1Q89kZFRDn_voYAA";
        let parts = extract_hash(url).expect("a node URL parses");
        assert_eq!(parts.kind, UrlKind::Node);
        assert_eq!(
            parts.payload,
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
        let parts = extract_hash(url).expect("a bare base64 string parses");
        assert_eq!(parts.kind, UrlKind::Unknown);
        assert_eq!(parts.payload, url);
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
    fn a_contact_carrying_its_flags_decodes() {
        // Reported: a `/v/` URL the firmware had produced was refused. The
        // flags are the reason, `manually_verified` being field 4, which in
        // the `NodeInfo` these URLs used to be decoded as is a float.
        let contact = SharedContact {
            node_num: 2697684788,
            user: Some(User {
                id: "!a0cb6734".to_string(),
                long_name: "Dom 6734".to_string(),
                short_name: "6734".to_string(),
                ..Default::default()
            }),
            should_ignore: true,
            manually_verified: true,
        };
        let url = format!(
            "{}{}",
            MESHTASTIC_NODE_URL_BASE,
            URL_SAFE_NO_PAD.encode(contact.encode_to_vec())
        );

        match decode_url(&url).expect("a shared contact decodes") {
            DecodeResult::Node(node) => {
                assert_eq!(node.num, 2697684788);
                assert_eq!(node.long_name, "Dom 6734");
                assert!(node.manually_verified, "the key was verified by hand");
                assert!(node.should_ignore, "and it is shared to be ignored");
            }
            DecodeResult::Channel(_) => panic!("expected a node URL"),
        }
    }

    #[test]
    fn a_contact_without_flags_keeps_them_off() {
        let url = format!("{}{}", MESHTASTIC_NODE_URL_BASE, NODE_PAYLOAD);

        match decode_url(&url).expect("a plain contact decodes") {
            DecodeResult::Node(node) => {
                assert!(!node.manually_verified);
                assert!(!node.should_ignore);
            }
            DecodeResult::Channel(_) => panic!("expected a node URL"),
        }
    }

    #[test]
    fn test_decode_empty_payload_is_an_error() {
        // These used to report an empty but successful channel configuration.
        for url in [
            "https://meshtastic.org/e/#",
            "https://meshtastic.org/v/#",
            "#",
        ] {
            let error = decode_url(url).expect_err("an empty payload is refused");
            assert!(
                error.to_string().contains("no configuration"),
                "unexpected error for {}: {}",
                url,
                error
            );
        }
    }

    #[test]
    fn test_channel_prefix_refuses_node_payload() {
        // A node payload behind an /e/ prefix used to be decoded as a channel.
        let node_payload = "CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";
        let url = format!("{}{}", MESHTASTIC_CHANNEL_URL_BASE, node_payload);

        assert!(decode_url(&url).is_err());
    }

    #[test]
    fn test_node_prefix_refuses_channel_payload() {
        let channel_payload = "CgsSAQEoATABOgIIDQ";
        let url = format!("{}{}", MESHTASTIC_NODE_URL_BASE, channel_payload);

        assert!(decode_url(&url).is_err());
    }

    #[test]
    fn test_prefix_decides_the_payload_kind() {
        let node_payload = "CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";
        let url = format!("{}{}", MESHTASTIC_NODE_URL_BASE, node_payload);

        match decode_url(&url).expect("a node URL decodes") {
            DecodeResult::Node(node) => assert_eq!(node.long_name, "Galicia Calidade"),
            DecodeResult::Channel(_) => panic!("the /v/ prefix must yield a node"),
        }

        let channel_payload = "CgsSAQEoATABOgIIDQ";
        let url = format!("{}{}", MESHTASTIC_CHANNEL_URL_BASE, channel_payload);

        match decode_url(&url).expect("a channel URL decodes") {
            DecodeResult::Channel(config) => assert_eq!(config.channels.len(), 1),
            DecodeResult::Node(_) => panic!("the /e/ prefix must yield a channel"),
        }
    }

    #[test]
    fn test_bare_payload_is_identified_by_shape() {
        // Without a prefix there is nothing else to go by.
        let node_payload = "#CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";
        assert!(matches!(
            decode_url(node_payload).expect("decodes"),
            DecodeResult::Node(_)
        ));

        let channel_payload = "#CgsSAQEoATABOgIIDQ";
        assert!(matches!(
            decode_url(channel_payload).expect("decodes"),
            DecodeResult::Channel(_)
        ));
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
        let parts = extract_hash("https://meshtastic.org/e/#").expect("parses");
        assert_eq!(parts.kind, UrlKind::Channel);
        assert_eq!(parts.payload, "");

        // ...but an empty payload is not a configuration.
        assert!(decode_url("https://meshtastic.org/e/#").is_err());
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
