# CI and release

Spec §9.5, §8. Workflows live in `.github/workflows/`.

## CI (`ci.yml`, runs on every push and on PRs to `main`)
- `linux`: builds the same `docker/Dockerfile` image (buildx, GHA cache), then `docker compose run --rm dev just precommit`.
- `macos`: `rustup show` (toolchain from `rust-toolchain.toml`), `brew install just`, clippy, tests. A later task adds the UI build and the `.dmg` artifact `deeboo-dmg`.
- Actions are pinned by commit SHA; concurrency cancels superseded runs per ref.
- `just ship` waits for the run of the exact pushed commit.

## Release (planned, Task 11)
- Tag `vX.Y.Z` (`v*`) triggers `release.yml`: GitHub Release with the `.dmg` attached.
- Dependabot covers cargo, npm and GitHub Actions.
- Task 11 must check this file against the real workflows and update it.

## Unsigned DMG notes (spec §8)
No signing or notarization. Install: drag to Applications; first open via System Settings, Privacy & Security, Open Anyway; if macOS says the app is damaged:
`xattr -dr com.apple.quarantine "/Applications/Deeboo MIDI Sync.app"`. The README carries these steps.
