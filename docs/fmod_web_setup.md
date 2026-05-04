# FMOD Studio on Web (HTML5)

This project keeps **desktop** audio on native FMOD libraries ([`src/sound/fmod.rs`](../src/sound/fmod.rs)). The **WebGL / wasm** path uses Firelight’s HTML5 runtime in the browser plus a small bridge ([`web/game_fmod_bridge.js`](../web/game_fmod_bridge.js)).

## License and SDK

You need a FMOD license that includes **HTML5** and the **FMOD Studio API HTML5** package from [FMOD downloads](https://www.fmod.com/download). This repository does not redistribute `fmodstudio.js` or `fmodstudio.wasm`.

## Files to copy from the HTML5 package

Typical layout from the SDK (names may vary slightly by version):

| Copy into repo | Source (conceptually) |
|----------------|------------------------|
| `third_party/fmod/fmodstudio.js` | Studio API JavaScript loader |
| `third_party/fmod/fmodstudio.wasm` | Same directory as `fmodstudio.js` (browser resolves it next to the script URL) |

Optional: keep a short `third_party/fmod/README.txt` noting your FMOD version.

## Banks

- Desktop banks stay under `resources/fmod/Desktop/` (unchanged).
- For production web builds, export **HTML5** banks from FMOD Studio into **`resources/fmod/Web/`** (e.g. `Master.bank`, `Master.strings.bank`). The bridge fetches them with a URL **resolved from the current page** (`new URL("resources/fmod/Web/...", location.href)`), so the same layout works on `localhost`, itch.io subpaths (`/html/.../`), etc.
- **Dev fallback:** if `resources/fmod/Web/` is missing, `scripts/build_web.ps1` / `build_web.sh` copies **`*.bank` from `resources/fmod/Desktop/`** into `dist/resources/fmod/Web/` so local `cargo web-dev` can find files. Prefer real HTML5 banks for shipping builds; Desktop bank bytes are not always valid on HTML5 depending on your FMOD Studio settings.

## Dev server URL root

`serve-web` / `dist/` serves files at the document root locally; bank files still live at **`resources/fmod/Web/...`** relative to `index.html`. The build script fills that folder from `resources/fmod/Web` or the Desktop `*.bank` fallback.

## itch.io and `dist.zip`

`cargo web-dev` runs `scripts/build_web.*`, then zips **everything under `dist/`** into `dist.zip` (see `src/bin/serve_web.rs` — no path exclusions). So **`Master.bank` is in the zip iff it exists as `dist/resources/fmod/Web/Master.bank` after the build.**

Before uploading, unzip `dist.zip` locally and confirm `resources/fmod/Web/Master.bank` is inside. The build scripts print a **warning** if that file is still missing (usually: banks are gitignored and the machine that ran the build had no `resources/fmod/Desktop/*.bank` or `resources/fmod/Web/`).

If itch showed **403** while requesting `https://html-classic.itch.zone/resources/...` (domain root), the game was probably hosted under a **subpath** (e.g. `/html/<id>/`) and root-relative URLs were wrong. The bridge now resolves bank URLs from **`window.location.href`**. If problems remain, confirm the zip layout and try itch’s HTML game upload help; ensure `index.html` is at the **top level** of the zip (as `serve-web` already does).

## Enabling the Rust + JS path

The wasm binary must be built with the **`web_audio`** feature (in addition to `web`) so `SoundManager` calls `LearnOpenglFmod` (see `Cargo.toml`).

**`cargo web-dev`** (alias for `serve-web`) runs `scripts/build_web.*`, which already passes **`--features web,web_audio`**.

Manual build:

```bash
cargo build --target wasm32-unknown-unknown --no-default-features --features web,web_audio
```

`index.html` loads `third_party/fmod/fmodstudio.js` if present, then `game_fmod_bridge.js`, then starts the wasm app. If the FMOD scripts are missing, audio calls no-op and the game still runs.

The bridge follows FMOD’s HTML5 **load banks** pattern: `FS_createPreloadedFile` in `preRun` fetches `Master.bank` / `Master.strings.bank` over HTTP into the Emscripten virtual FS, then `loadBankFile("/Master.bank", …)` reads from that FS. Those HTTP fetches use paths **relative to the page URL** next to `index.html` (same `bankBase` you pass from Rust, default `resources/fmod/Web`).

## Asset manifest and bank files

Preloaded wasm assets come from `asset-manifest.json` (see `web/index.html`). **FMOD Web banks and SDK binaries are excluded** from that manifest so they are not pulled into `window.__learn_opengl_rs_assets`; FMOD loads banks via HTTP from normal URLs under `dist/resources/fmod/Web/`, consistent with a future fetch-based asset pipeline for large binaries.

## User gesture

Browsers require a user gesture to start audio. The bridge starts the FMOD Emscripten runtime after the first pointer or key event on `document`, then runs Studio initialization. Until then, `SoundManager` calls are ignored.
