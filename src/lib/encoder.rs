//! Meshtastic URL encoder module.
//! Provides functions to encode Meshtastic configurations into URLs.

use crate::protobufs::ChannelSet;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use prost::Message;

use crate::errors::EncodeError;
use crate::models::{
    MESHTASTIC_CHANNEL_URL_BASE, MODEM_PRESETS, MeshtasticConfig, MeshtasticDisplay, REGION_CODES,
};

/// Encodes a MeshtasticConfig into a full URL.
///
/// # Arguments
/// * `config` - The Meshtastic configuration to encode
///
/// # Returns
/// * `Ok(String)` - A full URL in the format https://meshtastic.org/e/#<base64>
/// * `Err(EncodeError)` - If encoding fails
pub fn encode_url(config: &MeshtasticConfig) -> Result<String, EncodeError> {
    let channel_set = create_channel_set(config)?;
    let encoded = encode_protobuf(&channel_set)?;
    let base64 = encode_base64(&encoded)?;
    Ok(format!("{}{}", MESHTASTIC_CHANNEL_URL_BASE, base64))
}

/// Encodes a MeshtasticConfig into a short URL (just the hash part).
///
/// # Arguments
/// * `config` - The Meshtastic configuration to encode
///
/// # Returns
/// * `Ok(String)` - A short URL in the format #<base64>
/// * `Err(EncodeError)` - If encoding fails
pub fn encode_url_short(config: &MeshtasticConfig) -> Result<String, EncodeError> {
    let channel_set = create_channel_set(config)?;
    let encoded = encode_protobuf(&channel_set)?;
    let base64 = encode_base64(&encoded)?;
    Ok(format!("#{}", base64))
}

pub use crate::protobufs::config::lo_ra_config::{ModemPreset, RegionCode};

/// Looks up a region by name, case-insensitively.
///
/// Returns `None` for an unknown name. Falling back to a default instead, as
/// this used to, silently rewrote the region of a round-tripped URL: a
/// Philippine channel came back out as EU868.
pub fn region_code_from_str(s: &str) -> Option<RegionCode> {
    REGION_CODES
        .iter()
        .find(|region| region.to_mesh_string().eq_ignore_ascii_case(s))
        .copied()
}

/// Looks up a modem preset by name, case-insensitively.
///
/// Returns `None` for an unknown name.
pub fn modem_preset_from_str(s: &str) -> Option<ModemPreset> {
    MODEM_PRESETS
        .iter()
        .find(|preset| preset.to_mesh_string().eq_ignore_ascii_case(s))
        .copied()
}

/// Creates a protobuf ChannelSet from a MeshtasticConfig.
fn create_channel_set(config: &MeshtasticConfig) -> Result<ChannelSet, EncodeError> {
    let settings: Vec<crate::protobufs::ChannelSettings> =
        config.channels.iter().map(|ch| ch.into()).collect();

    let lora_config = config.lora.as_ref().map(|l| l.into());

    Ok(ChannelSet {
        settings,
        lora_config,
    })
}

/// Encodes a ChannelSet into protobuf bytes.
fn encode_protobuf(channel_set: &ChannelSet) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::new();
    channel_set
        .encode(&mut buf)
        .map_err(|e| EncodeError::ProtobufEncode(e.to_string()))?;
    Ok(buf)
}

fn encode_base64(data: &[u8]) -> Result<String, EncodeError> {
    Ok(URL_SAFE_NO_PAD.encode(data))
}

#[cfg(test)]
// Deprecated regions and presets still run on deployed devices, so the
// tests keep exercising them.
#[allow(deprecated)]
mod tests {
    use super::*;
    use crate::models::{ChannelInfo, ChannelRole, DEFAULT_PSK, LoRaInfo, ModemConfig, PskType};
    use crate::protobufs::config::lo_ra_config::{ModemPreset, RegionCode};

    #[test]
    fn test_encode_empty_config() {
        let mut config = MeshtasticConfig::new();

        let channel = ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name: "TestChannel".to_string(),
            psk: DEFAULT_PSK.to_string(),
            psk_type: PskType::Default,
            uplink_enabled: true,
            downlink_enabled: true,
            position_precision: None,
            is_client_muted: false,
        };

