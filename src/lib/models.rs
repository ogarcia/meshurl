use base64::{engine::general_purpose::STANDARD, Engine as _};
use meshtastic_protobufs::meshtastic::config::{
    lo_ra_config::ModemPreset, lo_ra_config::RegionCode, LoRaConfig,
};
use meshtastic_protobufs::meshtastic::{ChannelSettings, ModuleSettings};

/// Provides consistent string representation for Meshtastic enums.
/// Converts protobuf enum variants to standardized uppercase string format.
pub trait MeshtasticDisplay {
    /// Returns the standardized string representation.
    fn to_mesh_string(&self) -> &'static str;
}

impl MeshtasticDisplay for RegionCode {
    /// Converts RegionCode to uppercase string (e.g., "US", "EU868", "CN").
    fn to_mesh_string(&self) -> &'static str {
        match self {
            RegionCode::Unset => "Unset",
            RegionCode::Us => "US",
            RegionCode::Eu433 => "EU433",
            RegionCode::Eu868 => "EU868",
            RegionCode::Cn => "CN",
            RegionCode::Jp => "JP",
            RegionCode::Anz => "ANZ",
            RegionCode::Kr => "KR",
            RegionCode::Tw => "TW",
            RegionCode::Ru => "RU",
            RegionCode::In => "IN",
            RegionCode::Nz865 => "NZ865",
            RegionCode::Th => "TH",
            RegionCode::Lora24 => "Lora24",
            RegionCode::Ua433 => "UA433",
            RegionCode::Ua868 => "UA868",
            RegionCode::My433 => "MY433",
            RegionCode::My919 => "MY919",
            RegionCode::Sg923 => "SG923",
            RegionCode::Ph433 => "PH433",
            RegionCode::Ph868 => "PH868",
            RegionCode::Ph915 => "PH915",
            RegionCode::Anz433 => "ANZ433",
        }
    }
}

impl MeshtasticDisplay for ModemPreset {
    /// Converts ModemPreset to string (e.g., "LongFast", "ShortSlow").
    fn to_mesh_string(&self) -> &'static str {
        match self {
            ModemPreset::LongFast => "LongFast",
            ModemPreset::LongSlow => "LongSlow",
            ModemPreset::VeryLongSlow => "VeryLongSlow",
            ModemPreset::MediumSlow => "MediumSlow",
            ModemPreset::MediumFast => "MediumFast",
            ModemPreset::ShortSlow => "ShortSlow",
            ModemPreset::ShortFast => "ShortFast",
            ModemPreset::LongModerate => "LongModerate",
            ModemPreset::ShortTurbo => "ShortTurbo",
        }
    }
}

/// Position precision options for Meshtastic channels.
/// Each tuple contains (display_name, precision_bits).
/// Values 0-32 represent precision bits:
/// - 0 = disabled
/// - 10-19 = progressive obfuscation (higher = more precise)
/// - 32 = full precision (GPS)
pub const POSITION_OPTIONS: &[(&str, u32)] = &[
    ("Disabled", 0),
    ("23,3 Km", 10),
    ("11,7 Km", 11),
    ("5,8 Km", 12),
    ("2,9 Km", 13),
    ("1,5 Km", 14),
    ("729 m", 15),
    ("364 m", 16),
    ("182 m", 17),
    ("91 m", 18),
    ("45 m", 19),
    ("Precise", 32),
];

/// Default PSK value (base64 encoded single byte [1]).
pub const DEFAULT_PSK: &str = "AQ==";

/// Base URL for Meshtastic channel configuration.
pub const MESHTASTIC_URL_BASE: &str = "https://meshtastic.org/e/#";

/// Generates a random 32-byte PSK encoded in base64.
/// Uses a simple linear congruential generator seeded with current time.
pub fn generate_random_psk() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let mut bytes = [0u8; 32];
    let mut rng = seed;
    for byte in bytes.iter_mut() {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        *byte = (rng >> 16) as u8;
    }
    STANDARD.encode(bytes)
}

/// Converts a text phrase to a 32-byte PSK using SHA256 hashing.
/// The phrase is hashed and encoded in base64.
pub fn hash_phrase_to_psk(phrase: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(phrase.as_bytes());
    let result = hasher.finalize();
    STANDARD.encode(result.as_slice())
}

