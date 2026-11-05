# CI and release

Spec §9.5, §8. Workflows live in `.github/workflows/`.

## CI (`ci.yml`, runs on every push and on PRs to `main`)
- `linux`: builds the same `docker/Dockerfile` image (buildx, GHA cache), then `docker compose run --rm dev just precommit`.
- `macos`: `rustup show` (toolchain from `rust-toolchain.toml`), Node 26, `brew install just`, `npm ci --prefix ui`, UI build (needed by `generate_context!`), clippy, tests, `npm --prefix ui run tauri build -- --bundles dmg`, then uploads `target/release/bundle/dmg/*.dmg` as artifact `deeboo-dmg`.
- Actions are pinned by commit SHA; concurrency is per job: `linux` cancels superseded runs per ref, `macos` never cancels a running build (one pending run per ref, the newest), so every macOS build finishes.
- `just ship` waits for the run of the exact pushed commit.
- `changes` (`dorny/paths-filter`) gates `macos`: it runs only when `src-tauri/**`, `tools/**`, `ui/**`, `Cargo.lock`, `rust-toolchain.toml`, `.github/workflows/**` or `.github/actions/**` change. PRs to `main` always run it. Pushes compare against the previous commit on the same branch (`base: github.ref`), not `main`. Update the filter list here when a new build input appears.

## Release (planned, Task 11)
- Tag `vX.Y.Z` (`v*`) triggers `release.yml`: GitHub Release with the `.dmg` attached.
- Dependabot covers cargo, npm and GitHub Actions.
- Task 11 must check this file against the real workflows and update it.

## Unsigned DMG notes (spec §8)
No signing or notarization. Install: drag to Applications; first open via System Settings, Privacy & Security, Open Anyway; if macOS says the app is damaged:
`xattr -dr com.apple.quarantine "/Applications/Deeboo MIDI Sync.app"`. The README carries these steps.
