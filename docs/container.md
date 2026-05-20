# Container-first brew-code workflows

This repo already had **container detection** in the Rust runtime before this document was added:

- `rust/crates/runtime/src/sandbox.rs` detects Docker/Podman/container markers such as `/.dockerenv`, `/run/.containerenv`, matching env vars, and `/proc/1/cgroup` hints.
- `rust/crates/rusty-claude-cli/src/main.rs` exposes that state through the `brewcode sandbox` / `cargo run -p rusty-claude-cli -- sandbox` report.
- `.github/workflows/rust-ci.yml` runs on `ubuntu-latest`, but it does **not** define a Docker or Podman container job.
- Before this change, the repo did **not** have a checked-in `Dockerfile`, `Containerfile`, or `.devcontainer/` config.

This document adds a small checked-in `Containerfile` so Docker and Podman users have one canonical container workflow.

## What the checked-in container image is for

The root [`../Containerfile`](../Containerfile) gives you a reusable Rust build/test shell with the extra packages this workspace commonly needs (`git`, `pkg-config`, `libssl-dev`, certificates).

It does **not** copy the repository into the image. Instead, the recommended flow is to bind-mount your checkout into `/workspace` so edits stay on the host.

## Build the image

From the repository root:

### Docker

```bash
docker build -t brew-code-dev -f Containerfile .
```

### Podman

```bash
podman build -t brew-code-dev -f Containerfile .
```

## Run `cargo test --workspace` in the container

These commands mount the repo, keep Cargo build artifacts out of the working tree, and run from the Rust workspace at `rust/`.

### Docker

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -e CARGO_TARGET_DIR=/tmp/brewcode-target \
  -w /workspace/rust \
  brew-code-dev \
  cargo test --workspace
```

### Podman

```bash
podman run --rm -it \
  -v "$PWD":/workspace:Z \
  -e CARGO_TARGET_DIR=/tmp/brewcode-target \
  -w /workspace/rust \
  brew-code-dev \
  cargo test --workspace
```

If you want a fully clean rebuild, add `cargo clean &&` before `cargo test --workspace`.

## Open a shell in the container

### Docker

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -e CARGO_TARGET_DIR=/tmp/brewcode-target \
  -w /workspace/rust \
  brew-code-dev
```

### Podman

```bash
podman run --rm -it \
  -v "$PWD":/workspace:Z \
  -e CARGO_TARGET_DIR=/tmp/brewcode-target \
  -w /workspace/rust \
  brew-code-dev
```

Inside the shell:

```bash
cargo build --workspace
cargo test --workspace
cargo run -p rusty-claude-cli -- --help
cargo run -p rusty-claude-cli -- sandbox
```

The `sandbox` command is a useful sanity check: inside Docker or Podman it should report `In container true` and list the markers the runtime detected.

## Bind-mount this repo and another repo at the same time

If you want to run `brewcode` against a second checkout while keeping `brew-code` itself mounted read-write:

### Docker

```bash
docker run --rm -it \
  -v "$PWD":/workspace \
  -v "$HOME/src/other-repo":/repo \
  -e CARGO_TARGET_DIR=/tmp/brewcode-target \
  -w /workspace/rust \
  brew-code-dev
```

### Podman

```bash
podman run --rm -it \
  -v "$PWD":/workspace:Z \
  -v "$HOME/src/other-repo":/repo:Z \
  -e CARGO_TARGET_DIR=/tmp/brewcode-target \
  -w /workspace/rust \
  brew-code-dev
```

Then, for example:

```bash
cargo run -p rusty-claude-cli -- prompt "summarize /repo"
```

## Notes

- Docker and Podman use the same checked-in `Containerfile`.
- The `:Z` suffix in the Podman examples is for SELinux relabeling; keep it on Fedora/RHEL-class hosts.
- Running with `CARGO_TARGET_DIR=/tmp/brewcode-target` avoids leaving container-owned `target/` artifacts in your bind-mounted checkout.
- For non-container local development, keep using [`../USAGE.md`](../USAGE.md) and [`../rust/README.md`](../rust/README.md).
