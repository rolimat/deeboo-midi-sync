# Docker and macOS

## Docker
- `docker/Dockerfile`: `rust:1.99.1-bookworm`, Node 26 (NodeSource), Tauri/ALSA system libs, pinned `just`, `cargo-deny`, `lefthook`, `hadolint` (arch-aware: arm64 locally, amd64 on CI). Linted by hadolint (`.hadolint.yaml`).
- `compose.yaml`: service `dev`, image `deeboo-dev`, repo mounted at `/work`, named volumes for the cargo registry, `target/` and `ui/node_modules`; `command: sleep infinity`. `.devcontainer/devcontainer.json` opens the repo in the same image.
- Toolchain pinned in `rust-toolchain.toml` (with rustfmt, clippy); change it and the Dockerfile together.
- Rebuild after Dockerfile changes: `docker compose build dev`.

## macOS host
- `ui/node_modules` on the host is separate from the Docker volume. The `tauri` npm script runs the CLI from the repo root so it finds `src-tauri/tauri.conf.json`.
- Needs the Rust toolchain, Node and `just` installed on the host, plus `lefthook install` for the hook (the hook itself runs `just precommit` in Docker).
- `just dev` runs the app with real MIDI; `just dmg` builds the `.dmg`; `just jitter` runs the jitter probe. Run from the host, never inside the container (Docker on a Mac is Linux: no CoreMIDI, no bundles).
- Manual acceptance tests: spec §10.
