# MeshURL

A [Meshtastic][meshtastic] channel configuration URL encoder and decoder written in [Rust][rust].

[rust]: https://www.rust-lang.org/
[meshtastic]: https://meshtastic.org/

## Features

- Decode and encode Meshtastic channel URLs (`/e/`)
- Decode node info URLs (`/v/`) to display device information
- Support for multiple channels (up to 8)
- PSK modes: Default, None, Random, Base64, Passphrase
- LoRa configuration with all regions and modem presets
- Channel settings: Uplink/Downlink, Position precision, Mute
- TUI (Terminal User Interface) for easy configuration
- Command Line Interface for scripting

## Installation

### From binary

Download the latest release from the [releases page][releases].

[releases]: https://github.com/ogarcia/meshurl/releases

### From source

#### Installing Rust

MeshURL has been tested with current Rust stable release version. You can install Rust from your distribution package or use [`rustup`][rustup].

```shell
rustup default stable
```

[rustup]: https://rustup.rs/

#### Building MeshURL

```shell
git clone https://github.com/ogarcia/meshurl.git
cd meshurl
cargo build --release
```

After build, the binary is located in `target/release/meshurl`.

## Usage

### Command Line Interface

#### Decode a channel URL

```shell
meshurl decode "https://meshtastic.org/e/#CgMSAQ..."
```

#### Decode a node URL

```shell
meshurl decode "https://meshtastic.org/v/#CIiys4YK..."
```

#### Encode a configuration

```shell
meshurl encode --region eu868 -c 'name=TestChannel,psk_mode=random'
```

### TUI Interface

Run the TUI without arguments:

```shell
meshurl
```

### TUI Modes

The TUI has two modes:

- **Decode Mode**: Paste and decode Meshtastic channel or node URLs
- **Encode Mode**: Create channel configurations and generate URLs

#### Global Navigation

| Key | Action |
| --- | --- |
| `1` | Switch to Decode mode |
| `2` | Switch to Encode mode |
| `Tab` / `Shift+Tab` | Switch between panels |
| `Esc` | Quit |

#### Decode Mode

| Key | Action |
| --- | --- |
| `Enter` | Edit URL / Decode URL |
| `M` | Migrate decoded URL to Encode mode |
| `Del` | Clear URL and results |
| `↑` / `↓` | Scroll channels or LoRa config |

#### Encode Mode

| Key | Action |
| --- | --- |
| `A` | Add new channel |
| `Enter` | Edit selected channel |
| `D` | Delete selected channel |
| `+` / `-` | Move channel up/down |
| `E` | Edit LoRa configuration |
| `G` | Generate URL from current config |
| `C` | Copy generated URL to clipboard |
| `Del` | Clear all configuration |
| `↑` / `↓` | Scroll channels or LoRa config |

#### Channel Popup

| Key | Action |
| --- | --- |
| `↑` / `↓` | Navigate fields |
| `←` / `→` / `Space` | Cycle field value |
| `Enter` | Save / Edit name/PSK |
| `Esc` | Cancel and close |

Fields:
- **Name**: Channel name (up to 30 chars)
- **PSK Mode**: Default, None, Random, Base64, Passphrase
- **PSK**: Pre-shared key (when Base64 or Passphrase mode)
- **Uplink**: Enable uplink messages
- **Downlink**: Enable downlink messages
- **Position**: GPS position precision (0-3)
- **Muted**: Disable audio notifications

#### LoRa Configuration Popup

| Key | Action |
| --- | --- |
| `↑` / `↓` | Navigate fields |
| `←` / `→` / `Space` | Cycle field value |
| `Enter` | Save |
| `Esc` | Cancel and close |

Fields:
- **Region**: US, EU433, EU868, CN, JP, ANZ, KR, TW, RU, IN, NZ865, TH, Lora24, UA433, UA868
- **Modem Preset**: LongFast, LongSlow, VeryLongSlow, MediumSlow, MediumFast, ShortSlow, ShortFast, LongModerate, ShortTurbo
- **TX Power**: 0-30 dBm (0 = default)
- **Hop Limit**: 1-7 (default 3)
- **Channel**: 0-255 (0 = auto)
- **TX Enabled**: Enable transmission
- **Use Preset**: Use modem preset settings
- **Override Freq**: Custom frequency (MHz)
- **Freq Offset**: Frequency offset (-100 to 100 kHz)
- **SX126x RX**: Boost receive sensitivity
- **Duty Cycle**: Override duty cycle limit
- **PA Fan Disabled**: Disable PA fan
- **Ignore MQTT**: Ignore incoming MQTT
- **OK to MQTT**: Allow MQTT configuration

## Library

MeshURL provides a Rust library that can be used in other projects.

```rust
use meshurl::{decode_url, encode_url, DecodeResult, MeshtasticConfig};

// Decode a URL (channel or node)
let result = decode_url("https://meshtastic.org/e/#CgMSAQ...").unwrap();

match result {
    DecodeResult::Channel(config) => {
        // Handle channel configuration
        for channel in config.channels {
            println!("Channel: {}", channel.name);
        }
    }
    DecodeResult::Node(node) => {
        // Handle node information
        println!("Node: {} ({})", node.long_name, node.short_name);
        println!("Role: {}", node.role);
    }
}

// Encode a configuration
let url = encode_url(&config).unwrap();
```
