# MeshURL

A [Meshtastic][meshtastic] channel configuration URL encoder and decoder written in [Rust][rust].

[rust]: https://www.rust-lang.org/
[meshtastic]: https://meshtastic.org/

## Features

- Decode and encode Meshtastic channel URLs (`/e/`), replacing a device's
  channels or adding to them (`?add=true`)
- Decode node info URLs (`/v/`), the contacts devices share with each other
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
| `Shift+Del` | Clear the URL and its results |
| `↑` / `↓` | Scroll channels or LoRa config |

#### Encode Mode

| Key | Action |
| --- | --- |
| `A` | Add new channel |
| `Enter` | Edit selected channel |
| `D` / `Del` | Delete the selected channel, or the LoRa configuration |
| `U` | Undo the last delete or clear |
| `+` / `-` | Move channel up/down |
| `E` | Edit LoRa configuration |
| `G` | Generate URL from current config |
| `R` | Whether the URL replaces the device's channels or adds to them |
| `C` | Copy generated URL to clipboard |
| `Shift+Del` | Clear all configuration |
| `↑` / `↓` | Scroll channels or LoRa config |

`Del` takes what the focused panel holds — the selected channel, or the LoRa
configuration — and `Shift+Del` takes everything, so a mistyped key costs one
thing rather than the lot. Either way `U` puts back what went, one step deep,
and the notification says so when it happens.

Deleting the LoRa configuration is the only way back to a URL that carries
none: `E` opens the parameters for editing, but every set of them is a valid
configuration, so it can never leave the URL without one.

The footer at the bottom lists the keys the focused panel accepts, on as many
lines as they need for the width of the terminal.

#### Replacing or adding channels

Importing a channel URL normally replaces the whole channel table of the
device it is opened on. A URL carrying `?add=true` puts each channel into the
first free slot instead, as a secondary channel, leaving the ones already
there alone; channels whose name is already present are skipped.

`R` in the TUI and `--add` on the command line choose between the two. The
channel numbers are not yours to pick either way: the URL format carries an
ordered list of channels with no index in it, so whoever imports it decides
where they land. Ordering them with `+` and `-` is as far as it goes.

```shell
meshurl encode -c 'name=Private,psk_mode=random' --add
```

The short form of the URL has nowhere to put the query string, so only the
full URL adds rather than replaces.

#### Channel Popup

| Key | Action |
| --- | --- |
| `↑` / `↓` | Navigate fields |
| `←` / `→` / `Space` | Cycle field value |
| `Enter` | Open the field, or save |
| `Esc` | Close the open field, or cancel and close |

`Enter` opens what the field needs: a list to pick from for PSK Mode and
Position, and a box to type into for Name and PSK. `Esc` closes it again
without taking the value, leaving the rest of the channel alone.

Fields:
- **Name**: Channel name (up to 12 bytes, as the firmware stores)
- **PSK Mode**: Default, None, Random, Base64, Passphrase
- **PSK**: Pre-shared key (when Base64 or Passphrase mode)
- **Uplink**: Enable uplink messages
- **Downlink**: Enable downlink messages
- **Position**: Position precision, from disabled to exact GPS
- **Muted**: Do not notify on this channel

#### LoRa Configuration Popup

| Key | Action |
| --- | --- |
| `↑` / `↓` | Navigate fields |
| `←` / `→` / `Space` | Cycle field value |
| `Enter` | Open the field, or save |
| `Esc` | Close the open field, or cancel and close |

`Enter` opens a list of every value for Region, Modem and Hop Limit, and a box
to type into for TX Power, Channel and the two frequencies.

Fields:
- **Region**: every region the firmware defines, `UNSET` included
- **Modem**: every modem preset, plus `Custom` for parameters set by hand
- **Bandwidth**, **Spread Factor**, **Coding Rate**: only with `Custom`
- **TX Power**: 0-30 dBm (0 = default)
- **Hop Limit**: 1-7 (default 3)
- **Channel**: 0-255 (0 = auto)
- **TX Enabled**: Enable transmission
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