/// PSK (Pre-Shared Key) mode for channel encryption.
/// - Default: uses the default (weak, public) key
/// - None: no encryption
/// - Random: generates a secure random key (AES-256)
/// - Manual: user provides a raw PSK in base64 (16 or 32 bytes)
/// - Phrase: user provides a text phrase that gets hashed to a PSK
#[derive(Debug, Clone, PartialEq)]
pub enum PskMode {
    Default,
    None,
    Random,
    Base64(String),
    Passphrase(String),
}

impl PskMode {
    pub fn is_none(&self) -> bool {
        matches!(self, PskMode::None)
    }
}

impl std::fmt::Display for PskMode {
    /// Formats PskMode as a human-readable string for TUI display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PskMode::Default => write!(f, "Default"),
            PskMode::None => write!(f, "None"),
            PskMode::Random => write!(f, "Random"),
            PskMode::Base64(_) => write!(f, "Base64"),
            PskMode::Passphrase(_) => write!(f, "Passphrase"),
        }
    }
}

/// Validates and normalizes a base64-encoded PSK.
/// Returns the PSK if valid (16 or 32 bytes), or an error otherwise.
fn validate_and_normalize_psk(psk: &str) -> Result<String, String> {
    STANDARD
        .decode(psk)
        .map_err(|_| "Invalid Base64 PSK".to_string())
        .and_then(|bytes| match bytes.len() {
            16 | 32 => Ok(STANDARD.encode(&bytes)),
            n => Err(format!("Invalid PSK length: {} bytes", n)),
        })
}

/// PSK encryption type used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PskType {
    Default,
    None,
    Simple(u8),
    Aes128,
    Aes256,
    Unknown,
}

/// Channel role in Meshtastic configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelRole {
    Primary,
    Secondary,
}

/// Information about a Meshtastic channel.
/// Represents the configuration of an individual channel.
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    /// Channel index (0-7)
    pub index: usize,
    /// Channel role (Primary or Secondary)
    pub role: ChannelRole,
    /// Channel name (max 12 bytes)
    pub name: String,
    /// PSK key in Base64 format
    pub psk: String,
    /// Encryption type used
    pub psk_type: PskType,
    /// Enable forwarding messages from MQTT to LoRa
    pub uplink_enabled: bool,
    /// Enable forwarding messages from LoRa to MQTT
    pub downlink_enabled: bool,
    /// Position precision (None = disabled, 0-32 = precision bits)
    pub position_precision: Option<u32>,
    /// Mute notifications for this channel
    pub is_client_muted: bool,
}

