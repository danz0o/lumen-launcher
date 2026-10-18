# Lumen Launcher

Lumen Launcher is a native cross-platform Minecraft: Java Edition launcher built with Rust and Tauri 2. Its interface is plain HTML, CSS, and JavaScript; there is no Electron runtime and no frontend bundler.

## Requirements

- Rust stable and Cargo
- Tauri CLI 2 (`cargo install tauri-cli --version "^2" --locked`)
- Java is downloaded as needed by the launcher; a custom Java executable can be configured instead
- On Linux, the Tauri/WebKitGTK development libraries listed in the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/#linux)
- A Microsoft Entra/Azure application registration configured as a public client for device-code sign-in

## Run

```sh
cd lumen-launcher
cargo tauri dev
```

For official Microsoft sign-in, set the non-secret application ID when building/running:

```sh
LUMEN_MICROSOFT_CLIENT_ID="your-application-client-id" cargo tauri dev
```

The application ID is not an account credential. Microsoft account tokens are stored in the operating system keyring, not in launcher JSON files.

To build a platform bundle:

```sh
LUMEN_MICROSOFT_CLIENT_ID="your-application-client-id" cargo tauri build
```

Run the build on each target operating system to produce that platform's installer. The Linux, macOS, and Windows bundle formats are configured in `tauri.conf.json`.

## Included in this source build

- Microsoft device-code authentication through Xbox Live and XSTS, Minecraft Java profile/ownership verification, refresh tokens, and operating-system keyring storage
- Multiple account records, account switching, and custom offline-mode profiles with Minecraft-compatible offline UUIDs
- Official release/snapshot manifest lookup, SHA-1 checked client/library/asset downloads, retry handling, basic legacy asset mapping, and per-instance folders
- Instance creation, clone, delete, ZIP import/export, per-instance memory/JVM/Java settings, and Modrinth `.mrpack` import
- Temurin runtime selection/download for Java 8, 17, and 21
- Fabric and Quilt metadata/profile installation; Forge and NeoForge installer invocation
- Modrinth search, compatible version selection, required project dependencies, local enable/disable, and instance modpack files
- Live game stdout/stderr, crash report listing/reading, quick-join server address, and English/Arabic layout direction

## Data and account behavior

Launcher metadata and shared Minecraft files are stored in Tauri's application data directory. Instance-specific saves, mods, resource packs, and options are in each instance's `game` folder. Microsoft access and refresh tokens are saved through the system keyring.

Offline profiles are intended only for servers configured to accept offline-mode connections. They do not authenticate an account, verify ownership, or bypass online-mode server checks.

## Current limitations

- CurseForge API support, offline skin editing, and automatic mod update checking are not included in this initial implementation.
- Skin viewing/customization beyond the account avatar is not included in this initial implementation.
- Installation progress reports download phases, not per-file byte-level progress.
- Loader installer output varies by game and loader version; review installer errors in the launcher before retrying.
- The launcher source is native desktop software. It is not a hosted web app and cannot be launched from the browser preview.

## Project structure

- `src/auth.rs` — official and offline account flows
- `src/versions.rs` — manifest, game assets, libraries, rules, merges, and integrity checks
- `src/java.rs` — managed Temurin runtime lookup and extraction
- `src/loaders.rs` — Fabric, Quilt, Forge, and NeoForge installation
- `src/instances.rs` — isolated instance storage and ZIP operations
- `src/mods.rs` — Modrinth API and `.mrpack` installation
- `src/launch.rs` — argument construction, process output, and crash reports
- `src/storage.rs` — metadata persistence and keyring access
- `ui/` — Tauri WebView interface, plain HTML/CSS/JavaScript

