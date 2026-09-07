# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog][keepachangelog], and this project
adheres to [Semantic Versioning][semver].

[keepachangelog]: https://keepachangelog.com/en/1.1.0/
[semver]: https://semver.org/spec/v2.0.0.html

## [0.6.0] - unreleased

### Fixed

- Esc in the channel name box closed the whole popup instead of just the box,
  discarding everything else set on the channel. Both the popup and the
  application handled the key, and the one that ran did not know about the box.

## [0.5.0] - 2026-09-05

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

- Support for the regions and modem presets introduced by firmware 2.8: 15 new
  regions (Kazakhstan, Nepal, Brazil, the ITU amateur bands, EU 866/874/917 and
  EU narrow 868) and 8 new presets (LongTurbo, MediumTurbo, Lite, Narrow and
  Tiny). A URL using any of them used to decode as `UNSET` or `LONG_FAST` and
  lose the value on the way back out.
- The 2.4 GHz band (`LORA_24`) now gets the wider bandwidths the firmware uses
  there, instead of the narrow band figures.
- Enter on a LoRa field now opens the right thing for it, rather than making
  every value be reached one arrow press at a time. Region, Modem and Hop Limit
  open a list of every value; TX Power, Channel and the two frequencies open a
  box to type one into, checked against the range the field accepts. The modem
  list shows each preset's bandwidth, spreading factor and coding rate.
- Choosing a modem preset moves the region to the one it requires, as the
  firmware does: picking NarrowFast on EU_868 switches to EU_N_868 there and
  then, and a toast says so. Choosing a region moves the preset the same way,
  so the two fields never contradict each other.
- The TUI decodes node (`/v/`) URLs, which it used to refuse.
- The LoRa popup exposes bandwidth, spreading factor and coding rate for manual
  configurations, and offers `Custom` alongside the presets.
- Seven regions the decoder understood but neither interface offered:
  `MY_433`, `MY_919`, `SG_923`, `PH_433`, `PH_868`, `PH_915` and `ANZ_433`.
- `q` and `Ctrl+C` quit the TUI; `Esc` still works.
- The coding rate is shown in the TUI LoRa panel, matching the CLI.
- `--version`, and `\,` to put a literal comma in a channel name or passphrase.
- A CI workflow running rustfmt, clippy and the tests.
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
- **Regions and presets are now named as the protobuf names them**: `EU_868`
  rather than `EU868`, `LONG_FAST` rather than `LongFast`. The names come from
  the definitions, so they can no longer fall behind the firmware.
- The protobuf types are generated at build time from the definitions vendored
  in `proto/`, taken from upstream v2.8.0, instead of the `meshtastic_protobufs`
  crate, whose latest release predates firmware 2.8. Building needs no `protoc`:
  the definitions are compiled with protox.
- `VERY_LONG_SLOW` is no longer offered. The firmware deprecated it in 2.5 and
  treats it as an illegal value.

### Fixed

- Four modem presets carried the wrong radio parameters. Against the firmware
  table, `LONG_SLOW` is 125 kHz / SF12 / 4:8 (was 250 / 12 / 4:5), `MEDIUM_SLOW`
  is 250 kHz (was 125), `SHORT_SLOW` is 250 kHz (was 125) and `LONG_MODERATE` is
  125 kHz / SF11 / 4:8 (was 250 / 10 / 4:5).
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
- Esc in the LoRa list overlay closed the whole popup instead of just the list,
  because the key was acted on twice.
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