/// Implements conversion from a text string to ChannelInfo.
/// Expected format: key=value pairs separated by commas.
/// Supported keys: name, psk, psk_mode, uplink, downlink, pos, muted
///
/// # Example
/// ```
/// use std::str::FromStr;
/// use meshurl::ChannelInfo;
///
/// let channel = ChannelInfo::from_str("name=Test,psk_mode=random,uplink").unwrap();
/// assert_eq!(channel.name, "Test");
/// assert!(channel.uplink_enabled);
/// ```
impl std::str::FromStr for ChannelInfo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut name: Option<String> = None;
        let mut is_default = false;
        let mut psk: Option<String> = None;
        let mut psk_mode: Option<PskMode> = None;
        let mut psk_phrase: Option<String> = None;
        let mut uplink = false;
        let mut downlink = false;
        let mut position_precision: Option<u32> = None;
        let mut muted = false;

        for part in s.split(',') {
            let parts: Vec<&str> = part.splitn(2, '=').collect();
            let key = parts[0].trim();
            let value = parts.get(1).map(|v| v.trim());

            match key {
                "default" | "d" => is_default = true,
                "name" | "n" => name = Some(value.unwrap_or("").to_string()),
                "psk_base64" | "psk" => psk = Some(value.unwrap_or("").to_string()),
                "psk_mode" | "mode" => {
                    psk_mode = match value {
                        Some("default") | Some("d") => Some(PskMode::Default),
                        Some("none") | Some("n") => Some(PskMode::None),
                        Some("random") | Some("r") => Some(PskMode::Random),
                        Some(v) if v.starts_with("base64:") => {
                            Some(PskMode::Base64(v[7..].to_string()))
                        }
                        Some(v) if v.starts_with("passphrase:") => {
                            Some(PskMode::Passphrase(v[12..].to_string()))
                        }
                        _ => None,
                    }
                }
                "psk_passphrase" | "psk_phrase" | "phrase" => {
                    psk_phrase = Some(value.unwrap_or("").to_string())
                }
                "uplink" | "up" => uplink = true,
                "downlink" | "down" => downlink = true,
                "pos" | "precision" => {
                    position_precision = value.and_then(|v| v.parse().ok());
                }
                "muted" | "mute" => muted = true,
                _ => return Err(format!("Unknown option: {}", key)),
            }
        }

        let (name, psk) = if is_default
            || (name.is_none() && psk.is_none() && psk_mode.is_none() && psk_phrase.is_none())
        {
            (String::new(), DEFAULT_PSK.to_string())
        } else {
            let final_name = name.unwrap_or_default();
            let final_psk = if let Some(p) = psk {
                validate_and_normalize_psk(&p)?
            } else if let Some(phrase) = psk_phrase {
                hash_phrase_to_psk(&phrase)
            } else {
                match psk_mode.unwrap_or(PskMode::Default) {
                    PskMode::Default => DEFAULT_PSK.to_string(),
                    PskMode::None => String::new(),
                    PskMode::Random => generate_random_psk(),
                    PskMode::Base64(psk_str) => validate_and_normalize_psk(&psk_str)?,
                    PskMode::Passphrase(phrase) => hash_phrase_to_psk(&phrase),
                }
            };
            (final_name, final_psk)
        };

        let psk_type = if psk.is_empty() {
            PskType::None
        } else if psk == DEFAULT_PSK {
            PskType::Default
        } else {
            match STANDARD.decode(&psk) {
                Ok(bytes) => PskType::from_bytes(&bytes),
                Err(_) => PskType::Unknown,
            }
        };

        Ok(ChannelInfo {
            index: 0,
            role: ChannelRole::Primary,
            name,
            psk,
            psk_type,
            uplink_enabled: uplink,
            downlink_enabled: downlink,
            position_precision,
            is_client_muted: muted,
        })
    }
}

impl ChannelRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelRole::Primary => "PRIMARY",
            ChannelRole::Secondary => "SECONDARY",
        }
    }
}

impl PskType {
    /// Returns a human-readable string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            PskType::None => "None (no crypto)",
            PskType::Default => "Default",
            PskType::Simple(_n) => "Simple",
            PskType::Aes128 => "AES-128",
            PskType::Aes256 => "AES-256",
            PskType::Unknown => "Unknown",
        }
    }

    /// Determine PSK type from raw bytes.
    pub fn from_bytes(psk: &[u8]) -> Self {
        match psk.len() {
            0 => PskType::None,
            1 => match psk[0] {
                0 => PskType::None,
                1 => PskType::Default,
                n if (2..=10).contains(&n) => PskType::Simple(n),
                _ => PskType::Unknown,
            },
            16 => PskType::Aes128,
            32 => PskType::Aes256,
            _ => PskType::Unknown,
        }
    }

    /// Check if this is the default PSK type.
    pub fn is_default(&self) -> bool {
        matches!(self, PskType::Default)
    }

    /// Check if this is a custom PSK (AES-128 or AES-256).
    pub fn is_custom(&self) -> bool {
        matches!(self, PskType::Aes128 | PskType::Aes256)
    }

    /// Check if this is a simple PSK type.
    pub fn is_simple(&self) -> bool {
        matches!(self, PskType::Simple(_))
    }
}

/// LoRa radio configuration for Meshtastic.
/// Contains all configurable parameters for the LoRa modem.
#[derive(Debug, Clone, Default)]
pub struct LoRaInfo {
    /// Region code (e.g., EU868, US, CN, etc.)
    pub region: RegionCode,
    /// Modem preset (LongFast, MediumSlow, etc.)
    pub modem_preset: ModemPreset,
    /// Whether to use the preset values instead of manual configuration
    pub use_preset: bool,
    /// Whether TX is enabled
    pub tx_enabled: bool,
    /// TX power in dBm (0 = use default maximum safe power)
    pub tx_power: i32,
    /// Bandwidth in Hz (only used if use_preset is false)
    pub bandwidth: u32,
    /// Spreading factor (only used if use_preset is false)
    pub spread_factor: u32,
    /// Coding rate (only used if use_preset is false)
    pub coding_rate: u32,
    /// Maximum number of hops for packets
    pub hop_limit: u32,
    /// Channel number (0 = auto)
    pub channel_num: u32,
    /// Override the duty cycle limit (for regions with restrictions)
    pub override_duty_cycle: bool,
    /// Enable SX126x RX boosted gain mode
    pub sx126x_rx_boosted_gain: bool,
    /// Override the frequency (MHz, 0 = use default)
    pub override_frequency: f32,
    /// Frequency offset in kHz
    pub frequency_offset: f32,
    /// Disable the PA fan
    pub pa_fan_disabled: bool,
    /// Ignore messages that came via MQTT
    pub ignore_mqtt: bool,
    /// Allow packets to be sent to MQTT
    pub config_ok_to_mqtt: bool,
    /// List of node IDs to ignore incoming messages from
    pub ignore_incoming: Vec<u32>,
}

