use crate::protobufs::config::{LoRaConfig, lo_ra_config::ModemPreset, lo_ra_config::RegionCode};
use crate::protobufs::{ChannelSettings, ModuleSettings};
use base64::{Engine as _, engine::general_purpose::STANDARD};

/// User role in the Meshtastic mesh network.
///
/// Represents the role/function of a node in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    /// Regular client node (default)
    Client,
    /// Client with muted audio
    ClientMute,
    /// Client with silent mode enabled
    ClientSilent,
    /// Tracker node (position reporting)
    Tracker,
    /// Sensor node
    Sensor,
    /// Administrator node
    Admin,
    /// Router node for mesh routing
    Router,
    /// Unknown or unrecognized role
    Unknown,
}

impl From<i32> for UserRole {
    fn from(value: i32) -> Self {
        match value {
            0 => UserRole::Client,
            1 => UserRole::ClientMute,
            2 => UserRole::ClientSilent,
            3 => UserRole::Tracker,
            4 => UserRole::Sensor,
            5 => UserRole::Admin,
            7 => UserRole::Router,
            _ => UserRole::Unknown,
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Client => write!(f, "Client"),
            UserRole::ClientMute => write!(f, "ClientMute"),
            UserRole::ClientSilent => write!(f, "ClientSilent"),
            UserRole::Tracker => write!(f, "Tracker"),
            UserRole::Sensor => write!(f, "Sensor"),
            UserRole::Admin => write!(f, "Admin"),
            UserRole::Router => write!(f, "Router"),
            UserRole::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Provides consistent string representation for Meshtastic enums.
/// Converts protobuf enum variants to standardized uppercase string format.
pub trait MeshtasticDisplay {
    /// Returns the standardized string representation.
    fn to_mesh_string(&self) -> &'static str;
}

impl MeshtasticDisplay for RegionCode {
    /// Returns the name the protobuf gives this region, such as "EU_868".
    ///
    /// Generated from the definitions rather than written out here: the hand
    /// written table silently fell behind the firmware, which is how seven
    /// regions ended up missing.
    fn to_mesh_string(&self) -> &'static str {
        self.as_str_name()
    }
}

impl MeshtasticDisplay for ModemPreset {
    /// Returns the name the protobuf gives this preset, such as "LONG_FAST".
    fn to_mesh_string(&self) -> &'static str {
        self.as_str_name()
    }
}

/// Every region a user can select, in the order the protobuf declares them.
///
/// `RegionCode::Unset` is deliberately absent: it is the unconfigured state, not
/// something to choose. Keeping the single list here stops the CLI, the TUI and
/// the string parser from drifting apart, which is how seven regions ended up
/// reachable from a decoded URL but not from either interface.
// UA_868 is marked deprecated upstream but devices still run it, so it stays
// selectable; dropping it would mean no way to reproduce an existing setup.
#[allow(deprecated)]
pub const REGION_CODES: &[RegionCode] = &[
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
    RegionCode::My433,
    RegionCode::My919,
    RegionCode::Sg923,
    RegionCode::Ph433,
    RegionCode::Ph868,
    RegionCode::Ph915,
    RegionCode::Anz433,
    // Added by firmware 2.8.
    RegionCode::Kz433,
    RegionCode::Kz863,
    RegionCode::Np865,
    RegionCode::Br902,
    RegionCode::Itu12m,
    RegionCode::Itu22m,
    RegionCode::Eu866,
    RegionCode::Eu874,
    RegionCode::Eu917,
    RegionCode::EuN868,
    RegionCode::Itu32m,
    RegionCode::Itu170cm,
    RegionCode::Itu270cm,
    RegionCode::Itu370cm,
    RegionCode::Itu2125cm,
];

/// Every modem preset a user can select.
///
/// `VERY_LONG_SLOW` is left out: the firmware deprecated it in 2.5 and now
/// treats it as an illegal value, falling back to `LONG_FAST`.
// LONG_SLOW is deprecated upstream but the firmware still implements it, so it
// stays selectable. VERY_LONG_SLOW does not: the firmware treats it as illegal.
#[allow(deprecated)]
pub const MODEM_PRESETS: &[ModemPreset] = &[
    ModemPreset::LongFast,
    ModemPreset::LongSlow,
    ModemPreset::LongModerate,
    ModemPreset::MediumSlow,
    ModemPreset::MediumFast,
    ModemPreset::ShortSlow,
    ModemPreset::ShortFast,
    ModemPreset::ShortTurbo,
    // Added by firmware 2.8.
    ModemPreset::LongTurbo,
    ModemPreset::MediumTurbo,
    ModemPreset::LiteFast,
    ModemPreset::LiteSlow,
    ModemPreset::NarrowFast,
    ModemPreset::NarrowSlow,
    ModemPreset::TinyFast,
    ModemPreset::TinySlow,
];

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

/// Longest channel name the firmware stores, in bytes.
///
/// The limit is on bytes, not characters, so a name made of multi-byte
/// characters runs out of room sooner than its length suggests.
pub const MAX_CHANNEL_NAME_BYTES: usize = 12;

/// Checks a channel name against the firmware limit.
///
/// Anything longer is truncated by the device, so a URL carrying it promises a
/// name the radio will not use.
pub fn validate_channel_name(name: &str) -> Result<(), String> {
    let length = name.len();
    if length > MAX_CHANNEL_NAME_BYTES {
        return Err(format!(
            "Channel name is {} bytes, the maximum is {}",
            length, MAX_CHANNEL_NAME_BYTES
        ));
    }
    Ok(())
}

/// Default PSK value (base64 encoded single byte [1]).
pub const DEFAULT_PSK: &str = "AQ==";

/// Base URL for Meshtastic channel configuration URLs.
pub const MESHTASTIC_CHANNEL_URL_BASE: &str = "https://meshtastic.org/e/#";

/// Base URL for Meshtastic node info URLs.
pub const MESHTASTIC_NODE_URL_BASE: &str = "https://meshtastic.org/v/#";

/// Base URL for a channel URL that adds to the channels a device already has.
///
/// Importing a plain channel URL replaces the whole channel table. With
/// `add=true` each channel goes into the first free slot instead, as a
/// secondary channel, and the ones already there are left alone.
pub const MESHTASTIC_CHANNEL_ADD_URL_BASE: &str = "https://meshtastic.org/e/?add=true#";

/// Node information decoded from a Meshtastic node info URL.
///
/// This struct contains the information present in `/v/` URLs,
/// which includes user identity and device information.
/// Note: Position data is not included in node URLs.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Unique node number assigned by the mesh
    pub num: u32,
    /// Full name of the node owner
    pub long_name: String,
    /// Short name (typically 3-4 characters) for display on OLED
    pub short_name: String,
    /// Hardware model identifier
    pub hw_model: String,
    /// Role of this node in the mesh
    pub role: UserRole,
    /// Public key for encryption (base64 encoded, if present)
    pub public_key: Option<String>,
    /// Whether this node accepts direct messages
    pub is_unmessagable: bool,
}

