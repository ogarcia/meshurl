use colored::{Color, Colorize};
use meshurl::models::{ChannelRole, LoRaInfo, MeshtasticConfig};

pub fn print_config(config: &MeshtasticConfig) {
    print_channels(&config.channels);

    if let Some(lora) = &config.lora {
        print_lora(lora);
    }
}

fn print_channels(channels: &[meshurl::models::ChannelInfo]) {
    let title = "Meshtastic Channel Configuration".color(Color::Cyan).bold();
    println!("{}\n", title);

    for channel in channels {
        let role_color = if channel.role == ChannelRole::Primary {
            Color::Green
        } else {
            Color::Yellow
        };

        println!(
            "  {}  {}  ",
            format!("Channel {}", channel.index)
                .color(Color::Blue)
                .bold(),
            format!("({})", channel.role.as_str())
                .color(role_color)
                .bold()
        );

        println!(
            "    {} {}",
            "Name:".color(Color::White),
            channel.name.color(Color::BrightWhite)
        );

        if !channel.psk.is_empty() {
            let psk_value_color = get_psk_color(&channel.psk_type);
            let psk_type_color = get_psk_color(&channel.psk_type);

            println!(
                "    {} {}",
                "PSK:".color(Color::White),
                channel.psk.color(psk_value_color)
            );
            println!(
                "    {} {}",
                "PSK Type:".color(Color::White),
                channel.psk_type.as_str().color(psk_type_color)
            );
        } else {
            println!(
                "    {} {}",
                "PSK:".color(Color::White),
                "None (no crypto)".color(Color::Red)
            );
        }

        print_enabled_disabled("Uplink:", channel.uplink_enabled);
        print_enabled_disabled("Downlink:", channel.downlink_enabled);

        if let Some(precision) = channel.position_precision {
            println!(
                "    {} {}",
                "Position Precision:".color(Color::White),
                precision
            );
        }

        print_yes_no("Client Muted:", channel.is_client_muted);

        println!();
    }
}

fn get_psk_color(psk_type: &meshurl::models::PskType) -> Color {
    if psk_type.is_default() {
        Color::BrightCyan
    } else if psk_type.is_custom() {
        Color::Green
    } else if psk_type.is_simple() {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn print_yes_no(name: &str, value: bool) {
    let text = if value { "Yes" } else { "No" };
    let color = if value { Color::Green } else { Color::Red };
    println!("    {} {}", name.color(Color::White), text.color(color));
}

fn print_enabled_disabled(name: &str, value: bool) {
    let text = if value { "enabled" } else { "disabled" };
    let color = if value { Color::Green } else { Color::Red };
    println!("    {} {}", name.color(Color::White), text.color(color));
}

pub fn print_lora(lora: &LoRaInfo) {
    let lora_title = "LoRa Configuration".color(Color::Cyan).bold();
    println!("{}\n", lora_title);

    println!(
        "    {} {}",
        "Region:".color(Color::White),
        format!("{:?}", lora.region).color(Color::BrightCyan)
    );
    println!(
        "    {} {}",
        "Modem Preset:".color(Color::White),
        format!("{:?}", lora.modem_preset).color(Color::BrightCyan)
    );

    print_yes_no("Use Preset:", lora.use_preset);

    if lora.tx_enabled {
        let tx_power_str = if lora.tx_power == 0 {
            "0 (maximum safe power)".to_string()
        } else {
            format!("{} dBm", lora.tx_power)
        };
        println!(
            "    {} {}",
            "TX Power:".color(Color::White),
            tx_power_str.color(Color::BrightGreen)
        );
    } else {
        print_yes_no("TX enabled:", lora.tx_enabled);
    }

    let preset_str = format!("{:?}", lora.modem_preset);
    if !preset_str.is_empty() && preset_str != "Unset" {
        println!(
            "    {} {}",
            "Bandwidth:".color(Color::White),
            format!("{} kHz", lora.bandwidth)
        );
        println!(
            "    {} {}",
            "Spread Factor:".color(Color::White),
            lora.spread_factor
        );
        println!(
            "    {} {}",
            "Coding Rate:".color(Color::White),
            format!("4/{}", lora.coding_rate)
        );
    }

    let hop_color = if lora.hop_limit <= 3 {
        Color::Green
    } else if lora.hop_limit <= 5 {
        Color::Yellow
    } else {
        Color::Red
    };
    println!(
        "    {} {}",
        "Hop Limit:".color(Color::White),
        lora.hop_limit.to_string().color(hop_color)
    );
    println!(
        "    {} {}",
        "Channel Num:".color(Color::White),
        lora.channel_num
    );

    print_yes_no("Override Duty Cycle:", lora.override_duty_cycle);
    print_yes_no("SX126x RX Boosted Gain:", lora.sx126x_rx_boosted_gain);

    if lora.override_frequency > 0.0 {
        println!(
            "    {} {}",
            "Override Frequency:".color(Color::White),
            format!("{} MHz", lora.override_frequency).color(Color::Yellow)
        );
    }

    if lora.frequency_offset != 0.0 {
        println!(
            "    {} {}",
            "Frequency Offset:".color(Color::White),
            format!("{} kHz", lora.frequency_offset).color(Color::Yellow)
        );
    }

    print_yes_no("PA Fan Enabled:", !lora.pa_fan_disabled);
    print_yes_no("Ignore MQTT:", lora.ignore_mqtt);
    print_yes_no("OK to MQTT:", lora.config_ok_to_mqtt);

    if !lora.ignore_incoming.is_empty() {
        println!(
            "    {} {:?}",
            "Ignore Incoming:".color(Color::White),
            lora.ignore_incoming
        );
    }
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "Error:".color(Color::Red).bold(), msg);
}

pub fn print_encoded(config: &meshurl::models::MeshtasticConfig, short_url: &str, full_url: &str) {
    let title = "Generated Configuration".color(Color::Cyan).bold();
    println!("{}\n", title);

    print_urls(short_url, full_url);
    println!();
    print_config(config);
}

fn print_urls(short_url: &str, full_url: &str) {
    let label = "URL:".color(Color::White).bold();
    println!("  {} {}", label, full_url.color(Color::BrightBlue));

    let label_short = "Short:".color(Color::White).bold();
    println!("  {} {}", label_short, short_url.color(Color::Blue));
}
