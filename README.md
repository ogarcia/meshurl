# MeshURL

A [Meshtastic][meshtastic] channel configuration URL encoder and decoder written in [Rust][rust].

[rust]: https://www.rust-lang.org/
[meshtastic]: https://meshtastic.org/

## Features

- Decode Meshtastic channel URLs
- Encode channel configurations to Meshtastic URLs
- TUI (Terminal User Interface) for easy configuration
- Command Line Interface for scripting

## Installation

### From binary

Download the latest release from the [releases page][releases].

[releases]: https://github.com/ogarcia/meshchan/releases

### From source

#### Installing Rust

MeshURL has been tested with current Rust stable release version. You can install Rust from your distribution package or use [`rustup`][rustup].

```shell
rustup default stable
```

[rustup]: https://rustup.rs/

#### Building MeshURL

```shell
git clone https://github.com/ogarcia/meshchan.git
cd meshchan/meshurl
cargo build --release
```

After build, the binary is located in `target/release/meshurl`.

## Usage

### Command Line Interface

#### Decode a URL

```shell
meshurl decode "https://meshtastic.org/e/#CgMSAQ..."
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

#### Navigation

| Key | Action |
| --- | --- |
| `1` | Switch to Decode mode |
| `2` | Switch to Encode mode |
| `Tab` / `Shift+Tab` | Switch between panels |
| `↑` / `↓` | Scroll in panels |

#### Decode Mode

| Key | Action |
| --- | --- |
| `Enter` | Edit URL |
| `Del` | Clear URL |
| `Esc` | Exit edit / Quit |

#### Encode Mode

| Key | Action |
| --- | --- |
| `A` | Add new channel |
| `D` | Delete selected channel |
| `E` | Edit LoRa configuration |
| `G` | Generate URL |
| `C` | Copy URL to clipboard |
| `Del` | Clear all configuration |

## Library

MeshURL provides a Rust library that can be used in other projects.

```rust
use meshurl::{decode_url, encode_url, MeshtasticConfig};

// Decode a URL
let config = decode_url("https://meshtastic.org/e/#CgMSAQ...").unwrap();

// Encode a configuration
let url = encode_url(&config).unwrap();
```
