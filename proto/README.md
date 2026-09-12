# Vendored Meshtastic protobuf definitions

These are the `.proto` files meshurl compiles against, copied verbatim from
the official definitions:

    https://github.com/meshtastic/protobufs

Version: **v2.8.0**

Only the files meshurl needs are kept: `apponly.proto` (the channel set carried
in a URL), `channel.proto`, `config.proto` and `mesh.proto`, plus everything
they import.

They are vendored rather than taken from the `meshtastic_protobufs` crate
because that crate lags the firmware: its latest release, 2.7.8, predates the
regions and modem presets introduced in firmware 2.8, and decoding a URL that
used one of them silently lost the value.

## Not from upstream

`meshurl/shared_contact.proto` is ours. A `/v/` URL carries a `SharedContact`,
which upstream defines in `admin.proto`, and that file cannot be compiled here:
it defines `AS3935_config`, `DS248X_config` and their siblings, which prost
names exactly as it names `AS3935Config` and `DS248XConfig` from
`telemetry.proto`, leaving one package with the same Rust type defined twice.
The message is copied field for field instead, and the file itself says so.

## Updating

Re-copy the files from a newer tag of the upstream repository, keeping the
import closure complete, and update the version above. `cargo test` covers the
regions and presets, so a mismatch shows up there.