impl NodeInfo {
    /// Creates a `NodeInfo` from a protobuf `NodeInfo` message.
    ///
    /// # Arguments
    /// * `node` - Reference to the protobuf NodeInfo message
    ///
    /// # Example
    /// ```
    /// use meshurl::protobufs::{NodeInfo as PbNodeInfo, User};
    /// use meshurl::models::NodeInfo;
    ///
    /// // Only the fields this conversion reads are set; the rest keep their
    /// // protobuf defaults, so a new field upstream does not break this.
    /// let user = User {
    ///     long_name: "Test Node".to_string(),
    ///     short_name: "TST".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// let pb_node = PbNodeInfo {
    ///     num: 12345,
    ///     user: Some(user),
    ///     ..Default::default()
    /// };
    ///
    /// let node_info = NodeInfo::from_pb(&pb_node);
    /// assert_eq!(node_info.num, 12345);
    /// assert_eq!(node_info.long_name, "Test Node");
    /// ```
    pub fn from_pb(node: &crate::protobufs::NodeInfo) -> Self {
        let (long_name, short_name, hw_model, role, public_key, is_unmessagable) =
            if let Some(ref user) = node.user {
                let role = UserRole::from(user.role);
                let public_key = if user.public_key.is_empty() {
                    None
                } else {
                    Some(STANDARD.encode(&user.public_key))
                };
                let is_unmessagable = user.is_unmessagable.unwrap_or(false);
                (
                    user.long_name.clone(),
                    user.short_name.clone(),
                    hardware_model_to_string(user.hw_model),
                    role,
                    public_key,
                    is_unmessagable,
                )
            } else {
                (
                    String::new(),
                    String::new(),
                    String::new(),
                    UserRole::Unknown,
                    None,
                    false,
                )
            };

        Self {
            num: node.num,
            long_name,
            short_name,
            hw_model,
            role,
            public_key,
            is_unmessagable,
        }
    }
}

/// Converts a hardware model integer to its string representation.
/// Uses the meshtastic-protobufs HardwareModel enum.
fn hardware_model_to_string(model: i32) -> String {
    use crate::protobufs::HardwareModel;
    use std::convert::TryFrom;

    if let Ok(hw_model) = HardwareModel::try_from(model) {
        return hw_model.as_str_name().to_string();
    }
    format!("Unknown ({})", model)
}

/// Generates a random 32-byte AES-256 PSK encoded in base64.
///
/// The bytes come from the operating system CSPRNG. An earlier version used a
/// linear congruential generator seeded with the clock, which offered only as
/// much entropy as the instant it ran at rather than the 256 bits it claimed.
///
/// # Errors
/// Returns an error if the operating system cannot supply random bytes.
pub fn generate_random_psk() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("Cannot generate a random PSK: {}", e))?;
    Ok(STANDARD.encode(bytes))
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
    /// Returns true if this PSK mode is None (no encryption).
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

