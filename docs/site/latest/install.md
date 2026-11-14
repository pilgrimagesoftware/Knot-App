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
| Linux (x86_64) | CI builds, lints and tests on Ubuntu | Download the `tar.gz` from [releases](https://github.com/pilgrimagesoftware/Knot-App/releases), or build from source. Other distributions are expected to work with a supported GPUI-Kit desktop environment. |
| Windows (x86_64) | CI builds and lints; several crates' tests still fail on Windows | Download the `zip` from [releases](https://github.com/pilgrimagesoftware/Knot-App/releases), or build from source. |

## Download a release

1. Open [Knot releases](https://github.com/pilgrimagesoftware/Knot-App/releases).
2. Download the asset for your operating system and architecture, and `SHA256SUMS` from the same release.
3. Check the download from the folder it is in. `shasum -a 256 -c SHA256SUMS --ignore-missing` must print `OK` beside the file name. The Linux and Windows archives carry their own `<archive>.sha256` file, checked the same way with `sha256sum -c`.
4. On macOS, move `Knot.app` to `/Applications` before opening it. On Linux, extract the `tar.gz` and run `knot`. On Windows, extract the `zip` and run `knot.exe`. The Linux and Windows archives are unsigned and have no installer.
5. Launch Knot, create or open a workspace, then add the coding agents you want it to manage.

Each release also carries `Knot_<version>_aarch64.cdx.json`, a [CycloneDX](https://cyclonedx.org) software bill of materials that lists the Rust crates built into the app.

## macOS quarantine and signing

Release builds may be unsigned. macOS can prevent an unsigned application from making local network connections until you remove quarantine and ad-hoc sign it. After moving Knot to `/Applications`, run:

```sh
xattr -d com.apple.quarantine "/Applications/Knot.app"
codesign --force --deep --sign - "/Applications/Knot.app"
```

Run these commands only for a Knot app you downloaded from the project's release page. If Gatekeeper still blocks the app, confirm its path and repeat the commands against that path.

## Build from source

Source builds are supported on macOS, Ubuntu and Windows. Install the Rust version in `rust-toolchain.toml`, Git 2.30 or newer, a supported GPUI-Kit desktop environment, and at least one coding CLI such as Claude Code, Codex, Gemini CLI, GitHub Copilot, or OpenCode.

```sh
git clone https://github.com/pilgrimagesoftware/Knot-App.git
cd Knot-App
make
cargo run -p knot
```

Use `make fmt`, `make lint`, `make test`, and `make build` when developing from source.

## Agent shell notes

Knot's shell agents load your normal shell configuration. Prompt themes that start a daemon for every shell can create unnecessary background processes in agent terminals. See the [agent shells reference]({{ '/latest/reference/agent-shells/' | relative_url }}) for the shell configuration guidance.
