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

## Updating

Re-copy the files from a newer tag of the upstream repository, keeping the
import closure complete, and update the version above. `cargo test` covers the
regions and presets, so a mismatch shows up there.
