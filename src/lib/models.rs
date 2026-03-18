use base64::{engine::general_purpose::STANDARD, Engine as _};
use meshtastic_protobufs::meshtastic::config::{
    lo_ra_config::ModemPreset, lo_ra_config::RegionCode, LoRaConfig,
};
use meshtastic_protobufs::meshtastic::{ChannelSettings, ModuleSettings};

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

/// PSK (Pre-Shared Key) mode for channel encryption.
/// - Default: uses the default (weak, public) key
/// - None: no encryption
/// - Random: generates a secure random key (AES-256)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PskMode {
    Default,
    None,
    Random,
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
/// let channel = ChannelInfo::from_str("name=Test,psk_mode=random,uplink").unwrap();
/// assert_eq!(channel.name, "Test");
/// assert_eq!(channel.psk_mode, PskMode::Random);
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
                "psk" => psk = Some(value.unwrap_or("").to_string()),
                "psk_mode" | "mode" => {
                    psk_mode = match value {
                        Some("default") | Some("d") => Some(PskMode::Default),
                        Some("none") | Some("n") => Some(PskMode::None),
                        Some("random") | Some("r") => Some(PskMode::Random),
                        _ => None,
                    }
                }
                "psk_phrase" | "phrase" => psk_phrase = Some(value.unwrap_or("").to_string()),
                "uplink" | "up" => uplink = true,
                "downlink" | "down" => downlink = true,
                "pos" | "precision" => {
                    position_precision = value.and_then(|v| v.parse().ok());
                }
                "muted" | "mute" => muted = true,
                _ => return Err(format!("Unknown option: {}", key)),
            }
        }

        let (name, psk) = if is_default {
            (String::new(), DEFAULT_PSK.to_string())
        } else if name.is_none() && psk.is_none() && psk_mode.is_none() && psk_phrase.is_none() {
            (String::new(), DEFAULT_PSK.to_string())
        } else {
            let final_name = name.unwrap_or_default();
            let final_psk = if let Some(p) = psk {
                STANDARD
                    .decode(&p)
                    .map_err(|_| "Invalid Base64 PSK".to_string())
                    .and_then(|bytes| match bytes.len() {
                        16 | 32 => Ok(STANDARD.encode(&bytes)),
                        n => Err(format!("Invalid PSK length: {} bytes", n)),
                    })?
            } else if let Some(phrase) = psk_phrase {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(phrase.as_bytes());
                let result = hasher.finalize();
                STANDARD.encode(result.as_slice())
            } else {
                match psk_mode.unwrap_or(PskMode::Default) {
                    PskMode::Default => DEFAULT_PSK.to_string(),
                    PskMode::None => String::new(),
                    PskMode::Random => {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let seed = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as u64;
                        let mut bytes = vec![0u8; 32];
                        let mut rng = seed;
                        for byte in bytes.iter_mut() {
                            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                            *byte = (rng >> 16) as u8;
                        }
                        STANDARD.encode(&bytes)
                    }
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
                n if n >= 2 && n <= 10 => PskType::Simple(n),
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
            STANDARD.decode(&info.psk).unwrap_or_default()
        };

        let module_settings = if info.position_precision.is_some() || info.is_client_muted {
            let mut ms = ModuleSettings::default();
            ms.position_precision = info.position_precision.unwrap_or(0);
            ms.is_client_muted = info.is_client_muted;
            Some(ms)
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
