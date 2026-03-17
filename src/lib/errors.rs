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