        config.channels.push(channel);

        let result = encode_url(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encode_config_with_channel() {
        let mut config = MeshtasticConfig::new();

        let channel = ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name: "TestChannel".to_string(),
            psk: DEFAULT_PSK.to_string(),
            psk_type: PskType::Default,
            uplink_enabled: true,
            downlink_enabled: true,
            position_precision: None,
            is_client_muted: false,
        };

        config.channels.push(channel);

        let result = encode_url(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut config = MeshtasticConfig::new();

        let channel = ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name: "TestChannel".to_string(),
            psk: DEFAULT_PSK.to_string(),
            psk_type: PskType::Default,
            uplink_enabled: true,
            downlink_enabled: true,
            position_precision: None,
            is_client_muted: false,
        };

        config.channels.push(channel);

        let lora = LoRaInfo {
            region: RegionCode::Eu868,
            modem: ModemConfig::Preset(ModemPreset::LongFast),
            tx_enabled: true,
            tx_power: 0,
            hop_limit: 3,
            channel_num: 0,
            override_duty_cycle: false,
            sx126x_rx_boosted_gain: false,
            override_frequency: 0.0,
            frequency_offset: 0.0,
            pa_fan_disabled: false,
            ignore_mqtt: true,
            config_ok_to_mqtt: false,
            ignore_incoming: Vec::new(),
        };

        config.lora = Some(lora);

        let encoded = encode_url_short(&config).unwrap();

        let decoded = match crate::decoder::decode_url(&encoded).unwrap() {
            crate::decoder::DecodeResult::Channel(c) => c,
            crate::decoder::DecodeResult::Node(_) => panic!("Expected Channel"),
        };

        assert_eq!(decoded.channels.len(), 1);
        assert_eq!(decoded.channels[0].name, "TestChannel");
    }

    #[test]
    fn test_region_code_from_str() {
        use crate::protobufs::config::lo_ra_config::RegionCode;

        assert_eq!(region_code_from_str("US"), Some(RegionCode::Us));
        assert_eq!(region_code_from_str("EU_868"), Some(RegionCode::Eu868));
        assert_eq!(region_code_from_str("CN"), Some(RegionCode::Cn));
        assert_eq!(region_code_from_str("JP"), Some(RegionCode::Jp));
        // Regions the CLI and TUI used to be missing.
        assert_eq!(region_code_from_str("PH_915"), Some(RegionCode::Ph915));
        assert_eq!(region_code_from_str("MY_433"), Some(RegionCode::My433));
        assert_eq!(region_code_from_str("ANZ_433"), Some(RegionCode::Anz433));
        // An unknown name is rejected, not quietly turned into EU868.
        assert_eq!(region_code_from_str("unknown"), None);
        assert_eq!(region_code_from_str(""), None);
    }

    #[test]
    fn test_region_code_from_str_is_case_insensitive() {
        use crate::protobufs::config::lo_ra_config::RegionCode;

        assert_eq!(region_code_from_str("eu_868"), Some(RegionCode::Eu868));
        assert_eq!(region_code_from_str("lora_24"), Some(RegionCode::Lora24));
    }

    #[test]
    fn test_every_region_survives_a_name_round_trip() {
        use crate::models::{MeshtasticDisplay, REGION_CODES};

        for region in REGION_CODES {
            assert_eq!(
                region_code_from_str(region.to_mesh_string()),
                Some(*region),
                "{} does not round-trip",
                region.to_mesh_string()
            );
        }
    }

