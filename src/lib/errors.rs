//! Error types for Meshtastic URL encoding and decoding.

use std::fmt;

/// Errors that can occur when decoding a Meshtastic URL.
#[derive(Debug)]
pub enum DecodeError {
    /// The URL format is invalid
    InvalidUrl(String),
    /// Base64 decoding failed
    Base64Decode(String),
    /// Protobuf decoding failed
    ProtobufDecode(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            DecodeError::Base64Decode(msg) => write!(f, "Base64 decode error: {}", msg),
            DecodeError::ProtobufDecode(msg) => write!(f, "Protobuf decode error: {}", msg),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Errors that can occur when encoding a Meshtastic configuration to URL.
#[derive(Debug)]
pub enum EncodeError {
    /// Protobuf encoding failed
    ProtobufEncode(String),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::ProtobufEncode(msg) => write!(f, "Protobuf encode error: {}", msg),
        }
    }
}

impl std::error::Error for EncodeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_decode_error_display_invalid_url() {
        let err = DecodeError::InvalidUrl("missing hash".to_string());
        assert_eq!(format!("{}", err), "Invalid URL: missing hash");
    }

    #[test]
    fn test_decode_error_display_base64() {
        let err = DecodeError::Base64Decode("invalid padding".to_string());
        assert_eq!(format!("{}", err), "Base64 decode error: invalid padding");
    }

    #[test]
    fn test_decode_error_display_protobuf() {
        let err = DecodeError::ProtobufDecode("truncated data".to_string());
        assert_eq!(format!("{}", err), "Protobuf decode error: truncated data");
    }

    #[test]
    fn test_encode_error_display() {
        let err = EncodeError::ProtobufEncode("encoding failed".to_string());
        assert_eq!(format!("{}", err), "Protobuf encode error: encoding failed");
    }

    #[test]
    fn test_decode_error_debug() {
        let err = DecodeError::InvalidUrl("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidUrl"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_encode_error_debug() {
        let err = EncodeError::ProtobufEncode("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ProtobufEncode"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_decode_error_source() {
        let err = DecodeError::InvalidUrl("test".to_string());
        assert!(err.source().is_none());
    }

    #[test]
    fn test_encode_error_source() {
        let err = EncodeError::ProtobufEncode("test".to_string());
        assert!(err.source().is_none());
    }
}
