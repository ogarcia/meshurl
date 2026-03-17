mod formatter;
mod tui;

use clap::{Parser, ValueEnum};
use meshurl::{
    decode_url, encoder, errors::EncodeError, get_preset_params, ChannelInfo, ChannelRole,
    LoRaInfo, MeshtasticConfig,
};

#[derive(Parser, Debug)]
#[command(name = "meshurl")]
#[command(about = "Decode and encode Meshtastic channel URLs")]
enum Cli {
    #[command(about = "Decode a Meshtastic channel URL")]
    Decode { url: String },

    #[command(about = "Encode a Meshtastic channel URL")]
    Encode(EncodeArgs),

    #[command(about = "Open TUI interface")]
    Tui,
}

#[derive(Parser, Debug)]
struct EncodeArgs {
    #[arg(
        long,
        short = 'c',
        help = "Add a channel. Options: default, name=N, psk=XXX, psk_mode=default|none|random, psk_phrase=TEXT, uplink, downlink, pos=N, muted. Example: -c 'name=Test,psk_mode=random' -c 'name=Second,uplink'"
    )]
    channels: Vec<ChannelInfo>,

    #[command(flatten)]
    lora: Option<LoRaArgs>,
}

#[derive(Parser, Debug, Clone)]
struct LoRaArgs {
    #[arg(long, value_enum, help = "Region")]
    region: Option<Region>,

    #[arg(long, value_enum, help = "Modem preset")]
    modem_preset: Option<ModemPresetArg>,

    #[arg(long, help = "TX Power in dBm (0-30)")]
    tx_power: Option<i32>,

    #[arg(long, help = "Hop limit (1-7)")]
    hop_limit: Option<u32>,

    #[arg(long, help = "Channel number (0=auto)")]
    channel_num: Option<u32>,

    #[arg(long, help = "Override duty cycle limit")]
    override_duty_cycle: bool,

    #[arg(long, help = "Enable SX126x RX boosted gain")]
    sx126x_rx_boosted_gain: bool,

    #[arg(long, help = "Override frequency (MHz)")]
    override_frequency: Option<f32>,

    #[arg(long, help = "Frequency offset (kHz)")]
    frequency_offset: Option<f32>,

    #[arg(long, help = "Disable PA fan")]
    pa_fan_disabled: bool,

    #[arg(long = "no-ignore-mqtt", help = "Allow MQTT messages", action = clap::ArgAction::SetFalse, default_value = "true")]
    ignore_mqtt: bool,

