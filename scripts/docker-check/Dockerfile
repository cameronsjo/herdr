# Build/test image for scripts/docker-check.sh.
#
# Exists because this fork is developed on machines where herdr cannot build
# natively: a pinned Zig's bundled libSystem predates the host macOS SDK
# (herdrdev/herdr#285). Zig targeting Linux uses its own bundled libc, so the
# same Zig pin builds fine in here regardless of the host OS.
FROM ubuntu:24.04

ARG RUST_VERSION=1.96.1
ARG ZIG_VERSION=0.15.2
ARG JUST_VERSION=1.36.0

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq \
    && apt-get install -y -qq curl git build-essential xz-utils ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH="/opt/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile minimal --default-toolchain "${RUST_VERSION}" -c rustfmt,clippy

ARG ZIG_VERSION
RUN set -eu; \
    case "$(uname -m)" in \
      x86_64) zig_triple=x86_64-linux ;; \
      aarch64) zig_triple=aarch64-linux ;; \
      *) echo "unsupported arch: $(uname -m)" >&2; exit 1 ;; \
    esac; \
    curl -sSfL "https://ziglang.org/download/${ZIG_VERSION}/zig-${zig_triple}-${ZIG_VERSION}.tar.xz" -o /tmp/zig.tar.xz; \
    mkdir -p /opt/zig; \
    tar xf /tmp/zig.tar.xz -C /opt/zig --strip-components=1; \
    rm /tmp/zig.tar.xz
ENV PATH="/opt/zig:${PATH}"

RUN cargo install cargo-nextest --locked --quiet

ARG JUST_VERSION
RUN set -eu; \
    case "$(uname -m)" in \
      x86_64) just_triple=x86_64-unknown-linux-musl ;; \
      aarch64) just_triple=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported arch: $(uname -m)" >&2; exit 1 ;; \
    esac; \
    curl -sSfL "https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-${just_triple}.tar.gz" -o /tmp/just.tar.gz; \
    tar xzf /tmp/just.tar.gz -C /opt/cargo/bin just; \
    chmod +x /opt/cargo/bin/just; \
    rm /tmp/just.tar.gz

WORKDIR /work