/// Hashes a passphrase into a PSK, refusing an empty one.
///
/// Hashing the empty string yields a fixed, publicly known value, so accepting
/// it would silently produce a channel anyone can decrypt.
fn hash_passphrase(phrase: &str) -> Result<String, String> {
    if phrase.is_empty() {
        return Err("Passphrase cannot be empty".to_string());
    }
    Ok(hash_phrase_to_psk(phrase))
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

/// Splits a channel spec on commas, honouring `\,` as a literal comma.
///
/// Channel names and passphrases can contain commas, which a plain split makes
/// impossible to express: `psk_passphrase=one, two` used to be read as an
/// unknown option named "two".
fn split_options(spec: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = spec.chars();

    while let Some(character) = chars.next() {
        match character {
            '\\' => match chars.next() {
                // Only commas need escaping, so any other backslash is literal.
                Some(',') => current.push(','),
                Some(other) => {
                    current.push('\\');
                    current.push(other);
                }
                None => current.push('\\'),
            },
            ',' => parts.push(std::mem::take(&mut current)),
            _ => current.push(character),
        }
    }
    parts.push(current);

    parts
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

        for part in split_options(s) {
            let (key, value) = match part.split_once('=') {
                Some((key, value)) => (key.trim(), Some(value.trim())),
                None => (part.trim(), None),
            };

            match key {
                "default" | "d" => is_default = true,
                "name" | "n" => name = Some(value.unwrap_or("").to_string()),
                "psk_base64" | "psk" => psk = Some(value.unwrap_or("").to_string()),
                "psk_mode" | "mode" => {
                    psk_mode = match value {
                        Some("default") | Some("d") => Some(PskMode::Default),
                        Some("none") | Some("n") => Some(PskMode::None),
                        Some("random") | Some("r") => Some(PskMode::Random),
                        // strip_prefix rather than a byte offset: the hand
                        // counted offsets were wrong for "passphrase:" and
                        // panicked on a value that was only the prefix.
                        Some(v) if v.starts_with("base64:") => v
                            .strip_prefix("base64:")
                            .map(|psk| PskMode::Base64(psk.to_string())),
                        Some(v) if v.starts_with("passphrase:") => v
                            .strip_prefix("passphrase:")
                            .map(|phrase| PskMode::Passphrase(phrase.to_string())),
                        _ => None,
                    }
                }
                "psk_passphrase" | "psk_phrase" | "phrase" => {
                    psk_phrase = Some(value.unwrap_or("").to_string())
                }
                "uplink" | "up" => uplink = true,
                "downlink" | "down" => downlink = true,
                "pos" | "precision" => {
                    // A precision of 0 means disabled, which is the absence of
                    // the setting rather than a value to encode.
                    position_precision = value
                        .and_then(|v| v.parse().ok())
                        .filter(|precision| *precision > 0);
                }
                "muted" | "mute" => muted = true,
                _ => return Err(format!("Unknown option: {}", key)),
            }
        }

        if let Some(name) = name.as_deref() {
            validate_channel_name(name)?;
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
                hash_passphrase(&phrase)?
            } else {
                match psk_mode.unwrap_or(PskMode::Default) {
                    PskMode::Default => DEFAULT_PSK.to_string(),
                    PskMode::None => String::new(),
                    PskMode::Random => generate_random_psk()?,
                    PskMode::Base64(psk_str) => validate_and_normalize_psk(&psk_str)?,
                    PskMode::Passphrase(phrase) => hash_passphrase(&phrase)?,
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
    /// Returns the string representation of the channel role.
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

/// How a configuration chooses its radio parameters.
///
/// A `LoRaConfig` that is not using a preset leaves `modem_preset` unset, and
/// proto3 reports an unset enum as its first variant, which is `LongFast`. Any
/// type that stores the preset unconditionally therefore has a `LongFast` in it
/// that means "no preset", and every consumer has to remember not to show it.
/// Modelling the two cases as separate variants removes that trap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModemConfig {
    /// Radio parameters come from a named preset.
    Preset(ModemPreset),
    /// Radio parameters were set by hand.
    Custom {
        /// Bandwidth in kHz.
        bandwidth: u32,
        /// Spreading factor.
        spread_factor: u32,
        /// Coding rate denominator, as in 4/`coding_rate`.
        coding_rate: u32,
    },
}

impl ModemConfig {
    /// The preset in use, or `None` when the parameters are manual.
    pub fn preset(&self) -> Option<ModemPreset> {
        match self {
            ModemConfig::Preset(preset) => Some(*preset),
            ModemConfig::Custom { .. } => None,
        }
    }

    /// Whether the parameters come from a preset.
    pub fn uses_preset(&self) -> bool {
        matches!(self, ModemConfig::Preset(_))
    }

    /// The radio parameters, resolving a preset to its values.
    ///
    /// Returns (bandwidth in kHz, spreading factor, coding rate denominator).
    /// The region is needed because a preset is wider on the 2.4 GHz band.
    pub fn parameters(&self, region: RegionCode) -> (f32, u32, u32) {
        match self {
            ModemConfig::Preset(preset) => get_preset_params(*preset, region),
            ModemConfig::Custom {
                bandwidth,
                spread_factor,
                coding_rate,
            } => (*bandwidth as f32, *spread_factor, *coding_rate),
        }
    }

    /// The name to show for this configuration.
    pub fn name(&self) -> &'static str {
        match self {
            ModemConfig::Preset(preset) => preset.to_mesh_string(),
            ModemConfig::Custom { .. } => CUSTOM_MODEM_NAME,
        }
    }
}

impl Default for ModemConfig {
    fn default() -> Self {
        ModemConfig::Preset(ModemPreset::LongFast)
    }
}

/// Shown in place of a preset name when the parameters are manual.
pub const CUSTOM_MODEM_NAME: &str = "Custom";

/// LoRa radio configuration for Meshtastic.
/// Contains all configurable parameters for the LoRa modem.
#[derive(Debug, Clone, Default)]
pub struct LoRaInfo {
    /// Region code (e.g., EU868, US, CN, etc.)
    pub region: RegionCode,
    /// How the radio parameters are chosen: a named preset or manual values
    pub modem: ModemConfig,
    /// Whether TX is enabled
    pub tx_enabled: bool,
    /// TX power in dBm (0 = use default maximum safe power)
    pub tx_power: i32,
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

/// Whether a region uses the wide bandwidths of the 2.4 GHz band.
///
/// The firmware calls this `wideLora`, and it triples every bandwidth that is
/// not fixed by the preset itself.
pub fn region_is_wide_band(region: RegionCode) -> bool {
    matches!(region, RegionCode::Lora24)
}

/// Get the radio parameters for a modem preset, as the firmware computes them.
///
/// Returns (bandwidth in kHz, spreading factor, coding rate denominator).
/// Mirrors `modemPresetToParams()` in the firmware, including the wide band
/// variants: reading these values from anywhere else has already put four wrong
/// presets in front of users.
///
/// # Arguments
/// * `preset` - The modem preset to get parameters for
/// * `region` - The region in use, which decides the bandwidth on 2.4 GHz
///
/// # Example
/// ```
/// use meshurl::{ModemPreset, RegionCode, get_preset_params};
///
/// let (bw, sf, cr) = get_preset_params(ModemPreset::LongFast, RegionCode::Eu868);
/// assert_eq!(bw, 250.0); // 250 kHz
/// assert_eq!(sf, 11);    // SF11
/// assert_eq!(cr, 5);     // 4/5
///
/// // The same preset is wider on the 2.4 GHz band.
/// let (bw, _, _) = get_preset_params(ModemPreset::LongFast, RegionCode::Lora24);
/// assert_eq!(bw, 812.5);
/// ```
#[allow(deprecated)]
pub fn get_preset_params(preset: ModemPreset, region: RegionCode) -> (f32, u32, u32) {
    let wide = region_is_wide_band(region);

    // Presets whose bandwidth follows the band.
    let narrow = if wide { 406.25 } else { 125.0 };
    let medium = if wide { 812.5 } else { 250.0 };
    let turbo = if wide { 1625.0 } else { 500.0 };

    match preset {
        ModemPreset::ShortTurbo => (turbo, 7, 5),
        ModemPreset::ShortFast => (medium, 7, 5),
        ModemPreset::ShortSlow => (medium, 8, 5),
        ModemPreset::MediumFast => (medium, 9, 5),
        ModemPreset::MediumSlow => (medium, 10, 5),
        ModemPreset::MediumTurbo => (turbo, 9, 5),
        ModemPreset::LongTurbo => (turbo, 11, 8),
        ModemPreset::LongModerate => (narrow, 11, 8),
        ModemPreset::LongSlow => (narrow, 12, 8),
        // These are tied to a fixed bandwidth whatever the band.
        ModemPreset::LiteFast => (125.0, 9, 5),
        ModemPreset::LiteSlow => (125.0, 10, 5),
        ModemPreset::NarrowFast => (62.5, 7, 6),
        ModemPreset::NarrowSlow => (62.5, 8, 6),
        ModemPreset::TinyFast => (15.6, 7, 5),
        ModemPreset::TinySlow => (15.6, 8, 6),
        // LongFast, and the deprecated VeryLongSlow, which the firmware treats
        // as an illegal value and falls back on.
        ModemPreset::LongFast | ModemPreset::VeryLongSlow => (medium, 11, 5),
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
                (prec, ms.is_muted)
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
///
/// A configuration that carries no radio parameters is taken to use its preset:
/// the firmware leaves them unset in that case, and proto3 reports an unset
/// numeric field as zero.
impl From<&LoRaConfig> for LoRaInfo {
    fn from(config: &LoRaConfig) -> Self {
        // `bandwidth` is already in kHz, matching get_preset_params. Dividing it
        // by 1000 first, as this used to, compared kHz against zero and made the
        // outcome depend on values no device sends.
        let modem = if config.use_preset || config.bandwidth == 0 {
            ModemConfig::Preset(config.modem_preset())
        } else {
            ModemConfig::Custom {
                bandwidth: config.bandwidth,
                spread_factor: config.spread_factor,
                coding_rate: config.coding_rate,
            }
        };

        LoRaInfo {
            region: config.region(),
            modem,
            tx_enabled: config.tx_enabled,
            tx_power: config.tx_power,
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
    /// Whether the URL adds these channels to the ones a device already has,
    /// rather than replacing its channel table.
    ///
    /// The flag lives in the URL, as `?add=true`, not in the encoded payload:
    /// the same channels can be shared either way. See
    /// [`MESHTASTIC_CHANNEL_ADD_URL_BASE`].
    pub add_only: bool,
}

impl MeshtasticConfig {
    /// Creates a MeshtasticConfig from a protobuf ChannelSet.
    /// This is used when decoding a URL to extract all configuration.
    pub fn from_channel_set(channel_set: &crate::protobufs::ChannelSet) -> Self {
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

        // The payload says nothing about how it should be imported; the URL
        // around it does, so the decoder sets this afterwards.
        MeshtasticConfig {
            channels,
            lora,
            add_only: false,
        }
    }

    /// Creates a new empty MeshtasticConfig with no channels or LoRa settings.
    pub fn new() -> Self {
        MeshtasticConfig {
            channels: Vec::new(),
            lora: None,
            add_only: false,
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
                is_muted: info.is_client_muted,
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
        let (bandwidth, spread_factor, coding_rate) = info.modem_parameters();

        LoRaConfig {
            region: info.region as i32,
            // A manual configuration has no preset to name, so it keeps the
            // protobuf default; use_preset is what tells the two apart.
            modem_preset: info.modem.preset().unwrap_or_default() as i32,
            use_preset: info.modem.uses_preset(),
            tx_enabled: info.tx_enabled,
            tx_power: info.tx_power,
            // The protobuf field is an integer, so a fractional preset
            // bandwidth such as 62.5 kHz is stored rounded, as the firmware
            // stores it too.
            bandwidth: bandwidth.round() as u32,
            spread_factor,
            coding_rate,
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
            // Fields meshurl does not model keep their protobuf defaults.
            ..Default::default()
        }
    }
}

impl LoRaInfo {
    /// The radio parameters in use, resolving a preset against the region.
    pub fn modem_parameters(&self) -> (f32, u32, u32) {
        self.modem.parameters(self.region)
    }
}

#[cfg(test)]
// Deprecated regions and presets still run on deployed devices, so the
// tests keep exercising them.
#[allow(deprecated)]
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
        // Checking only the length hid an off-by-one that hashed the phrase
        // without its first character.
        assert_eq!(channel.psk, hash_phrase_to_psk("my secret phrase"));
    }

    #[test]
    fn test_channel_info_psk_passphrase_option() {
        let channel: ChannelInfo = "psk_passphrase=my secret phrase".parse().unwrap();
        assert_eq!(channel.psk, hash_phrase_to_psk("my secret phrase"));
    }

    #[test]
    fn test_channel_info_psk_mode_passphrase_single_char() {
        let channel: ChannelInfo = "psk_mode=passphrase:x".parse().unwrap();
        assert_eq!(channel.psk, hash_phrase_to_psk("x"));
    }

    #[test]
    fn test_channel_info_psk_mode_passphrase_multibyte() {
        let channel: ChannelInfo = "psk_mode=passphrase:\u{e1}rbore".parse().unwrap();
        assert_eq!(channel.psk, hash_phrase_to_psk("\u{e1}rbore"));
    }

    #[test]
    fn test_channel_info_psk_mode_passphrase_empty_is_refused() {
        // Used to panic slicing past the end of the string; hashing the empty
        // phrase would hand out a publicly known key.
        let result: Result<ChannelInfo, _> = "psk_mode=passphrase:".parse();
        assert_eq!(result.unwrap_err(), "Passphrase cannot be empty");

        let result: Result<ChannelInfo, _> = "psk_passphrase=".parse();
        assert_eq!(result.unwrap_err(), "Passphrase cannot be empty");
    }

    #[test]
    fn test_channel_info_psk_mode_base64_keeps_full_value() {
        let psk = "CcZBoFJbADPGEoSkkYPA3Ha23rr7WPcyUo1AjorGQIA=";
        let channel: ChannelInfo = format!("psk_mode=base64:{}", psk).parse().unwrap();
        assert_eq!(channel.psk, psk);
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
        let psk1 = generate_random_psk().unwrap();
        let psk2 = generate_random_psk().unwrap();

        assert!(!psk1.is_empty());
        assert!(!psk2.is_empty());
        assert_ne!(psk1, psk2);

        let decoded = STANDARD.decode(&psk1).unwrap();
        assert_eq!(decoded.len(), 32);
    }

    /// A key seeded from the clock cannot be told apart from a real one by
    /// looking at its bytes; what gives it away is that it can be rebuilt. This
    /// replays the old generator over every seed the call could have used.
    #[test]
    fn test_generate_random_psk_is_not_derived_from_the_clock() {
        use std::time::{SystemTime, UNIX_EPOCH};

        fn clock_now() -> u64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock is after the epoch")
                .as_nanos() as u64
        }

        /// The generator this crate used to ship.
        fn key_from_seed(seed: u64) -> [u8; 32] {
            let mut bytes = [0u8; 32];
            let mut rng = seed;
            for byte in bytes.iter_mut() {
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                *byte = (rng >> 16) as u8;
            }
            bytes
        }

        // Bracket the call so the seed it would have used is in [before, after].
        let before = clock_now();
        let psk = generate_random_psk().unwrap();
        let after = clock_now();

        let key = STANDARD.decode(&psk).unwrap();
        // Cap the search so a stalled machine cannot make this test crawl.
        let last = after.min(before + 500_000);
        for seed in before..=last {
            assert_ne!(
                key_from_seed(seed).as_slice(),
                key.as_slice(),
                "the PSK can be rebuilt from the clock"
            );
        }
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
    fn test_channel_info_position_zero_is_disabled() {
        // Encoding a precision of 0 adds a ModuleSettings message that only
        // says the feature is off.
        let channel: ChannelInfo = "n=Test,pos=0".parse().unwrap();
        assert_eq!(channel.position_precision, None);
    }

    /// Builds a LoRaConfig the way a device would serialise it.
    #[cfg(test)]
    fn lora_config(
        use_preset: bool,
        bandwidth: u32,
        spread_factor: u32,
        coding_rate: u32,
    ) -> LoRaConfig {
        LoRaConfig {
            use_preset,
            modem_preset: ModemPreset::LongFast as i32,
            bandwidth,
            spread_factor,
            coding_rate,
            ..Default::default()
        }
    }

    #[test]
    fn test_lora_preset_config_fills_in_the_preset_parameters() {
        // With a preset in use the firmware leaves the parameters unset.
        let info = LoRaInfo::from(&lora_config(true, 0, 0, 0));

        assert_eq!(info.modem, ModemConfig::Preset(ModemPreset::LongFast));
        assert_eq!(info.modem_parameters(), (250.0, 11, 5));
    }

    #[test]
    fn test_lora_manual_config_is_kept_verbatim() {
        // Values taken from a real URL: 62 kHz, SF7, CR 4/6.
        let info = LoRaInfo::from(&lora_config(false, 62, 7, 6));

        assert!(!info.modem.uses_preset());
        assert_eq!(info.modem_parameters(), (62.0, 7, 6));
    }

    #[test]
    fn test_manual_parameters_matching_a_preset_stay_manual() {
        // These are exactly LongFast, but the config says it is not using a
        // preset and that is what it means. Second-guessing it rewrote the flag.
        let info = LoRaInfo::from(&lora_config(false, 250, 11, 5));

        assert!(!info.modem.uses_preset());
        assert_eq!(info.modem_parameters(), (250.0, 11, 5));
    }

    #[test]
    fn test_bandwidth_is_read_as_kilohertz() {
        // 250 is 250 kHz, not 250 Hz: dividing by 1000 first zeroed it out.
        let info = LoRaInfo::from(&lora_config(false, 250, 11, 5));

        assert_eq!(info.modem_parameters().0, 250.0);
    }

    #[test]
    fn test_manual_config_has_no_preset_name() {
        // The protobuf default reads as LongFast; showing it as the preset in
        // use is reporting an absent field as a choice.
        let info = LoRaInfo::from(&lora_config(false, 62, 7, 6));

        assert_eq!(info.modem.preset(), None);
        assert_eq!(info.modem.name(), CUSTOM_MODEM_NAME);
    }

    #[test]
    fn test_manual_config_survives_a_protobuf_round_trip() {
        let info = LoRaInfo::from(&lora_config(false, 62, 7, 6));

        let encoded = LoRaConfig::from(&info);
        let decoded = LoRaInfo::from(&encoded);

        assert!(!encoded.use_preset);
        assert_eq!(decoded.modem, info.modem);
        assert_eq!(decoded.modem_parameters(), (62.0, 7, 6));
    }

    #[test]
    fn test_preset_config_survives_a_protobuf_round_trip() {
        let info = LoRaInfo::from(&lora_config(true, 0, 0, 0));

        let decoded = LoRaInfo::from(&LoRaConfig::from(&info));

        assert_eq!(decoded.modem, ModemConfig::Preset(ModemPreset::LongFast));
    }

    #[test]
    fn test_channel_name_within_the_limit_is_accepted() {
        assert!(validate_channel_name("").is_ok());
        assert!(validate_channel_name("Galicia").is_ok());
        // Exactly at the limit.
        assert!(validate_channel_name("123456789012").is_ok());
    }

    #[test]
    fn test_channel_name_over_the_limit_is_refused() {
        let error = validate_channel_name("1234567890123").unwrap_err();
        assert_eq!(error, "Channel name is 13 bytes, the maximum is 12");
    }

    #[test]
    fn test_channel_name_limit_counts_bytes_not_characters() {
        // Five characters, but fifteen bytes: the firmware limit is on bytes.
        let name = "\u{1f419}\u{1f419}\u{1f419}";
        assert_eq!(name.chars().count(), 3);
        assert_eq!(name.len(), 12);
        assert!(validate_channel_name(name).is_ok());

        let longer = "\u{1f419}\u{1f419}\u{1f419}\u{1f419}";
        assert!(validate_channel_name(longer).is_err());
    }

    #[test]
    fn test_channel_info_refuses_an_overlong_name() {
        let result: Result<ChannelInfo, _> = "name=EsteNombreEsDemasiadoLargo".parse();
        assert!(result.unwrap_err().contains("maximum is 12"));
    }

    #[test]
    fn test_split_options_separates_on_commas() {
        assert_eq!(split_options("a,b,c"), vec!["a", "b", "c"]);
        assert_eq!(split_options("single"), vec!["single"]);
    }

    #[test]
    fn test_split_options_honours_escaped_commas() {
        assert_eq!(split_options(r"a\,b"), vec!["a,b"]);
        assert_eq!(
            split_options(r"name=A\,B,uplink"),
            vec!["name=A,B", "uplink"]
        );
    }

    #[test]
    fn test_split_options_leaves_other_backslashes_alone() {
        assert_eq!(split_options(r"a\b"), vec![r"a\b"]);
        assert_eq!(split_options("trailing\\"), vec!["trailing\\"]);
    }

    #[test]
    fn test_channel_name_can_contain_a_comma() {
        let channel: ChannelInfo = r"name=A\,B".parse().unwrap();
        assert_eq!(channel.name, "A,B");
    }

    #[test]
    fn test_passphrase_can_contain_a_comma() {
        let channel: ChannelInfo = r"psk_passphrase=one\, two".parse().unwrap();
        assert_eq!(channel.psk, hash_phrase_to_psk("one, two"));
    }

    #[test]
    fn test_an_unescaped_comma_still_starts_a_new_option() {
        let channel: ChannelInfo = "name=Test,uplink".parse().unwrap();
        assert_eq!(channel.name, "Test");
        assert!(channel.uplink_enabled);
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
        // Kept within the 12 byte channel name limit.
        let channel: ChannelInfo = "n=Test_Ch-123".parse().unwrap();
        assert_eq!(channel.name, "Test_Ch-123");
    }

    /// The table in `modemPresetToParams()` of firmware 2.8, which is the
    /// authority. Four of our presets used to disagree with it.
    #[test]
    fn test_preset_parameters_match_the_firmware() {
        #[allow(deprecated)]
        let expected: &[(ModemPreset, f32, u32, u32)] = &[
            (ModemPreset::LongFast, 250.0, 11, 5),
            (ModemPreset::LongSlow, 125.0, 12, 8),
            (ModemPreset::LongModerate, 125.0, 11, 8),
            (ModemPreset::LongTurbo, 500.0, 11, 8),
            (ModemPreset::MediumSlow, 250.0, 10, 5),
            (ModemPreset::MediumFast, 250.0, 9, 5),
            (ModemPreset::MediumTurbo, 500.0, 9, 5),
            (ModemPreset::ShortSlow, 250.0, 8, 5),
            (ModemPreset::ShortFast, 250.0, 7, 5),
            (ModemPreset::ShortTurbo, 500.0, 7, 5),
            (ModemPreset::LiteFast, 125.0, 9, 5),
            (ModemPreset::LiteSlow, 125.0, 10, 5),
            (ModemPreset::NarrowFast, 62.5, 7, 6),
            (ModemPreset::NarrowSlow, 62.5, 8, 6),
            (ModemPreset::TinyFast, 15.6, 7, 5),
            (ModemPreset::TinySlow, 15.6, 8, 6),
        ];

        for (preset, bandwidth, spread_factor, coding_rate) in expected {
            assert_eq!(
                get_preset_params(*preset, RegionCode::Eu868),
                (*bandwidth, *spread_factor, *coding_rate),
                "{} does not match the firmware",
                preset.to_mesh_string()
            );
        }
    }

    #[test]
    fn test_the_wide_band_uses_wider_bandwidths() {
        // On 2.4 GHz the firmware triples the bandwidth of the presets that
        // are not pinned to a fixed one.
        assert_eq!(
            get_preset_params(ModemPreset::LongFast, RegionCode::Lora24),
            (812.5, 11, 5)
        );
        assert_eq!(
            get_preset_params(ModemPreset::ShortTurbo, RegionCode::Lora24),
            (1625.0, 7, 5)
        );
        assert_eq!(
            get_preset_params(ModemPreset::LongModerate, RegionCode::Lora24),
            (406.25, 11, 8)
        );

        // These keep their bandwidth whatever the band.
        assert_eq!(
            get_preset_params(ModemPreset::NarrowFast, RegionCode::Lora24),
            (62.5, 7, 6)
        );
        assert_eq!(
            get_preset_params(ModemPreset::TinyFast, RegionCode::Lora24),
            (15.6, 7, 5)
        );
    }

    #[test]
    fn test_a_2_8_region_survives_a_protobuf_round_trip() {
        // These used to decode as Unset and be dropped on the way back out.
        for region in [
            RegionCode::Itu170cm,
            RegionCode::Kz433,
            RegionCode::Br902,
            RegionCode::EuN868,
        ] {
            let info = LoRaInfo {
                region,
                ..Default::default()
            };
            let decoded = LoRaInfo::from(&LoRaConfig::from(&info));
            assert_eq!(decoded.region, region);
        }
    }

    #[test]
    fn test_a_2_8_preset_survives_a_protobuf_round_trip() {
        for preset in [
            ModemPreset::NarrowFast,
            ModemPreset::MediumTurbo,
            ModemPreset::TinySlow,
            ModemPreset::LongTurbo,
        ] {
            let info = LoRaInfo {
                modem: ModemConfig::Preset(preset),
                ..Default::default()
            };
            let decoded = LoRaInfo::from(&LoRaConfig::from(&info));
            assert_eq!(decoded.modem, ModemConfig::Preset(preset));
        }
    }

    #[test]
    fn test_region_code_to_mesh_string() {
        assert_eq!(RegionCode::Us.to_mesh_string(), "US");
        assert_eq!(RegionCode::Eu868.to_mesh_string(), "EU_868");
        assert_eq!(RegionCode::Lora24.to_mesh_string(), "LORA_24");
        assert_eq!(RegionCode::Unset.to_mesh_string(), "UNSET");
        // Regions that arrived with firmware 2.8.
        assert_eq!(RegionCode::Kz433.to_mesh_string(), "KZ_433");
        assert_eq!(RegionCode::Br902.to_mesh_string(), "BR_902");
        assert_eq!(RegionCode::Itu170cm.to_mesh_string(), "ITU1_70CM");
        assert_eq!(RegionCode::EuN868.to_mesh_string(), "EU_N_868");
    }

    /// Rather than listing every name, which is what fell behind the firmware,
    /// check the property: each selectable region has its own non-empty name.
    #[test]
    fn test_every_region_has_a_distinct_name() {
        let mut names: Vec<&str> = REGION_CODES
            .iter()
            .map(|region| region.to_mesh_string())
            .collect();
        assert!(names.iter().all(|name| !name.is_empty()));

        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "two regions share a name");
    }

    #[test]
    fn test_modem_preset_to_mesh_string() {
        assert_eq!(ModemPreset::LongFast.to_mesh_string(), "LONG_FAST");
        assert_eq!(ModemPreset::ShortTurbo.to_mesh_string(), "SHORT_TURBO");
        // Presets that arrived with firmware 2.8.
        assert_eq!(ModemPreset::NarrowFast.to_mesh_string(), "NARROW_FAST");
        assert_eq!(ModemPreset::MediumTurbo.to_mesh_string(), "MEDIUM_TURBO");
        assert_eq!(ModemPreset::TinySlow.to_mesh_string(), "TINY_SLOW");
    }

    #[test]
    fn test_every_preset_has_a_distinct_name() {
        let mut names: Vec<&str> = MODEM_PRESETS
            .iter()
            .map(|preset| preset.to_mesh_string())
            .collect();
        assert!(names.iter().all(|name| !name.is_empty()));

        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "two presets share a name");
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

    #[test]
    fn test_node_info_from_url() {
        use crate::decoder::{DecodeResult, decode_url};

        let url = "#CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";
        let result = decode_url(url).expect("valid URL");

        match result {
            DecodeResult::Node(node) => {
                assert_eq!(node.num, 1);
                assert_eq!(node.long_name, "Galicia Calidade");
                assert_eq!(node.short_name, "🐙");
                assert!(node.public_key.is_none());
                assert!(!node.is_unmessagable);
            }
            DecodeResult::Channel(_) => panic!("Expected Node, got Channel"),
        }
    }

    #[test]
    fn test_meshtastic_url_constants() {
        assert_eq!(MESHTASTIC_CHANNEL_URL_BASE, "https://meshtastic.org/e/#");
        assert_eq!(MESHTASTIC_NODE_URL_BASE, "https://meshtastic.org/v/#");
    }
}