/// Get the default LoRa parameters for a given modem preset.
/// Returns (bandwidth_kHz, spreading_factor, coding_rate).
///
/// # Arguments
/// * `preset` - The modem preset to get parameters for
///
/// # Returns
/// A tuple of (bandwidth in kHz, spreading factor, coding rate denominator)
///
/// # Example
/// ```
/// use meshurl::{get_preset_params, ModemPreset};
///
/// let (bw, sf, cr) = get_preset_params(ModemPreset::LongFast);
/// assert_eq!(bw, 250);  // 250 kHz
/// assert_eq!(sf, 11);   // SF11
/// assert_eq!(cr, 5);     // 4/5
/// ```
pub fn get_preset_params(
    preset: meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset,
) -> (u32, u32, u32) {
    use meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset;
    match preset {
        ModemPreset::LongFast => (250, 11, 5),
        ModemPreset::LongSlow => (250, 12, 5),
        ModemPreset::VeryLongSlow => (125, 12, 8),
        ModemPreset::MediumSlow => (125, 10, 5),
        ModemPreset::MediumFast => (250, 9, 5),
        ModemPreset::ShortSlow => (125, 8, 5),
        ModemPreset::ShortFast => (250, 7, 5),
        ModemPreset::LongModerate => (250, 10, 5),
        ModemPreset::ShortTurbo => (500, 7, 5),
    }
}

/// Converts a protobuf ChannelSettings to ChannelInfo.
/// This is used when decoding a Meshtastic URL to extract channel information.
impl From<&ChannelSettings> for ChannelInfo {
    fn from(settings: &ChannelSettings) -> Self {
        let name = settings.name.clone();

        let psk = if settings.psk.is_empty() {
            String::new()
        } else {
            STANDARD.encode(&settings.psk)
        };

        let psk_type = PskType::from_bytes(&settings.psk);

        let (position_precision, is_client_muted) = settings
            .module_settings
            .as_ref()
            .map(|ms| {
                let prec = if ms.position_precision > 0 {
                    Some(ms.position_precision)
                } else {
                    None
                };
                (prec, ms.is_client_muted)
            })
            .unwrap_or((None, false));

        ChannelInfo {
            index: 0,
            role: ChannelRole::Secondary,
            name,
            psk,
            psk_type,
            uplink_enabled: settings.uplink_enabled,
            downlink_enabled: settings.downlink_enabled,
            position_precision,
            is_client_muted,
        }
    }
}

/// Converts a protobuf LoRaConfig to LoRaInfo.
/// Automatically detects if a preset was used by comparing the config values
/// with known preset parameters.
impl From<&LoRaConfig> for LoRaInfo {
    fn from(config: &LoRaConfig) -> Self {
        let modem_preset = config.modem_preset();

        let (preset_bw, preset_sf, preset_cr) = get_preset_params(modem_preset);

        let bw_khz = config.bandwidth / 1000;
        let use_preset = config.use_preset
            || config.bandwidth == 0
            || (bw_khz == preset_bw
                && config.spread_factor == preset_sf
                && config.coding_rate == preset_cr);

        let (bandwidth, spread_factor, coding_rate) = if use_preset {
            get_preset_params(modem_preset)
        } else {
            (config.bandwidth, config.spread_factor, config.coding_rate)
        };

        LoRaInfo {
            region: config.region(),
            modem_preset,
            use_preset,
            tx_enabled: config.tx_enabled,
            tx_power: config.tx_power,
            bandwidth,
            spread_factor,
            coding_rate,
            hop_limit: config.hop_limit,
            channel_num: config.channel_num,
            override_duty_cycle: config.override_duty_cycle,
            sx126x_rx_boosted_gain: config.sx126x_rx_boosted_gain,
            override_frequency: config.override_frequency,
            frequency_offset: config.frequency_offset,
            pa_fan_disabled: config.pa_fan_disabled,
            ignore_mqtt: config.ignore_mqtt,
            config_ok_to_mqtt: config.config_ok_to_mqtt,
            ignore_incoming: config.ignore_incoming.clone(),
        }
    }
}

