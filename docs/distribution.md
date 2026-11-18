# Distribution

## What a release contains

A tag such as `v0.1.0` starts the Release workflow, which builds on a macOS runner for each architecture:

- `macmetry_<version>_aarch64.dmg` for Apple silicon and `macmetry_<version>_x64.dmg` for Intel
- `macmetry-<version>-aarch64-apple-darwin.tar.gz` and `macmetry-<version>-x86_64-apple-darwin.tar.gz`, the command line
- `SHA256SUMS` for every file

The release is created as a **draft** and published by hand after the files have been checked.

## Signing

Signing and notarization are **optional and off by default**, because they need an Apple Developer account. When these
repository secrets exist, the workflow signs and notarizes the app:

`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`,
`APPLE_TEAM_ID`

Without them the build still succeeds and says "No Apple signing secrets" in its log. Unsigned builds need the
one-time step in [Getting started](getting-started.md).

## Checking a download

```bash
shasum -a 256 -c SHA256SUMS --ignore-missing
```

## Requirements

macOS 13 (Ventura) or newer, set in `tauri.conf.json` as `minimumSystemVersion`.