    #[arg(long, help = "Allow MQTT")]
    ok_mqtt: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Region {
    Us,
    Eu433,
    Eu868,
    Cn,
    Jp,
    Anz,
    Kr,
    Tw,
    Ru,
    In,
    Nz865,
    Th,
    Lora24,
    Ua433,
    Ua868,
}

impl Region {
    fn to_region_code(&self) -> meshtastic_protobufs::meshtastic::config::lo_ra_config::RegionCode {
        use meshtastic_protobufs::meshtastic::config::lo_ra_config::RegionCode;
        match self {
            Region::Us => RegionCode::Us,
            Region::Eu433 => RegionCode::Eu433,
            Region::Eu868 => RegionCode::Eu868,
            Region::Cn => RegionCode::Cn,
            Region::Jp => RegionCode::Jp,
            Region::Anz => RegionCode::Anz,
            Region::Kr => RegionCode::Kr,
            Region::Tw => RegionCode::Tw,
            Region::Ru => RegionCode::Ru,
            Region::In => RegionCode::In,
            Region::Nz865 => RegionCode::Nz865,
            Region::Th => RegionCode::Th,
            Region::Lora24 => RegionCode::Lora24,
            Region::Ua433 => RegionCode::Ua433,
            Region::Ua868 => RegionCode::Ua868,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModemPresetArg {
    LongFast,
    LongSlow,
    VeryLongSlow,
    MediumSlow,
    MediumFast,
    ShortSlow,
    ShortFast,
    LongModerate,
    ShortTurbo,
}

impl ModemPresetArg {
    fn to_modem_preset(
        &self,
    ) -> meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset {
        use meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset;
        match self {
            ModemPresetArg::LongFast => ModemPreset::LongFast,
            ModemPresetArg::LongSlow => ModemPreset::LongSlow,
            ModemPresetArg::VeryLongSlow => ModemPreset::VeryLongSlow,
            ModemPresetArg::MediumSlow => ModemPreset::MediumSlow,
            ModemPresetArg::MediumFast => ModemPreset::MediumFast,
            ModemPresetArg::ShortSlow => ModemPreset::ShortSlow,
            ModemPresetArg::ShortFast => ModemPreset::ShortFast,
            ModemPresetArg::LongModerate => ModemPreset::LongModerate,
            ModemPresetArg::ShortTurbo => ModemPreset::ShortTurbo,
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        if let Err(e) = tui::run() {
            eprintln!("TUI Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let cli = Cli::parse();

    match cli {
        Cli::Decode { url } => match decode_url(&url) {
            Ok(config) => {
                formatter::print_config(&config);
            }
            Err(e) => {
                formatter::print_error(&e.to_string());
                std::process::exit(1);
            }
        },
        Cli::Encode(args) => match encode_config(&args) {
            Ok((config, short_url, full_url)) => {
                formatter::print_encoded(&config, &short_url, &full_url);
            }
            Err(e) => {
                formatter::print_error(&e.to_string());
                std::process::exit(1);
            }
        },
        Cli::Tui => {
            if let Err(e) = tui::run() {
                eprintln!("TUI Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn encode_config(args: &EncodeArgs) -> Result<(MeshtasticConfig, String, String), EncodeError> {
    let mut config = MeshtasticConfig::new();

    let channels_to_create: Vec<ChannelInfo> = if args.channels.is_empty() {
        vec!["default"
            .parse()
            .map_err(|e: String| EncodeError::ProtobufEncode(e))?]
    } else {
        args.channels.clone()
    };

    for (i, mut ch) in channels_to_create.into_iter().enumerate() {
        ch.index = i;
        ch.role = if i == 0 {
            ChannelRole::Primary
        } else {
            ChannelRole::Secondary
        };
        config.channels.push(ch);
    }

    if let Some(lora_args) = &args.lora {
        let lora = create_lora_config(lora_args);
        config.lora = Some(lora);
    }

    let short_url = encoder::encode_url_short(&config)?;
    let full_url = encoder::encode_url(&config)?;

    Ok((config, short_url, full_url))
}

fn create_lora_config(args: &LoRaArgs) -> LoRaInfo {
    let has_preset = args.region.is_some() || args.modem_preset.is_some();
    let modem_preset = args.modem_preset.map(|m| m.to_modem_preset());

    let (bandwidth, spread_factor, coding_rate) = if has_preset {
        let preset = modem_preset.unwrap_or(
            meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset::LongFast,
        );
        get_preset_params(preset)
    } else {
        (0, 0, 0)
    };

    LoRaInfo {
        region: args
            .region
            .map(|r| r.to_region_code())
            .unwrap_or(unsafe { std::mem::transmute(0i32) }),
        modem_preset: modem_preset.unwrap_or(
            meshtastic_protobufs::meshtastic::config::lo_ra_config::ModemPreset::LongFast,
        ),
        use_preset: has_preset,
        tx_enabled: true,
        tx_power: args.tx_power.unwrap_or(0),
        bandwidth,
        spread_factor,
        coding_rate,
        hop_limit: args.hop_limit.unwrap_or(0),
        channel_num: args.channel_num.unwrap_or(0),
        override_duty_cycle: args.override_duty_cycle,
        sx126x_rx_boosted_gain: args.sx126x_rx_boosted_gain,
        override_frequency: args.override_frequency.unwrap_or(0.0),
        frequency_offset: args.frequency_offset.unwrap_or(0.0),
        pa_fan_disabled: args.pa_fan_disabled,
        ignore_mqtt: args.ignore_mqtt,
        config_ok_to_mqtt: args.ok_mqtt,
        ignore_incoming: Vec::new(),
    }
}
