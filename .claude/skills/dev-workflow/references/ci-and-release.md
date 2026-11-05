# CI and release

Spec §9.5, §8. Workflows live in `.github/workflows/`.

## CI (`ci.yml`, runs on every push and on PRs to `main`)
- `linux`: builds the same `docker/Dockerfile` image (buildx, GHA cache), then `docker compose run --rm dev just precommit`.
- `macos`: checkout, the composite action `.github/actions/macos-build` (`rustup show` for the toolchain from `rust-toolchain.toml`, Node 26, `npm ci --prefix ui`, UI build needed by `generate_context!`, `npm --prefix ui run tauri build -- --bundles dmg`), then `brew install just`, clippy, tests, and uploads `target/release/bundle/dmg/*.dmg` as artifact `deeboo-dmg`.
- Actions are pinned by commit SHA; concurrency is per job: `linux` cancels superseded runs per ref, `macos` never cancels a running build (one pending run per ref, the newest), so every macOS build finishes.
- `just ship` waits for the run of the exact pushed commit.
- `changes` (`dorny/paths-filter`) gates `macos`: it runs only when `src-tauri/**`, `tools/**`, `ui/**`, `Cargo.lock`, `rust-toolchain.toml`, `.github/workflows/**` or `.github/actions/**` change. PRs to `main` always run it. Pushes compare against the previous commit on the same branch (`base: github.ref`), not `main`. Update the filter list here when a new build input appears.

## Release (`release.yml`)
- Tag `vX.Y.Z` (`v*`) triggers it on `macos-latest` with `contents: write`: checkout, the same `macos-build` composite action, then `softprops/action-gh-release` (pinned by SHA) attaches `target/release/bundle/dmg/*.dmg` to the GitHub Release. Never push a tag without asking the user.
- `.github/dependabot.yml`: weekly `cargo` (/), `npm` (/ui), `github-actions` (/), `docker` (/docker); minor and patch updates are grouped per ecosystem.
- Local `just dmg` can fail at create-dmg's Finder AppleScript step in non-GUI sessions; `CI=true just dmg` skips the styling.

## Ad-hoc signed DMG notes (spec §8)
Ad-hoc signed since 0.1.1 (`bundle.macOS.signingIdentity` = `"-"` in `tauri.conf.json`, no Apple account), still not notarized. Verify with `codesign -dv --verbose=2` (`Signature=adhoc`) and `codesign --verify --deep --strict`. Install: drag to Applications; first open via System Settings, Privacy & Security, Open Anyway (normal path); fallback if macOS still says the app is damaged:
`xattr -dr com.apple.quarantine "/Applications/Deeboo MIDI Sync.app"`. The README carries these steps.
