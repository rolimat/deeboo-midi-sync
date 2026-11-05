FROM rust:1.99.1-bookworm

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG JUST_VERSION=1.59.0
ARG CARGO_DENY_VERSION=0.20.2
ARG LEFTHOOK_VERSION=2.2.2
ARG HADOLINT_VERSION=2.15.1

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libxdo-dev \
    libayatana-appindicator3-dev librsvg2-dev libasound2-dev curl ca-certificates git \
 && curl -fsSL https://deb.nodesource.com/setup_26.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

# Pinned tool binaries, picked per CPU architecture (arm64 locally, amd64 on CI).
RUN case "$(dpkg --print-architecture)" in \
      amd64) musl=x86_64-unknown-linux-musl; lh=x86_64; hl=x86_64 ;; \
      arm64) musl=aarch64-unknown-linux-musl; lh=aarch64; hl=arm64 ;; \
      *) echo "unsupported architecture" >&2; exit 1 ;; \
    esac \
 && curl -fsSL "https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-${musl}.tar.gz" \
    | tar -xz -C /usr/local/bin just \
 && curl -fsSL "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-${musl}.tar.gz" \
    | tar -xz --strip-components=1 -C /usr/local/bin "cargo-deny-${CARGO_DENY_VERSION}-${musl}/cargo-deny" \
 && curl -fsSL -o /usr/local/bin/lefthook "https://github.com/evilmartians/lefthook/releases/download/v${LEFTHOOK_VERSION}/lefthook_${LEFTHOOK_VERSION}_Linux_${lh}" \
 && curl -fsSL -o /usr/local/bin/hadolint "https://github.com/hadolint/hadolint/releases/download/v${HADOLINT_VERSION}/hadolint-linux-${hl}" \
 && chmod +x /usr/local/bin/lefthook /usr/local/bin/hadolint

RUN rustup component add rustfmt clippy

ENV DEEBOO_IN_CONTAINER=1
WORKDIR /work
