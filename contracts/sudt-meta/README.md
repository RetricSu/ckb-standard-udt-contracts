# sudt-meta

Type script for sUDT metadata cells.

The contract validates sUDT Meta creation and updates, including type-id
creation, immutable supply-tracking mode, initial tracked supply, and metadata
or mint authority checks.

Contract builds require `SUDT_CODE_HASH`, the 32-byte Data2 code hash
of the matching `sudt` binary. The root `Makefile` builds
`sudt` first and passes this value automatically for
`make build CONTRACT=sudt-meta`.
