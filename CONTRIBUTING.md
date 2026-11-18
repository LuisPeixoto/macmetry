# Contributing to macmetry

Thank you for helping make macmetry better! Bug reports, documentation, design polish, tests and new collectors are
all welcome. By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

For anything bigger than a small fix, please open an issue first so we can agree on the approach.

## Development setup

You need a Mac (Intel or Apple silicon, macOS 13 or newer), [Rust](https://rustup.rs) (stable) and
[Node.js](https://nodejs.org) 20 or newer.

```bash
git clone https://github.com/LuisPeixoto/macmetry.git
cd macmetry

# Libraries and command line
cargo test --workspace --exclude macmetry-desktop
cargo run -p macmetry-cli -- snapshot

# Desktop app (hot reload for the interface)
cd apps/desktop
npm install
npm run tauri dev
```

Set `MACMETRY_HOME` to a throwaway folder while developing so your own history is never touched.

## Before you open a pull request

Run every gate and read the real exit status (do not pipe them through `tail`):

```bash
cargo fmt --all --check \
  && cargo clippy --workspace --exclude macmetry-desktop --all-targets -- -D warnings \
  && cargo test --workspace --exclude macmetry-desktop
cd apps/desktop && npm run check && npm test && npm run build
```

## Commits and branches

- [Conventional Commits](https://www.conventionalcommits.org): `type(scope): imperative summary`, with types `feat`,
  `fix`, `docs`, `test`, `ci`, `chore`, `build`, `style` and `refactor`.
- Keep commits small: one per file, or per group of files with the same purpose.
- Use one branch per piece of work: `feat/…`, `fix/…`, `docs/…`.

## Tests and fixtures

Parsers take text or bytes and return structs, so they are tested with **real output captured once from a Mac** and
committed under `tests/fixtures/`. Prefer a real fixture to an invented one. Anything that needs real hardware is an
`#[ignore]` test collected in `tools/live-tests.sh`.
