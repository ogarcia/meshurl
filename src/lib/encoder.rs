//! Meshtastic URL encoder module.
//! Provides functions to encode Meshtastic configurations into URLs.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use meshtastic_protobufs::meshtastic::ChannelSet;
use prost::Message;

use crate::errors::EncodeError;
use crate::models::{MeshtasticConfig, MESHTASTIC_URL_BASE};

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
    Ok(format!("{}{}", MESHTASTIC_URL_BASE, base64))
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

pub use meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset;
pub use meshtastic_protobufs::meshtastic::config::lo_ra_config::RegionCode;

/// Converts a string region name to RegionCode.
pub fn region_code_from_str(s: &str) -> RegionCode {
    match s {
        "US" => RegionCode::Us,
        "EU433" => RegionCode::Eu433,
        "EU868" => RegionCode::Eu868,
        "CN" => RegionCode::Cn,
        "JP" => RegionCode::Jp,
        "ANZ" => RegionCode::Anz,
        "KR" => RegionCode::Kr,
        "TW" => RegionCode::Tw,
        "RU" => RegionCode::Ru,
        "IN" => RegionCode::In,
        "NZ865" => RegionCode::Nz865,
        "TH" => RegionCode::Th,
        "Lora24" => RegionCode::Lora24,
        "UA433" => RegionCode::Ua433,
        "UA868" => RegionCode::Ua868,
        _ => RegionCode::Eu868,
    }
}

/// Converts a string modem preset name to ModemPreset.
pub fn modem_preset_from_str(s: &str) -> ModemPreset {
    match s {
        "LongFast" => ModemPreset::LongFast,
        "LongSlow" => ModemPreset::LongSlow,
        "VeryLongSlow" => ModemPreset::VeryLongSlow,
        "MediumSlow" => ModemPreset::MediumSlow,
        "MediumFast" => ModemPreset::MediumFast,
        "ShortSlow" => ModemPreset::ShortSlow,
        "ShortFast" => ModemPreset::ShortFast,
        "LongModerate" => ModemPreset::LongModerate,
        "ShortTurbo" => ModemPreset::ShortTurbo,
        _ => ModemPreset::LongFast,
    }
}

/// Creates a protobuf ChannelSet from a MeshtasticConfig.
fn create_channel_set(config: &MeshtasticConfig) -> Result<ChannelSet, EncodeError> {
    let settings: Vec<meshtastic_protobufs::meshtastic::ChannelSettings> =
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
mod tests {
    use super::*;
    use crate::models::{ChannelInfo, ChannelRole, LoRaInfo, PskType, DEFAULT_PSK};
    use meshtastic_protobufs::meshtastic::config::lo_ra_config::{ModemPreset, RegionCode};

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
            modem_preset: ModemPreset::LongFast,
            use_preset: true,
            tx_enabled: true,
            tx_power: 0,
            bandwidth: 250,
            spread_factor: 11,
            coding_rate: 5,
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

        let decoded = crate::decoder::decode_url(&encoded).unwrap();

        assert_eq!(decoded.channels.len(), 1);
        assert_eq!(decoded.channels[0].name, "TestChannel");
    }

    #[test]
    fn test_region_code_from_str() {
        use meshtastic_protobufs::meshtastic::config::lo_ra_config::RegionCode;

        assert_eq!(region_code_from_str("US"), RegionCode::Us);
        assert_eq!(region_code_from_str("EU868"), RegionCode::Eu868);
        assert_eq!(region_code_from_str("CN"), RegionCode::Cn);
        assert_eq!(region_code_from_str("JP"), RegionCode::Jp);
        assert_eq!(region_code_from_str("unknown"), RegionCode::Eu868);
    }

    #[test]
    fn test_modem_preset_from_str() {
        use meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset;

        assert_eq!(modem_preset_from_str("LongFast"), ModemPreset::LongFast);
        assert_eq!(modem_preset_from_str("LongSlow"), ModemPreset::LongSlow);
        assert_eq!(
            modem_preset_from_str("VeryLongSlow"),
            ModemPreset::VeryLongSlow
        );
        assert_eq!(modem_preset_from_str("ShortTurbo"), ModemPreset::ShortTurbo);
        assert_eq!(modem_preset_from_str("unknown"), ModemPreset::LongFast);
    }
}
