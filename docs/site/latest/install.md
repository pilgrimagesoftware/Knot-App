---
title: Installation
version: latest
permalink: /latest/install/
---

# Installation

Knot is a desktop workspace for running multiple AI coding agents. Download a packaged build from the [Knot releases page](https://github.com/pilgrimagesoftware/Knot-App/releases), or build the Rust application from source.

## Supported platforms

| Platform | Status | Installation path |
|---|---|---|
| macOS | Supported | Download the macOS application from [releases](https://github.com/pilgrimagesoftware/Knot-App/releases). |
| Ubuntu | Supported for source builds | Build from source with Rust. |
| Other Linux distributions | Expected to work with a supported GPUI-Kit desktop environment | Build from source and report platform issues in the repository. |
| Windows | Not supported | No installer or tested build is currently provided. |

## Download a release

1. Open [Knot releases](https://github.com/pilgrimagesoftware/Knot-App/releases).
2. Download the asset for your operating system and architecture.
3. On macOS, move `Knot.app` to `/Applications` before opening it.
4. Launch Knot, create or open a workspace, then add the coding agents you want it to manage.

## macOS quarantine and signing

Release builds may be unsigned. macOS can prevent an unsigned application from making local network connections until you remove quarantine and ad-hoc sign it. After moving Knot to `/Applications`, run:

```sh
xattr -d com.apple.quarantine "/Applications/Knot.app"
codesign --force --deep --sign - "/Applications/Knot.app"
```

Run these commands only for a Knot app you downloaded from the project's release page. If Gatekeeper still blocks the app, confirm its path and repeat the commands against that path.

## Build from source

Source builds are supported on macOS and Ubuntu. Install the Rust version in `rust-toolchain.toml`, Git 2.30 or newer, a supported GPUI-Kit desktop environment, and at least one coding CLI such as Claude Code, Codex, Gemini CLI, GitHub Copilot, or OpenCode.

```sh
git clone https://github.com/pilgrimagesoftware/Knot-App.git
cd Knot-App
make
cargo run -p knot
```

Use `make fmt`, `make lint`, `make test`, and `make build` when developing from source.

## Agent shell notes

Knot's shell agents load your normal shell configuration. Prompt themes that start a daemon for every shell can create unnecessary background processes in agent terminals. See [`docs/agent-shells.md`](https://github.com/pilgrimagesoftware/Knot-App/blob/develop/docs/agent-shells.md) for the shell configuration guidance.