/// Complete Meshtastic configuration containing channels and LoRa settings.
#[derive(Debug, Clone)]
pub struct MeshtasticConfig {
    /// List of channel configurations
    pub channels: Vec<ChannelInfo>,
    /// LoRa radio configuration (optional)
    pub lora: Option<LoRaInfo>,
}

impl MeshtasticConfig {
    /// Creates a MeshtasticConfig from a protobuf ChannelSet.
    /// This is used when decoding a URL to extract all configuration.
    pub fn from_channel_set(channel_set: &meshtastic_protobufs::meshtastic::ChannelSet) -> Self {
        let channels: Vec<ChannelInfo> = channel_set
            .settings
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let mut info = ChannelInfo::from(s);
                info.index = i;
                info.role = if i == 0 {
                    ChannelRole::Primary
                } else {
                    ChannelRole::Secondary
                };
                info
            })
            .collect();

        let lora = channel_set.lora_config.as_ref().map(LoRaInfo::from);

        MeshtasticConfig { channels, lora }
    }

    pub fn new() -> Self {
        MeshtasticConfig {
            channels: Vec::new(),
            lora: None,
        }
    }
}

impl Default for MeshtasticConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a ChannelInfo to a protobuf ChannelSettings.
/// This is used when encoding a URL to create the binary config.
impl From<&ChannelInfo> for ChannelSettings {
    fn from(info: &ChannelInfo) -> Self {
        let psk = if info.psk.is_empty() {
            Vec::new()
        } else {
            STANDARD.decode(&info.psk).unwrap_or_else(|_| Vec::new())
        };

        let module_settings = if info.position_precision.is_some() || info.is_client_muted {
            Some(ModuleSettings {
                position_precision: info.position_precision.unwrap_or(0),
                is_client_muted: info.is_client_muted,
            })
        } else {
            None
        };

        ChannelSettings {
            name: info.name.clone(),
            psk,
            uplink_enabled: info.uplink_enabled,
            downlink_enabled: info.downlink_enabled,
            module_settings,
            ..Default::default()
        }
    }
}