    #[test]
    fn test_modem_preset_from_str() {
        use crate::protobufs::config::lo_ra_config::ModemPreset;

        assert_eq!(
            modem_preset_from_str("LONG_FAST"),
            Some(ModemPreset::LongFast)
        );
        assert_eq!(
            modem_preset_from_str("LONG_SLOW"),
            Some(ModemPreset::LongSlow)
        );
        // VERY_LONG_SLOW is no longer offered: the firmware deprecated it in
        // 2.5 and now falls back to LONG_FAST when it sees it.
        assert_eq!(modem_preset_from_str("VERY_LONG_SLOW"), None);
        // Presets that arrived with firmware 2.8.
        assert_eq!(
            modem_preset_from_str("NARROW_FAST"),
            Some(ModemPreset::NarrowFast)
        );
        assert_eq!(
            modem_preset_from_str("MEDIUM_TURBO"),
            Some(ModemPreset::MediumTurbo)
        );
        assert_eq!(
            modem_preset_from_str("SHORT_TURBO"),
            Some(ModemPreset::ShortTurbo)
        );
        assert_eq!(modem_preset_from_str("unknown"), None);
    }

    #[test]
    fn test_every_modem_preset_survives_a_name_round_trip() {
        use crate::models::{MODEM_PRESETS, MeshtasticDisplay};

        for preset in MODEM_PRESETS {
            assert_eq!(
                modem_preset_from_str(preset.to_mesh_string()),
                Some(*preset),
                "{} does not round-trip",
                preset.to_mesh_string()
            );
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_all_modem_presets() {
        use crate::protobufs::config::lo_ra_config::ModemPreset;

        let presets = [
            ModemPreset::LongFast,
            ModemPreset::LongSlow,
            ModemPreset::VeryLongSlow,
            ModemPreset::MediumSlow,
            ModemPreset::MediumFast,
            ModemPreset::ShortSlow,
            ModemPreset::ShortFast,
            ModemPreset::LongModerate,
            ModemPreset::ShortTurbo,
        ];

        for preset in presets {
            let mut config = MeshtasticConfig::new();
            let channel = ChannelInfo {
                index: 0,
                role: ChannelRole::Primary,
                name: "Test".to_string(),
                psk: DEFAULT_PSK.to_string(),
                psk_type: PskType::Default,
                uplink_enabled: true,
                downlink_enabled: true,
                position_precision: None,
                is_client_muted: false,
            };
            config.channels.push(channel);

            let lora = LoRaInfo {
                region: RegionCode::Us,
                modem: ModemConfig::Preset(preset),
                tx_enabled: true,
                tx_power: 0,
                hop_limit: 3,
                channel_num: 0,
                override_duty_cycle: false,
                sx126x_rx_boosted_gain: false,
                override_frequency: 0.0,
                frequency_offset: 0.0,
                pa_fan_disabled: false,
                ignore_mqtt: true,
                config_ok_to_mqtt: false,
                ignore_incoming: Vec::new(),
            };
            config.lora = Some(lora);

            let encoded = encode_url_short(&config).unwrap();
            let decoded = match crate::decoder::decode_url(&encoded).unwrap() {
                crate::decoder::DecodeResult::Channel(c) => c,
                crate::decoder::DecodeResult::Node(_) => panic!("Expected Channel"),
            };
            assert!(decoded.lora.is_some(), "Failed for preset {:?}", preset);
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_all_regions() {
        use crate::protobufs::config::lo_ra_config::RegionCode;

        let regions = [
            RegionCode::Us,
            RegionCode::Eu433,
            RegionCode::Eu868,
            RegionCode::Cn,
            RegionCode::Jp,
            RegionCode::Anz,
            RegionCode::Kr,
            RegionCode::Tw,
            RegionCode::Ru,
            RegionCode::In,
            RegionCode::Nz865,
            RegionCode::Th,
            RegionCode::Lora24,
            RegionCode::Ua433,
            RegionCode::Ua868,
        ];

        for region in regions {
            let mut config = MeshtasticConfig::new();
            let channel = ChannelInfo {
                index: 0,
                role: ChannelRole::Primary,
                name: "Test".to_string(),
                psk: DEFAULT_PSK.to_string(),
                psk_type: PskType::Default,
                uplink_enabled: true,
                downlink_enabled: true,
                position_precision: None,
                is_client_muted: false,
            };
            config.channels.push(channel);

            let lora = LoRaInfo {
                region,
                modem: ModemConfig::Preset(ModemPreset::LongFast),
                tx_enabled: true,
                tx_power: 0,
                hop_limit: 3,
                channel_num: 0,
                override_duty_cycle: false,
                sx126x_rx_boosted_gain: false,
                override_frequency: 0.0,
                frequency_offset: 0.0,
                pa_fan_disabled: false,
                ignore_mqtt: true,
                config_ok_to_mqtt: false,
                ignore_incoming: Vec::new(),
            };
            config.lora = Some(lora);

            let encoded = encode_url_short(&config).unwrap();
            let decoded = match crate::decoder::decode_url(&encoded).unwrap() {
                crate::decoder::DecodeResult::Channel(c) => c,
                crate::decoder::DecodeResult::Node(_) => panic!("Expected Channel"),
            };
            assert!(decoded.lora.is_some(), "Failed for region {:?}", region);
        }
    }

    #[test]
    fn test_encode_multiple_channels() {
        let mut config = MeshtasticConfig::new();

        let channel1 = ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name: "Primary".to_string(),
            psk: DEFAULT_PSK.to_string(),
            psk_type: PskType::Default,
            uplink_enabled: true,
            downlink_enabled: true,
            position_precision: None,
            is_client_muted: false,
        };
        config.channels.push(channel1);

        let channel2 = ChannelInfo {
            index: 1,
            role: ChannelRole::Secondary,
            name: "Secondary".to_string(),
            psk: "AQ==".to_string(),
            psk_type: PskType::Aes128,
            uplink_enabled: false,
            downlink_enabled: true,
            position_precision: None,
            is_client_muted: false,
        };
        config.channels.push(channel2);

        let encoded = encode_url_short(&config).unwrap();
        let decoded = match crate::decoder::decode_url(&encoded).unwrap() {
            crate::decoder::DecodeResult::Channel(c) => c,
            crate::decoder::DecodeResult::Node(_) => panic!("Expected Channel"),
        };

        assert_eq!(decoded.channels.len(), 2);
        assert_eq!(decoded.channels[0].name, "Primary");
        assert_eq!(decoded.channels[1].name, "Secondary");
    }

    #[test]
    fn test_encode_channel_with_position_precision() {
        let mut config = MeshtasticConfig::new();

        let channel = ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name: "Test".to_string(),
            psk: DEFAULT_PSK.to_string(),
            psk_type: PskType::Default,
            uplink_enabled: true,
            downlink_enabled: true,
            position_precision: Some(10),
            is_client_muted: false,
        };
        config.channels.push(channel);

        let encoded = encode_url_short(&config).unwrap();
        let decoded = match crate::decoder::decode_url(&encoded).unwrap() {
            crate::decoder::DecodeResult::Channel(c) => c,
            crate::decoder::DecodeResult::Node(_) => panic!("Expected Channel"),
        };

        assert_eq!(decoded.channels.len(), 1);
    }

    #[test]
    fn test_encode_tx_disabled() {
        let mut config = MeshtasticConfig::new();
        let channel = ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name: "Test".to_string(),
            psk: DEFAULT_PSK.to_string(),
            psk_type: PskType::Default,
            uplink_enabled: true,
            downlink_enabled: true,
            position_precision: None,
            is_client_muted: false,
        };
        config.channels.push(channel);

        let lora = LoRaInfo {
            region: RegionCode::Us,
            modem: ModemConfig::Preset(ModemPreset::LongFast),
            tx_enabled: false,
            tx_power: 0,
            hop_limit: 3,
            channel_num: 0,
            override_duty_cycle: false,
            sx126x_rx_boosted_gain: false,
            override_frequency: 0.0,
            frequency_offset: 0.0,
            pa_fan_disabled: false,
            ignore_mqtt: true,
            config_ok_to_mqtt: false,
            ignore_incoming: Vec::new(),
        };
        config.lora = Some(lora);

        let encoded = encode_url_short(&config).unwrap();
        let decoded = match crate::decoder::decode_url(&encoded).unwrap() {
            crate::decoder::DecodeResult::Channel(c) => c,
            crate::decoder::DecodeResult::Node(_) => panic!("Expected Channel"),
        };
        assert!(decoded.lora.is_some());
    }
}
