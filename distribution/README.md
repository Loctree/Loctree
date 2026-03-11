# Distribution Spine

This directory is the single source of truth for every release channel that is
not "cargo publish the crate and hope for the best."

## Channels

- `crates/`
  Rust crates.io release contract and publish notes.
- `homebrew/`
  Formula source, tap sync notes, and helper scripts.
- `npm/`
  Canonical npm wrapper and platform-package release flow.
- `macos/`
  Signing, notarization, and direct-download bundle contract.
- `linux/`
  Linux release asset contract.
- `windows/`
  Windows release asset contract.

## Principle

One channel, one home.

Do not scatter release state across root-level `Formula/`, ad-hoc docs, and
half-remembered shell rituals. If a distribution path is real, it belongs in
`distribution/`.
