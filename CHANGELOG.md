# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog][keepachangelog], and this project
adheres to [Semantic Versioning][semver].

[keepachangelog]: https://keepachangelog.com/en/1.1.0/
[semver]: https://semver.org/spec/v2.0.0.html

## [Unreleased]

An audit of the whole codebase, covering key generation, several panics, and
the fidelity of what is encoded and displayed.

### Security

- Random PSKs now come from the operating system CSPRNG. They were produced by
  a linear congruential generator seeded with the clock, so a key advertised as
  AES-256 carried only as much entropy as the instant it was generated at.
- The TUI no longer discards the PSK typed into the channel popup. A Base64 key
  was dropped and the channel silently ended up unencrypted, while Passphrase
  hashed the empty string into a key that is a well-known public constant.
- An invalid or missing PSK is refused instead of falling back to the default
  key, which handed out the weak public key while the popup claimed otherwise.
- `psk_mode=passphrase:` on the command line hashed the phrase without its
  first character, and an empty passphrase is now refused.

### Added

- The TUI decodes node (`/v/`) URLs, which it used to refuse.
- The LoRa popup exposes bandwidth, spreading factor and coding rate for manual
  configurations, and offers `Custom` alongside the presets.
- Seven regions that the decoder understood but neither interface offered:
  `MY433`, `MY919`, `SG923`, `PH433`, `PH868`, `PH915` and `ANZ433`.
- `q` and `Ctrl+C` quit the TUI; `Esc` still works.
- The coding rate is shown in the TUI LoRa panel, matching the CLI.
- `--version`, and `\,` to put a literal comma in a channel name or passphrase.
- A CI workflow running rustfmt, clippy and the tests, plus an MSRV job.
- End to end tests for the command line, and the first tests for the TUI.

### Changed

- **`--no-ignore-mqtt` is now `--ignore-mqtt`.** The CLI forced `ignore_mqtt`
  on in every URL it produced; it now follows the firmware default of off.
- The `/e/` and `/v/` prefix decides how a URL is decoded. Decoding used to try
  a channel first and fall back to node info, and since protobuf skips unknown
  fields, a node payload behind an `/e/` prefix was reported as a channel.
- Channel names over the 12 byte firmware limit are refused rather than
  silently truncated by the device.
- A manual modem configuration is shown as `Custom`. It used to read as
  `LongFast`, which is what an unset protobuf enum decodes to.
- The TUI redraws after an event instead of on a fixed 60 Hz tick, and toasts
  expire on a timer rather than on a count of draw calls.
- Library API: `LoRaInfo` replaces its five modem fields with a `ModemConfig`
  enum, `generate_random_psk` returns a `Result`, and `region_code_from_str`
  and `modem_preset_from_str` return an `Option` instead of a silent default.

### Fixed

- A panic inside the TUI left the terminal in raw mode on the alternate screen,
  hiding both the panic message and the shell.
- Five reachable panics: reordering an empty channel list, an unbounded
  selection that the reorder keys then indexed with, a toast wider than the
  terminal, a channel name cut in the middle of a multi-byte character, and
  popups on a terminal narrower than their margin.
- The clipboard passed the URL as a command line argument, which none of the
  helpers accept: xclip took it as a filename and xsel was left reading the
  terminal. `pbcopy` and `clip.exe` are now tried as well.
- Opening a manual LoRa configuration in the popup and saving it replaced the
  parameters with those of LongFast.
- The LoRa bandwidth was divided by 1000 before being compared against a table
  already in kilohertz, and `use_preset` was overridden whenever the parameters
  happened to match a preset.
- `pos=0` and a disabled position no longer emit an empty `ModuleSettings`.
- Decoding a URL with an empty payload reported success and printed nothing.
- Mode keys reached through open popups, stranding them in the state.
- `meshurl decode <url> | head` ended in a "Broken pipe" panic.
- Reordering channels with `+`/`-` did not renumber their index.

### Removed

- The "Use Preset" toggle in the LoRa popup, which wrote the preset values
  whichever way it was set. Selecting `Custom` in the modem field replaces it.
- `bytes` and `crossterm` as direct dependencies; both were unused.

## [0.4.1] - 2026-03-22

### Documentation

- Expanded the README with full TUI and CLI documentation.

## [0.4.0] - 2026-03-21

### Added

- Node info (`/v/`) URL decoding, including user role and public key.

## [0.3.0] - 2026-03-20

### Added

- Base64 and passphrase PSK modes, in both the CLI and the TUI.

### Fixed

- LoRa popup behaviour, and a consistent region and preset display.

## [0.2.0] - 2026-03-18

### Added

- Clipboard copying with a fallback, and toast notifications.
- Channel reordering with `+` and `-` in encode mode.
- `[M]` to carry a decoded URL into encode mode.

## [0.1.0] - 2026-03-17

First release: decoding and encoding of Meshtastic channel URLs, with a CLI and
a TUI.