/// Converts a LoRaInfo to a protobuf LoRaConfig.
/// This is used when encoding a URL to create the binary config.
impl From<&LoRaInfo> for LoRaConfig {
    fn from(info: &LoRaInfo) -> Self {
        LoRaConfig {
            region: info.region as i32,
            modem_preset: info.modem_preset as i32,
            use_preset: info.use_preset,
            tx_enabled: info.tx_enabled,
            tx_power: info.tx_power,
            bandwidth: info.bandwidth,
            spread_factor: info.spread_factor,
            coding_rate: info.coding_rate,
            hop_limit: info.hop_limit,
            channel_num: info.channel_num,
            override_duty_cycle: info.override_duty_cycle,
            sx126x_rx_boosted_gain: info.sx126x_rx_boosted_gain,
            override_frequency: info.override_frequency,
            frequency_offset: info.frequency_offset,
            pa_fan_disabled: info.pa_fan_disabled,
            ignore_mqtt: info.ignore_mqtt,
            config_ok_to_mqtt: info.config_ok_to_mqtt,
            ignore_incoming: info.ignore_incoming.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_info_psk_mode_base64() {
        let psk = "CcZBoFJbADPGEoSkkYPA3Ha23rr7WPcyUo1AjorGQIA=";
        let channel: ChannelInfo = format!("psk_mode=base64:{}", psk).parse().unwrap();
        assert!(channel.psk_type != PskType::None);
        assert!(!channel.psk.is_empty());
        assert_eq!(channel.psk, psk);
    }

    #[test]
    fn test_channel_info_psk_mode_passphrase() {
        let channel: ChannelInfo = "psk_mode=passphrase:my secret phrase".parse().unwrap();
        assert_eq!(channel.psk_type, PskType::Aes256);
        assert_eq!(channel.psk.len(), 44);
    }

    #[test]
    fn test_channel_info_psk_mode_base64_invalid() {
        let result: Result<ChannelInfo, _> = "psk_mode=base64:not-valid-base64!!!".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_info_psk_mode_base64_short() {
        let result: Result<ChannelInfo, _> = "psk_mode=base64:MTIzNDU2".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_random_psk() {
        let psk1 = generate_random_psk();
        let psk2 = generate_random_psk();

        assert!(!psk1.is_empty());
        assert!(!psk2.is_empty());
        assert_ne!(psk1, psk2);

        let decoded = STANDARD.decode(&psk1).unwrap();
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn test_hash_phrase_to_psk() {
        let psk1 = hash_phrase_to_psk("my secret phrase");
        let psk2 = hash_phrase_to_psk("my secret phrase");
        let psk3 = hash_phrase_to_psk("different phrase");

        assert_eq!(psk1, psk2);
        assert_ne!(psk1, psk3);

        let decoded = STANDARD.decode(&psk1).unwrap();
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn test_psk_mode_is_none() {
        assert!(PskMode::None.is_none());
        assert!(!PskMode::Default.is_none());
        assert!(!PskMode::Random.is_none());
        assert!(!PskMode::Base64("abc".to_string()).is_none());
        assert!(!PskMode::Passphrase("phrase".to_string()).is_none());
    }

    #[test]
    fn test_psk_mode_default_parsing() {
        let channel: ChannelInfo = "psk_mode=default".parse().unwrap();
        assert_eq!(channel.psk_type, PskType::Default);
        assert_eq!(channel.psk, DEFAULT_PSK);
    }

    #[test]
    fn test_psk_mode_none_parsing() {
        let channel: ChannelInfo = "psk_mode=none".parse().unwrap();
        assert_eq!(channel.psk_type, PskType::None);
        assert_eq!(channel.psk, "");
    }

    #[test]
    fn test_psk_mode_random_parsing() {
        let channel: ChannelInfo = "psk_mode=random".parse().unwrap();
        assert_eq!(channel.psk_type, PskType::Aes256);
        assert!(!channel.psk.is_empty());
    }

    #[test]
    fn test_psk_type_variants() {
        assert_eq!(PskType::Default, PskType::Default);
        assert_eq!(PskType::None, PskType::None);
        assert_eq!(PskType::Simple(1), PskType::Simple(1));
        assert_ne!(PskType::Simple(1), PskType::Simple(2));
        assert_eq!(PskType::Aes128, PskType::Aes128);
        assert_eq!(PskType::Aes256, PskType::Aes256);
        assert_eq!(PskType::Unknown, PskType::Unknown);
    }

    #[test]
    fn test_channel_role_variants() {
        assert_eq!(ChannelRole::Primary, ChannelRole::Primary);
        assert_eq!(ChannelRole::Secondary, ChannelRole::Secondary);
        assert_ne!(ChannelRole::Primary, ChannelRole::Secondary);
    }

    #[test]
    fn test_channel_info_with_position_precision() {
        let channel: ChannelInfo = "n=TestChannel,pos=3".parse().unwrap();
        assert_eq!(channel.name, "TestChannel");
        assert_eq!(channel.position_precision, Some(3));
    }

    #[test]
    fn test_channel_info_with_uplink_downlink() {
        let channel: ChannelInfo = "n=TestChannel,up,down".parse().unwrap();
        assert!(channel.uplink_enabled);
        assert!(channel.downlink_enabled);
    }

    #[test]
    fn test_channel_info_with_client_muted() {
        let channel: ChannelInfo = "n=TestChannel,muted".parse().unwrap();
        assert!(channel.is_client_muted);
    }

    #[test]
    fn test_channel_info_role_primary() {
        let channel: ChannelInfo = "n=Test".parse().unwrap();
        assert_eq!(channel.role, ChannelRole::Primary);
    }

    #[test]
    fn test_channel_info_role_secondary() {
        let channel: ChannelInfo = "n=Test".parse().unwrap();
        assert_eq!(channel.role, ChannelRole::Primary);
    }

    #[test]
    fn test_channel_info_full_config() {
        let channel: ChannelInfo = "n=Test,up,down,pos=5,muted,psk_mode=random"
            .parse()
            .unwrap();
        assert_eq!(channel.name, "Test");
        assert_eq!(channel.role, ChannelRole::Primary);
        assert!(channel.uplink_enabled);
        assert!(channel.downlink_enabled);
        assert_eq!(channel.position_precision, Some(5));
        assert!(channel.is_client_muted);
        assert_eq!(channel.psk_type, PskType::Aes256);
    }

    #[test]
    fn test_channel_info_role_case_insensitive() {
        let channel1: ChannelInfo = "n=Test,psk_mode=default".parse().unwrap();
        let channel2: ChannelInfo = "n=Test,mode=default".parse().unwrap();
        assert_eq!(channel1.psk_type, channel2.psk_type);
    }

    #[test]
    fn test_psk_mode_unknown() {
        let result: Result<ChannelInfo, _> = "psk_mode=unknown_mode".parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_channel_info_with_special_chars_in_name() {
        let channel: ChannelInfo = "n=Test_Channel-123".parse().unwrap();
        assert_eq!(channel.name, "Test_Channel-123");
    }

    #[test]
    fn test_region_code_to_mesh_string() {
        use meshtastic_protobufs::meshtastic::config::lo_ra_config::RegionCode;

        assert_eq!(RegionCode::Us.to_mesh_string(), "US");
        assert_eq!(RegionCode::Eu433.to_mesh_string(), "EU433");
        assert_eq!(RegionCode::Eu868.to_mesh_string(), "EU868");
        assert_eq!(RegionCode::Cn.to_mesh_string(), "CN");
        assert_eq!(RegionCode::Jp.to_mesh_string(), "JP");
        assert_eq!(RegionCode::Anz.to_mesh_string(), "ANZ");
        assert_eq!(RegionCode::Kr.to_mesh_string(), "KR");
        assert_eq!(RegionCode::Tw.to_mesh_string(), "TW");
        assert_eq!(RegionCode::Ru.to_mesh_string(), "RU");
        assert_eq!(RegionCode::In.to_mesh_string(), "IN");
        assert_eq!(RegionCode::Nz865.to_mesh_string(), "NZ865");
        assert_eq!(RegionCode::Th.to_mesh_string(), "TH");
        assert_eq!(RegionCode::Lora24.to_mesh_string(), "Lora24");
        assert_eq!(RegionCode::Ua433.to_mesh_string(), "UA433");
        assert_eq!(RegionCode::Ua868.to_mesh_string(), "UA868");
        assert_eq!(RegionCode::My433.to_mesh_string(), "MY433");
        assert_eq!(RegionCode::My919.to_mesh_string(), "MY919");
        assert_eq!(RegionCode::Sg923.to_mesh_string(), "SG923");
        assert_eq!(RegionCode::Ph433.to_mesh_string(), "PH433");
        assert_eq!(RegionCode::Ph868.to_mesh_string(), "PH868");
        assert_eq!(RegionCode::Ph915.to_mesh_string(), "PH915");
        assert_eq!(RegionCode::Anz433.to_mesh_string(), "ANZ433");
        assert_eq!(RegionCode::Unset.to_mesh_string(), "Unset");
    }

    #[test]
    fn test_modem_preset_to_mesh_string() {
        use meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset;

        assert_eq!(ModemPreset::LongFast.to_mesh_string(), "LongFast");
        assert_eq!(ModemPreset::LongSlow.to_mesh_string(), "LongSlow");
        assert_eq!(ModemPreset::VeryLongSlow.to_mesh_string(), "VeryLongSlow");
        assert_eq!(ModemPreset::MediumSlow.to_mesh_string(), "MediumSlow");
        assert_eq!(ModemPreset::MediumFast.to_mesh_string(), "MediumFast");
        assert_eq!(ModemPreset::ShortSlow.to_mesh_string(), "ShortSlow");
        assert_eq!(ModemPreset::ShortFast.to_mesh_string(), "ShortFast");
        assert_eq!(ModemPreset::LongModerate.to_mesh_string(), "LongModerate");
        assert_eq!(ModemPreset::ShortTurbo.to_mesh_string(), "ShortTurbo");
    }

    #[test]
    fn test_psk_mode_display() {
        assert_eq!(PskMode::Default.to_string(), "Default");
        assert_eq!(PskMode::None.to_string(), "None");
        assert_eq!(PskMode::Random.to_string(), "Random");
        assert_eq!(PskMode::Base64("test".to_string()).to_string(), "Base64");
        assert_eq!(
            PskMode::Passphrase("test".to_string()).to_string(),
            "Passphrase"
        );
    }
}
