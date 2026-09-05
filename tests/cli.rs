//! End to end tests driving the compiled binary.
//!
//! These cover the command line surface and the terminal output formatting,
//! neither of which is reachable from the library tests.

use std::process::{Command, Output};

/// A channel URL with three channels and a LoRa config using a preset.
const CHANNEL_URL: &str = "https://meshtastic.org/e/#CgsSAQEoATABOgIIDQoPEgEBGgZJYmVyaWEoATABChESAQEaCEFDb3J1w7FhKAEwARIWCAEY-gEgCygFOANABkgBUBtoAcAGAQ";

/// A node URL, as shared by the Meshtastic app.
const NODE_URL: &str =
    "https://meshtastic.org/v/#CAESJQoLIXRlc3QwMDAwMDESEEdhbGljaWEgQ2FsaWRhZGUaBPCfkJk";

/// A channel URL whose LoRa parameters are set by hand rather than by preset.
const CUSTOM_LORA_URL: &str = "https://meshtastic.org/e/#ChcSAQEaCk5hcnJvd0Zhc3QoATABOgIIDgoPEgECGgRUZXN0KAEwAToAChISAQEaB0dhbGljaWEoATABOgAKExIBARoIQUNvcnXDsWEoATABOgAKDRIBARoETHVnbygBMAEKEhIBARoHT3VyZW5zZSgBMAE6AAoTEgEBGgpQb250ZXZlZHJhKAEwARIdGD4gBygGOANAA0gBUBtYAWgBdURcWUTABgHIBgE";

fn meshurl(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_meshurl"))
        .args(args)
        // Keep the output free of ANSI codes so it can be matched on.
        .env("NO_COLOR", "1")
        .output()
        .expect("the binary runs")
}

fn stdout_of(args: &[&str]) -> String {
    let output = meshurl(args);
    assert!(
        output.status.success(),
        "meshurl {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the output is UTF-8")
}

#[test]
fn decodes_a_channel_url() {
    let out = stdout_of(&["decode", CHANNEL_URL]);

    assert!(out.contains("Meshtastic Channel Configuration"));
    assert!(out.contains("Iberia"));
    assert!(out.contains("(PRIMARY)"));
    assert!(out.contains("(SECONDARY)"));
}

#[test]
fn decodes_a_node_url() {
    let out = stdout_of(&["decode", NODE_URL]);

    assert!(out.contains("Node Information"));
    assert!(out.contains("Galicia Calidade"));
    assert!(out.contains("Role:"));
}

#[test]
fn prints_the_lora_configuration() {
    let out = stdout_of(&["decode", CHANNEL_URL]);

    assert!(out.contains("LoRa Configuration"));
    assert!(out.contains("Region:"));
    assert!(out.contains("Modem Preset: LongFast"));
    assert!(out.contains("Bandwidth: 250 kHz"));
    assert!(out.contains("Coding Rate: 4/5"));
}

#[test]
fn names_a_manual_modem_configuration_custom() {
    let out = stdout_of(&["decode", CUSTOM_LORA_URL]);

    // Not "LongFast", which is what an unset protobuf enum reads as.
    assert!(out.contains("Modem Preset: Custom"));
    assert!(out.contains("Use Preset: No"));
    assert!(out.contains("Bandwidth: 62 kHz"));
}

#[test]
fn an_invalid_url_fails_with_a_message() {
    let output = meshurl(&["decode", "not-a-url!!!"]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error:"), "stderr was: {}", stderr);
    assert!(output.stdout.is_empty(), "errors do not go to stdout");
}

#[test]
fn an_empty_payload_fails() {
    let output = meshurl(&["decode", "https://meshtastic.org/e/#"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no configuration"));
}

#[test]
fn encodes_a_default_channel() {
    let out = stdout_of(&["encode", "-c", "default"]);

    assert!(out.contains("https://meshtastic.org/e/#"));
    assert!(out.contains("Short: #"));
    assert!(out.contains("PSK Type: Default"));
}

#[test]
fn encodes_several_channels() {
    let out = stdout_of(&["encode", "-c", "default", "-c", "name=Iberia,uplink"]);

    assert!(out.contains("Channel 0"));
    assert!(out.contains("Channel 1"));
    assert!(out.contains("Iberia"));
    assert!(out.contains("Uplink: enabled"));
}

#[test]
fn a_generated_url_decodes_back() {
    let encoded = stdout_of(&["encode", "-c", "name=Galicia,pos=14", "--region", "eu868"]);
    let url = encoded
        .lines()
        .find_map(|line| line.trim().strip_prefix("URL: "))
        .expect("the output carries a URL")
        .trim()
        .to_string();

    let decoded = stdout_of(&["decode", &url]);

    assert!(decoded.contains("Galicia"));
    assert!(decoded.contains("Region: EU868"));
    assert!(decoded.contains("Position Precision: 14"));
}

#[test]
fn an_overlong_channel_name_is_refused() {
    let output = meshurl(&["encode", "-c", "name=EsteNombreEsDemasiadoLargo"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("maximum is 12"));
}

#[test]
fn an_invalid_psk_is_refused() {
    let output = meshurl(&["encode", "-c", "psk_base64=MTIzNDU2"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid PSK length"));
}

#[test]
fn an_empty_passphrase_is_refused() {
    let output = meshurl(&["encode", "-c", "psk_passphrase="]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Passphrase cannot be empty"));
}

#[test]
fn a_comma_can_be_escaped_in_a_value() {
    let out = stdout_of(&["encode", "-c", r"name=A\,B"]);

    assert!(out.contains("Name: A,B"));
}

#[test]
fn every_region_is_selectable() {
    // Seven of these were missing from the command line.
    for region in [
        "us", "eu433", "eu868", "cn", "jp", "anz", "kr", "tw", "ru", "in", "nz865", "th", "lora24",
        "ua433", "ua868", "my433", "my919", "sg923", "ph433", "ph868", "ph915", "anz433",
    ] {
        let output = meshurl(&["encode", "-c", "default", "--region", region]);
        assert!(
            output.status.success(),
            "--region {} was rejected: {}",
            region,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn mqtt_traffic_is_not_ignored_by_default() {
    let out = stdout_of(&["encode", "-c", "default", "--region", "eu868"]);

    assert!(out.contains("Ignore MQTT: No"));
}

#[test]
fn ignore_mqtt_can_be_turned_on() {
    let out = stdout_of(&[
        "encode",
        "-c",
        "default",
        "--region",
        "eu868",
        "--ignore-mqtt",
    ]);

    assert!(out.contains("Ignore MQTT: Yes"));
}

#[test]
fn reports_its_version() {
    let out = stdout_of(&["--version"]);

    assert!(out.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_the_subcommands() {
    let out = stdout_of(&["--help"]);

    assert!(out.contains("decode"));
    assert!(out.contains("encode"));
    assert!(out.contains("tui"));
}
