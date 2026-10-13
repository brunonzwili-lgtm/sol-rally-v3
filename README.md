# Sol Rally

> Browser-based 3D kart racing powered by Solana and MagicBlock.

[Play Sol Rally](https://solrally.vercel.app/)

Sol Rally is a browser-based 3D kart racing game compiled to WebAssembly with
[Emscripten](https://emscripten.org/), paired with an [Anchor](https://www.anchor-lang.com/)
program on Solana that implements Grand Prix entry fees and prize-pool settlement.

**This repository is in active development.** The kart racing game itself runs in the
browser today. The Solana settlement program compiles and deploys. The wiring between
them — wallet connection, on-chain race state, MagicBlock real-time game state — is
**not implemented yet**. Sections below are labelled accordingly, and the
[Current Status](#current-status) section is the authoritative summary of what works.

---

## Table of Contents

- [Overview](#overview)
- [Why Sol Rally](#why-sol-rally)
- [Features](#features)
- [Game Modes](#game-modes)
- [Screenshots and Gameplay](#screenshots-and-gameplay)
- [Technology Stack](#technology-stack)
- [System Architecture](#system-architecture)
- [How It Works](#how-it-works)
- [Solana Program](#solana-program)
- [MagicBlock Integration](#magicblock-integration)
- [Prize Pool and Settlement](#prize-pool-and-settlement)
- [Security Considerations](#security-considerations)
- [Getting Started](#getting-started)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Environment Variables](#environment-variables)
- [Running Locally](#running-locally)
- [Building for Production](#building-for-production)
- [Testing](#testing)
- [Deployment](#deployment)
- [Project Structure](#project-structure)
- [Current Status](#current-status)
- [Roadmap](#roadmap)
- [Known Limitations](#known-limitations)
- [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)
- [License](#license)

---

## Overview

Sol Rally is a 3D kart racing game written in C++ and built on the Irrlicht engine.
Shipping a game of this scope to a browser requires solving two hard problems, and
Sol Rally's architecture is entirely shaped by them.

**Problem one: the engine is native.** Sol Rally links against SDL2, OpenAL,
libpng, libjpeg, HarfBuzz and a dozen other native libraries, and it ships a very
large asset tree. To run in a browser, the whole engine is cross-compiled to
WebAssembly with Emscripten, the native dependencies are rebuilt to `wasm32` and linked
statically, and the graphics backend is pointed at WebGL. The asset tree is repacked
into split, compressed tarballs that the browser downloads on demand.

**Problem two: the game is ~90 MB of assets.** A naive static build makes a player wait
through a single enormous download before a single frame renders. Sol Rally instead
defers all asset loading behind an explicit start gate, streams the asset archive in
20 MB chunks with visible progress, and caches the decompressed result in IndexedDB so
that a second visit skips the network entirely.

On the Solana side, an Anchor program manages a **Grand Prix** account: an organiser
creates one with an entry fee, players pay that fee to enter, and a resolve instruction
distributes the accumulated pool. The account is a PDA seeded by the organiser, so the
organiser provably controls it and players cannot be redirected to a look-alike account.

The long-term direction is to make race state itself on-chain and driven by
[MagicBlock](https://magicblock.gg/), so that game state updates at Solana block rate
rather than at server tick rate. That work has not started.

### What runs today

| Component | State |
| --- | --- |
| Sol Rally C++ engine compiled to `wasm32` | Working |
| WebGL rendering, audio, input, persistent saves | Working |
| Chunked asset download with progress reporting | Working |
| IndexedDB asset + save caching | Working |
| Landing page and deployment pipeline | Working |
| Anchor program (`init_gp` / `enter_gp` / `resolve_gp` / `cancel_gp`) | Compiles; logic incomplete (see [Prize Pool and Settlement](#prize-pool-and-settlement)) |
| Browser wallet connection | **Not implemented** |
| Solana program called from the frontend | **Not implemented** |
| MagicBlock real-time game state | **Not implemented** |
| Solana-themed karts and tracks | **Not implemented** — see [Current Status](#current-status) |

---

## Why Sol Rally

**A blockchain game that actually ships the game.** Most on-chain gaming projects
demonstrate a wallet flow and a transaction, then show a video of gameplay captured
elsewhere. Sol Rally is the opposite shape: the complete game runs, in a browser, at
`https://solrally.vercel.app/`, with the settlement program as a real component of the
repository rather than a slide in a deck.

**The engine problem is genuinely solved.** Getting Sol Rally into a browser is not
a matter of wrapping an existing WebGL build. It required rebuilding SDL2, OpenAL, zlib,
libpng, libjpeg and HarfBuzz for `wasm32`, working around the Irrlicht GLES 3 path,
replacing the native tar reader with a streaming-friendly one, and rearchitecting asset
delivery. That work is committed and reproducible.

**Cheap to run, expensive to host.** The deployed artifact is pure static files. There
is no game server, no per-tick infrastructure, and no database. Hosting cost is a CDN
and a static host. This is a direct consequence of the on-chain design: the settlement
layer is the only stateful component, and it is the chain.

**An honest substrate for iteration.** The game engine, the asset pipeline and the
settlement layer are cleanly separated. Swapping the settlement layer, adding a
leaderboard, or building a real-time multiplayer mode does not require touching the
engine.

### Honest framing

The landing page at `wasm/web/index.html` currently advertises four Solana-themed karts
(Validator, Bonk, Jupiter, Sol), five Solana-themed tracks (Sol Dunes, Orca Lagoon,
Marinade Trench, Pyth Peak, Tensor Town), and three game modes (Mainnet Race, Slot
Sprint, MEV Battle). **None of these are implemented.** The kart and track thumbnails
on that page are static PNGs in `wasm/web/images/`; the actual playable content is stock
Sol Rally. This README documents the code as it exists. The landing page copy should
be brought into line, or the features should be built.

---

## Features

### Implemented

- **Complete Sol Rally in the browser.** Full game loop, physics, AI opponents,
  item boxes, particle effects and audio, running at 60 FPS on a modern desktop browser.
- **Zero-install play.** No plugin, no launcher, no account. Navigation to a URL is the
  entire onboarding flow.
- **Player-selectable asset quality.** Low (256px textures), Medium (512px) and
  High (1024px) bundles, chosen before launch so a player on a weak connection is never
  forced to download the 1K texture set.
- **Chunked, resumable-feeling asset download.** The asset archive is split into 20 MB
  chunks with a manifest declaring the exact total byte count; the download reports
  live `chunk N/M` and `MiB downloaded/total` progress.
- **Full IndexedDB asset cache.** After the first load, subsequent visits restore assets
  from IndexedDB and skip the network entirely. A `data_version` constant invalidates
  the cache when assets change.
- **Persistent user data.** Emscripten's `IDBFS` is mounted at `/home` and synced on
  launch and exit, so saves, settings and profiles survive reloads and browser restarts.
- **Explicit error surfacing.** WASM aborts and uncaught runtime errors are caught and
  rendered into the loading overlay, with the last 10 lines of runtime stderr, instead of
  leaving the player with a silent black canvas.
- **Cache-poisoning defences.** The manifest is fetched with a cache-busting query and
  `cache: "no-store"`; the `.wasm` URL is versioned; downloads verify both the total
  byte count and per-chunk bounds.
- **Deterministic static deployment.** The full site is served as static files with no
  build step at deploy time. `vercel.json` and `_headers` set the cross-origin isolation
  headers the engine requires and cache policy tuned per asset class.
- **Anchor settlement program.** Grand Prix account creation, entry-fee collection and
  resolution instructions, with a PDA-seeded authority and a custom error type.

### Planned, not implemented

- Browser wallet connection (Phantom, Backpack, wallet-standard).
- Calling the Anchor program from the game to enter a Grand Prix.
- Solana-themed karts and tracks replacing the stock Sol Rally content.
- MagicBlock-driven real-time race state.
- On-chain leaderboards and results.
- Multiplayer.

---

## Game Modes

**This section describes intent, not shipped functionality.** The game currently ships
with the stock Sol Rally mode set: single race, time trial, and Grand Prix against
AI opponents, selectable from the in-game menus.

The Solana-specific modes below are the design target.

### Race (planned)

The standard kart race. Every participant drives the same track; finishing order
determines payout position. This maps most directly onto the `resolve_gp` instruction
and is the mode the settlement program was designed around.

### Slot Sprint (planned)

A short, high-tempo race on a compressed track layout, framed as a series of slots — each
player occupies a "slot" in the entry list, and the race resolves quickly enough to feel
like a slot machine pull. Intended as the low-friction, high-frequency entry mode.

### MEV Battle (planned)

A PvP mode in which the race order can be contested, framed around the searcher /
validator / user roles that already exist in Solana's MEV supply chain. This is the most
speculative of the three modes and depends entirely on the MagicBlock integration
existing first, since it requires sub-second state updates.

### Grand Prix with on-chain entry fees (program written, frontend unwired)

The settlement layer for all modes. A Grand Prix account is created by an organiser with
a fixed entry fee in lamports. Each player pays the fee to enter. Resolution
distributes the pool. See [Prize Pool and Settlement](#prize-pool-and-settlement) for
the current limitations of this implementation.

---

## Screenshots and Gameplay

### Landing page

The site opens on a full-screen canvas behind a centred panel: the Sol Rally wordmark,
a `SOLANA × MAGICBLOCK` badge, a feature grid, the four (currently unimplemented) kart
thumbnails, a track hero image, a texture-quality selector and a `▶ START GAME` button.

While assets download, the panel reports live progress. When the game launches, the
overlay is hidden and the canvas takes the full viewport.

```html
<div class="feat">
  <div class="ic">🏁</div>
  <div><b>4 Solana Karts</b><small>Play as Validator, Bonk, Jupiter or Sol.</small></div>
</div>
<div class="feat">
  <div class="ic">🛵</div>
  <div><b>Grand Prix<span class="soon">SOON</span></b><small>SOL prize pools and onchain leaderboards.</small></div>
</div>
```

Note the `SOON` badge on Grand Prix. The same honesty should be applied to the other
three cards.

### In-game

The full Sol Rally experience is playable: character select, kart select, track
select, four karts (`tux`, `beastie`, `emule`, `kiki`), and the stock track set —
`abyss`, `battleisland`, `sandtrack`, `scotland`, `volcano_island`, `hacienda`, plus
the tutorial and cutscene tracks.

### Asset delivery progress

```
Downloading manifest for data_low.tar.gz...
Downloading data_low.tar.gz... (chunk 1/5, 0/85MiB)
Downloading data_low.tar.gz... (chunk 2/5, 19/85MiB)
Downloading data_low.tar.gz... (chunk 3/5, 39/85MiB)
Decompressing data_low.tar.gz...
Saving data_low.tar.gz to the cache...
Extracting data_low.tar.gz...
Extracting files... (2000 done)
Extracted 3695 files and 281 directories
```

---

## Technology Stack

### Game engine

| Layer | Technology |
| --- | --- |
| Language | C++17 |
| Engine | Irrlicht (legacy renderer, OpenGL ES 2 path) |
| Platform | `wasm32-unknown-emscripten` via Emscripten |
| Build system | CMake 3.5+, driven by `wasm/build.sh` |
| Graphics | WebGL (GLES 2). GLES 3 does not work on this port |
| Audio | OpenAL → Web Audio |
| Input | SDL2 (keyboard, mouse, gamepad, touch) |

### Web layer

| Layer | Technology |
| --- | --- |
| Loader | Vanilla ES modules, no framework, no bundler |
| Compression | `pako@2.1.0` via jsDelivr ESM CDN |
| Asset archive | `tar --format=ustar` + `gzip -9`, split into 20 MB chunks |
| Asset cache | IndexedDB (`stk_db` / `stk_store`) |
| Save persistence | Emscripten `IDBFS` mounted at `/home` |
| Tar reader | Hand-written streaming reader in `script.js` (~65 lines) |

### On-chain layer

| Layer | Technology |
| --- | --- |
| Language | Rust 1.89.0 |
| Framework | Anchor 0.32.1 |
| Build | `cargo` workspace, `cdylib` + `lib` |
| Tooling | Anchor CLI, Yarn 1.x, TypeScript 5.7 |
| Cluster | `localnet` (no devnet or mainnet configuration committed) |
| Program ID | `GtzDo8g279ocmbeUUSLUEKrYBBUsGWWYYWkUjHoR9TRe` |

The browser client is the notable gap: there is no `@solana/web3.js`, no wallet
adapter, and no IDL client in `wasm/web/`. When it is added it will be loaded as an ES
module from a CDN to match the existing no-bundler approach.

---

## System Architecture

### High-level

```
╔═════════════════════════════════════════════════════════════════════════════════════════════╗
║                                                                                             ║
║ PLAYER'S BROWSER                                                                            ║
║                  ┌────────────────────────────────────────────────────────────────────────┐ ║
║                  │ index.html                                                             │ ║
║                  │ ┌─────────────────────────┐  ┌──────────────────────────────┐          │ ║
║                  │ │ run() override shim     │  │ #info_container (overlay)    │          │ ║
║                  │ │ intercepts first run()  │  │  brand / features / karts /  │          │ ║
║                  │ │ to detect WASM ready    │  │  quality / START button      │          │ ║
║                  │ └───────────┬─────────────┘  └────────────┬─────────────────┘          │ ║
║                  └────────────────────────────────────────────────────────────────────────┐ ║
║                                                                                             ║
║                  │ <script src="/game/stk20.js">                                            ║
║                  ▼                                                                          ║
║                  ┌────────────────────────────────────────────────────────────────────────┐ ║
║                  │ Emscripten loader  (stk20.js, 368 KB)                                  │ ║
║                  │ PThread pool · Module.FS (MEMFS) · mainLoop                            │ ║
║                  │ SOCKFS / WS stubs · GL / EGL / AL shims                                │ ║
║                  │ locateFile → supertuxkart.wasm?v=<version>                             │ ║
║                  └────────────────────────────────────────────────────────────────────────┐ ║
║                                                                                             ║
║                  │ instantiate (15,284,722 bytes)                                           ║
║                  ▼                                                                          ║
║                  ┌────────────────────────────────────────────────────────────────────────┐ ║
║                  │ supertuxkart.wasm   (15.3 MB)                                          │ ║
║                  │ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────────┐          │ ║
║                  │ │ game loop  │ │ Irrlicht   │ │ physics /  │ │ SDL2 input,  │          │ ║
║                  │ │ + mainloop │ │ GL (GLES2) │ │ collision  │ │ OpenAL audio  │         │ ║
║                  │ └────────────┘ └────────────┘ └────────────┘ └──────────────┘          │ ║
║                  │                                                                        │ ║
║                  │ Module.FS   /home ← IDBFS (persistent saves)                           │ ║
║                  │             /data ← MEMFS, from extract_tar()                          │ ║
║                  └────────────────────────────────────────────────────────────────────────┐ ║
║                                                                                             ║
║                  ▲                                                                          ║
║                  │ onAbort / printErr / window.onerror                                      ║
║                  ┌────────────────────────────────────────────────────────────────────────┐ ║
║                  │ script.js  (vanilla ES module, no bundler)                             │ ║
║                  │ load_config() → load_idbfs() → [START] → run()                         │ ║
║                  │ download_chunks() → pako.inflate() → untar()                           │ ║
║                  │ install_runtime_error_hooks() → show_fatal()                           │ ║
║                  └────────────────────────────────────────────────────────────────────────┐ ║
║                                                                                             ║
║                  ▼                           ▼                                              ║
║                  ┌────────────────────────────────────────────────────────────────────────┐ ║
║                  │ IndexedDB                     static asset host                        │ ║
║                  │ stk_db / stk_store            (Vercel / GitHub Pages)                  │ ║
║                  │   /version       → 21          /game/data_low.tar.gz.manifest          │ ║
║                  │   data_low.tar.gz → chunks    /game/data_low.tar.gz.00..04             │ ║
║                  │   data_mid.tar.gz → chunks    /game/supertuxkart.wasm                  │ ║
║                  │   data_high.tar.gz→ chunks    /images/karts, /images/tracks            │ ║
║                  └────────────────────────────────────────────────────────────────────────┐ ║
╚═════════════════════════════════════════════════════════════════════════════════════════════╗
╔═════════════════════════════════════════════╗ ╔═════════════════════════════════════════════╗
╔═ SOLANA CLUSTER ════════════════════════════╗ ╔═ MAGICBLOCK (not integrated) ═══════════════╗
║                                             ║ ║                                             ║
║ sol_rally_program                           ║ ║ Ephemeral / TEE accounts                    ║
║   GtzDo8g279…R9TRe                          ║ ║                                             ║
║                                             ║ ║ sub-second game-state updates               ║
║   init_gp    → create GrandPrix PDA         ║ ║ (position, laps, items)                     ║
║   enter_gp   → collect entry fee            ║ ║                                             ║
║   resolve_gp → distribute pool              ║ ║ no SDK dependency in repo                   ║
║   cancel_gp  → close + refund               ║ ╚═════════════════════════════════════════════╗
║                                             ║               │ permissioned writes            
╚═════════════════════════════════════════════╗                                                
              │ lamports                                                                       
╔═ GrandPrix PDA ═════════════════════════════╗                                                
║   seeds  = [b"gp", authority]               ║                                                
║   fields = authority, bump, entry_fee,      ║                                                
║           total_fees, total_entried         ║                                                
╚═════════════════════════════════════════════╗                                                
╚═════════════════════════════════════════════╝ ╚═════════════════════════════════════════════╝
```

### Build and deploy pipeline

```
┌────────────────────────────────────────────────────────────────────────┐
│ src/    C++ engine                                                     │
│ data/   stkgui, tracks, challenges                                     │
└────────────────────────────────────────────────────────────────────────┘
  │
  └── emcmake cmake -DNO_SHADERC=on -DCHECK_ASSETS=off
                          -DCMAKE_PREFIX_PATH=wasm/prefix
  ├───────────────────────────────────────────────────────────────────────┤
   ┌────────────────────────────────────────────────────────────────────┐
   │ bin/supertuxkart.js    (367 KB)                                    │
   │ bin/supertuxkart.wasm  (15.3 MB)                                   │
   └────────────────────────────────────────────────────────────────────┘
  ├───────────────────────────────────────────────────────────────────────┤
     wasm/build.sh:  cp ./bin/*  →  wasm/web/game/
     wasm/patch_js.py + fragments/  →  patched loader
  ├───────────────────────────────────────────────────────────────────────┤
   ┌─ wasm/web/game/ ───────────────────────────────────────────────────┐
   │ supertuxkart.wasm?v=21   loaded by the page                        │
   │ stk20.js                   loader named in index.html              │
   │ supertuxkart.js            raw build output                        │
   │                                                                    │
   │ data_low.tar.gz.{manifest,00..04}                                  │
   │ data_low/                  3,695 extracted files                   │
   │                                                                    │
   │ └── web server root = wasm/web/                                    │
   └────────────────────────────────────────────────────────────────────┘
  ├───────────────────────────────────────────────────────────────────────┤
     wasm/pack_assets.sh ./stk-assets
         android/generate_assets.sh  TEXTURE_SIZE=256|512|1024
         optimize_data.sh           pngquant, optipng, advancecomp
         tar --format=ustar | gzip -9 | split -b 20m -d -a 2
  ├───────────────────────────────────────────────────────────────────────┤
   ┌────────────────────────────────────────────────────────────────────┐
   │ /version    = 21         invalidates player cache                  │
   │ manifest   = 89744472    exact byte total                          │
   │ 00 .. 04   = 5 chunks    86 MiB total                              │
   └────────────────────────────────────────────────────────────────────┘
```

### Asset pipeline detail

Three quality tiers are generated from the same `stk-assets` source tree:

| Tier | Texture size | `TEXTURE_SIZE` | Approx. download |
| --- | --- | --- | --- |
| `low` | 256 px | `256` | 86 MiB |
| `mid` | 512 px | `512` | 171 MiB |
| `high` | 1024 px | `1024` | 249 MiB |

Each tier runs `android/generate_assets.sh` with a `TEXTURE_SIZE` override, then
`optimize_data.sh`, then packs. Three details in `pack_assets.sh` matter and are easy to
regress:

- **`--format=ustar`** keeps paths at 100 characters or fewer. pax extended headers for
  long paths roughly double the archive size and break the in-browser tar reader.
- **`--exclude='._*'`** strips AppleDouble sidecar files that macOS leaves in
  `stk-assets`, which similarly inflate the archive.
- **`size="$(cat chunk.* | wc -c)"`** rather than `du -k`. `du` reports allocated
  blocks, not bytes, inflating the declared size; the browser then allocates an
  oversized buffer and pako fails with `unknown compression method`.

---

## How It Works

### Page load

`index.html` loads the Emscripten loader and then installs a shim that intercepts the
first call to `run()`. Emscripten signals runtime readiness by starting `main()`, which
would otherwise race the loader's own initialisation. The shim suppresses the first call
and flips `globalThis.ready = true`:

```js
globalThis.ready = false;
var real_run = run;
run = () => {
  if (globalThis.ready) { real_run(); }
  globalThis.ready = true;
};
```

`script.js` polls `globalThis.ready` every 100 ms. Once it is set, `main()` runs:

1. `load_config()` — fetches `config.json`, which exposes `ws_enabled` and `ws_proxy`.
   With `ws_enabled: false` (the committed default) no WebSocket proxy is configured.
2. `load_idbfs()` — mounts `IDBFS` at `/home` and calls `syncfs(populate: true)` to
   hydrate `/home` from IndexedDB.
3. Enables the `▶ START GAME` button and clears the status text.

Assets are **not** downloaded at page load. This is deliberate: the player chooses a
quality tier and commits before spending bandwidth.

### Asset download and extraction

Clicking `START GAME` triggers `load_data()`:

1. **Version check.** If IndexedDB holds no `/version` key, or holds a value other than
   the current `data_version` (currently `21`), the entire database is deleted and
   rewritten. This is how an asset change invalidates a player's cache.
2. **Chunked download.** `download_chunks()` fetches `<url>.manifest?ts=<Date.now()>`
   with `cache: "no-store"`. The manifest's first line is the exact byte total; the
   remaining lines are chunk filenames. Each chunk is fetched and written into a
   pre-allocated `Uint8Array`. Two integrity checks run: a chunk must not overrun the
   declared size, and the final offset must equal the declared size exactly.
3. **Decompression.** `pako.inflate()` produces the raw tar. The compressed buffer is
   dropped before extraction to release ~90 MB.
4. **Caching.** The decompressed tar is written back to IndexedDB as 20 MB chunks under
   the URL as key, with a `{size, chunk_count}` header record.
5. **Extraction.** A hand-written `untar()` walks the 512-byte ustar headers, honouring
   `x`/`g` pax extended headers for the `path` record. Directories are created with
   `FS.mkdir`, files with `FS.writeFile`, into the Emscripten `MEMFS` at `/data`.

The hand-written reader replaced `js-untar`, which was unusably slow on these archives
and mis-handled pax headers.

### Launch

After extraction, `install_runtime_error_hooks()` wraps `Module.printErr`, assigns
`Module.onAbort` and registers a `window.onerror` listener. This exists because a
successful `run()` does not mean a successful launch — an abort surfaces a frame later.
Without the hooks the failure mode is an unexplained black canvas. The last 20 runtime
stderr lines are retained and the last 10 are rendered into the overlay on abort.

`run()` is then called, the overlay is hidden, and `sync_idbfs()` persists saves.

### Cache invalidation

Three independent layers, because each failure mode is silent:

| Layer | Mechanism | Prevents |
| --- | --- | --- |
| Manifest | `?t=<Date.now()>` + `cache: "no-store"` | A CDN-cached manifest from a previous `pack_assets` run declaring the wrong byte count |
| WASM binary | `Module.locateFile` appends `?v=<data_version>` | A stale `supertuxkart.wasm` paired with a new `stk20.js`, which aborts with `No EM_ASM constant found at address ...` |
| IndexedDB | `/version` key vs. `data_version` | Reusing a previous pack's decompressed assets |

---

## Solana Program

### Workspace layout

```
sol-rally-program/
├── Anchor.toml                    # localnet cluster, program ID, test script
├── Cargo.toml                     # workspace: members = ["programs/*"]
├── rust-toolchain.toml            # Rust 1.89.0 + rustfmt + clippy
├── package.json                   # @coral-xyz/anchor 0.32.1, mocha/ts-mocha, prettier
├── tsconfig.json
├── migrations/deploy.ts           # empty — no migration logic yet
├── app/                           # empty — Anchor client app not scaffolded
├── tests/sol-rally-program.ts     # scaffold test, targets the other program (see below)
└── programs/
    ├── sol_rally/                 # ← the real program
    │   ├── Cargo.toml
    │   └── lib.rs                 # GrandPrix account + 4 instructions
    └── sol-rally-program/         # ← leftover `anchor init` scaffold
        ├── Cargo.toml
        └── lib.rs                 # `initialize()` that prints a greeting
```

**Two programs exist.** `programs/sol_rally/lib.rs` contains the Grand Prix
implementation and is the one described below. `programs/sol-rally-program/src/lib.rs`
is an unmodified `anchor init` scaffold. Both are workspace members, so both compile.
`Anchor.toml` declares the program key as `sol_rally_program` and
`tests/sol-rally-program.ts` exercises `anchor.workspace.solRallyProgram` — meaning the
committed test targets the **scaffold**, not the real program. The scaffold should be
deleted.

### The `GrandPrix` account

```rust
#[account]
#[derive(InitSpace)]
pub struct GrandPrix {
    pub authority: Pubkey,   // organiser; PDA seed owner
    pub bump: u8,            // canonical bump for the PDA
    pub entry_fee: u64,      // lamports charged per entry, set at init
    pub total_fees: u64,     // lamports currently held
    pub total_entried: u64,  // entries recorded so far
}
```

Sized via `InitSpace` and allocated as `8 + GrandPrix::INIT_SPACE`.

### PDA derivation

```rust
seeds = [b"gp", authority.key().as_ref()]
```

Every instruction re-derives the account from the stored `authority` with
`bump = gp.bump`, so only the derived address is accepted. A player cannot be pointed at
a look-alike Grand Prix account.

### Instructions

#### `init_gp(ctx, entry_fee)`

Creates the Grand Prix account. `authority` signs and pays rent. `entry_fee` is
arbitrary at creation and **never validated** — see
[Security Considerations](#security-considerations).

#### `enter_gp(ctx)`

Collects the entry fee. `player` signs.

```rust
let player_lamports = ctx.accounts.player.lamports.borrow();
let gp_lamports     = ctx.accounts.gp.lamports.borrow_mut();

if *player_lamports < gp.entry_fee {
    return Err(GrandPrixError::InsufficientEntryFee.into());
}

*player_lamports -= gp.entry_fee;
*gp_lamports     += gp.entry_fee;
gp.total_fees   += gp.entry_fee;
gp.total_entried += 1;
```

Lamports are moved by direct mutation rather than a `system_program::transfer` CPI. It
functions because `player` is a signer, but it is non-idiomatic and the declared
`system_program` account is unused. It also records no per-player identity: after
`enter_gp` returns, nothing links that player to their entry.

#### `resolve_gp(ctx)`

Splits the pool 50/30/20 and closes the account. As written, all three transfers go to
`authority`, which is the same account in every one:

```rust
let first  = total_fees * 50 / 100;
let second = total_fees * 30 / 100;
let third  = total_fees * 20 / 100;
// three transfer() CPIs, all with to: ctx.accounts.authority
```

The percentage split is computed and then discarded. This is the single most important
thing to fix; see [Prize Pool and Settlement](#prize-pool-and-settlement).

#### `cancel_gp(ctx)`

Closes the account via `close = authority`, returning rent and any uncollected lamports
to the organiser. Function body is empty — the `close` constraint does all the work.

### Error codes

```rust
#[error_code]
pub enum GrandPrixError {
    #[msg("Insufficient entry fee")]
    InsufficientEntryFee,
}
```

One error. `init_gp` adds no validation of `entry_fee`.

---

## MagicBlock Integration

**Not implemented. There is no MagicBlock code in this repository.**

Search the tree and you will find the string "MagicBlock" in exactly one place that
matters: the `SOLANA × MAGICBLOCK` badge and the tagline on the landing page. There is
no MagicBlock SDK dependency in any `package.json`, no `Cargo.toml` entry, no gRPC
client, and no ephemeral-account or TEE code. The Anchor program contains no
MagicBlock-related instruction.

### What MagicBlock would provide

[MagicBlock](https://magicblock.gg/) runs game state in ephemeral accounts so that
updates land at Solana block rate rather than at a game's server tick rate, with the
state verifiable. For a kart racer the motivation is specific: authoritative per-player
position, lap and item state need to update continuously, and doing that through
consensus-layer accounts costs a validator write per frame.

### The intended design

```
    ┌──────────────────────────────────────────────────────────────────┐
    │ ┌────────────────────────────┐    ┌────────────────────────────┐ │
    │ │                            │    │                            │ │
    │ │ Solana RPC                 │    │ Game client                │ │
    │ │ (read accounts)            │    │ (subscribe)                │ │
    │ │    ↓                       │    │    ↓                       │ │
    │ └────────────────────────────┘    └────────────────────────────┘ │
    └──────────────────────────────────────────────────────────────────┘

                                      │
                                      ▼
      ┌──────────────────────────────────────────────────────────────────────────┐
      │                                                                          │
      │ MAGICBLOCK                                                               │
      │                                                                          │
      │ Ephemeral / TEE-backed accounts                                          │
      │                                                                          │
      │ Sub-second read + write for                                              │
      │ transient race state:                                                    │
      │                                                                          │
      │   • kart position                                                        │
      │   • lap count                                                            │
      │   • item pickups                                                         │
      │                                                                          │
      │ NOT IMPLEMENTED                                                          │
      └──────────────────────────────────────────────────────────────────────────┘
                                      │
                                      │ permissioned writes
                                      │
                                      ▼
      ┌──────────────────────────────────────────────────────────────────────────┐
      │                                                                          │
      │ sol_rally_program                                                        │
      │                                                                          │
      │ Durable outcomes only:                                                   │
      │                                                                          │
      │   • entry fee collection                                                 │
      │   • finish order                                                         │
      │   • prize split                                                          │
      │                                                                          │
      │ Runs on Solana                                                           │
      └──────────────────────────────────────────────────────────────────────────┘
```

Transient state (position, lap count, item pickup) would live in MagicBlock ephemeral
accounts. Durable outcomes (entry, finish order, payout) would settle through the Anchor
program on Solana.

### Work required before this can be claimed

- Add the MagicBlock dependency (Rust or TS) and authenticate the game client.
- Decide the authority model: which key signs state updates, and how a player is
  prevented from writing another player's state.
- Define the state schema and the mapping from transient state to settled results.
- Handle disconnection: a MagicBlock outage mid-race must not lose a paid entry.
- Remove or qualify the landing page badge until this is real.

---

## Prize Pool and Settlement

### The intended model

An organiser creates a Grand Prix with an entry fee. Each entry adds the fee to a
locked pool. At resolution the pool is split across finishing positions. The account is
closed, returning rent to the organiser.

```
   organiser                    player A            player B            player C
      │                            │                   │                   │
      │ init_gp(entry_fee=1 SOL)   │                   │                   │
      ├───────────────────────────▶│                   │                   │
      │   PDA seeds=[b"gp",org]    │                   │                   │
      │   total_entried = 0        │                   │                   │
      │                            │                   │                   │
      │        enter_gp()          │ enter_gp()        │ enter_gp()        │
      ├────────────────────────────┼───────────────────┼──────────────────▶│
      │   total_entried = 1        │ total_entried = 2 │ ... = 3           │
      │   total_fees = 3 SOL       │                   │                   │
      │                            │                   │                   │
      │                     resolve_gp()  ── 50/30/20 split ──▶
      │   account closed, rent returned
```

### What the code actually does

`resolve_gp` computes a 50/30/20 split and then sends all three amounts to
`authority`:

```rust
let first  = total_fees * 50 / 100;
let second = total_fees * 30 / 100;
let third  = total_fees * 20 / 100;

anchor_lang::system_program::transfer(
    CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: ctx.accounts.gp.to_account_info(),
            to:   ctx.accounts.authority.to_account_info(),   // ← first
        },
    ),
    first,
)?;
// ... identical, `second`, then `third`, all to authority
```

Net effect: the organiser receives the entire pool, and the percentage split is dead
code. Whatever fractional dust the integer division leaves is swept to `authority` in a
fourth transfer.

### Additional gaps

- **No payout accounts.** `ResolveGp` has no winner accounts in its `Accounts` struct,
  so there is nowhere to send prize money even if the destinations were correct.
- **No per-entry records.** `enter_gp` increments a counter but records no player. At
  resolution the program has no way to know who finished where. A `PlayerEntry` PDA per
  entrant is required.
- **No timing window.** Nothing records when a Grand Prix was created or when entries
  close. A player can enter after the organiser has already resolved, as long as the
  account is still open.
- **No reentrancy or double-resolve guard beyond `close`.** The `close = authority`
  constraint means a second `resolve_gp` fails on account-not-found, which is
  acceptable, but this is incidental rather than an explicit state machine.
- **No `init_gp` validation.** `entry_fee` can be `0` (a free Grand Prix) or absurdly
  large. A zero fee makes `enter_gp` a no-op that still increments `total_entried`.
- **Overflow.** `total_fees += gp.entry_fee` is a plain `u64` add in a release profile
  compiled with `overflow-checks = true`, so it panics rather than wrapping. Fine, but
  the panic is not translated into a program error.

### Before this handles real value

1. Replace the scaffold `sol-rally-program` crate with nothing — delete it.
2. Add a `PlayerEntry` account (PDA seeded by Grand Prix + player) recording position
   and paid amount.
3. Add explicit winner accounts to `ResolveGp`.
4. Add a race-state enum (`Open`, `Entering`, `Finished`, `Resolved`) with
   `enter_gp` gated on `Open`.
5. Transfer prize amounts to the recorded winner accounts.
6. Validate `entry_fee > 0` and add a rent-safety check against zero-fee entry.
7. Add tests. The current test file exercises the scaffold program's `initialize`.

---

## Security Considerations

### Cross-origin isolation

The engine is built with Emscripten's pthread support, which requires the page to be
cross-origin isolated. Both `wasm/run_server.py` and `wasm/web/_headers` set:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

`wasm/vercel.json` sets these plus `Cross-Origin-Resource-Policy: same-origin` for all
paths. Removing any one of these headers without rebuilding without pthreads will break
`SharedArrayBuffer` and the game will fail to start. Note that the COEP header means any
third-party resource the page loads must serve `Cross-Origin-Resource-Policy: cross-origin`
or it will be blocked — this constrains which CDNs can be used.

### Asset integrity

The chunked download verifies both the declared total size and per-chunk bounds, which
turns a truncated or stale CDN response into a clear error rather than a corrupt
filesystem. The manifest is fetched with `no-store` and a cache-busting parameter
specifically because a stale manifest from a previous `pack_assets` run declares the
wrong byte count.

There is **no cryptographic verification** of the asset archive. A player cannot
confirm the tar they downloaded matches the one the maintainer built. If asset
integrity matters, add a SHA-256 of the archive to the manifest and verify after
assembly. The current checks catch truncation and cache staleness, not substitution.

### Program security

**PDA authority is sound.** The `[b"gp", authority]` seed plus stored bump is verified on
every instruction, so a substituted Grand Prix account cannot be used to steal entry
fees.

**Settlement is not safe for real value.** As detailed above, `resolve_gp` pays the
organiser, not the winners. There are no winner accounts and no per-entry records.
**Do not accept real value through this program in its current form.**

**`enter_gp` bypasses the system program.** Direct lamport mutation works because the
player is a signer, but it discards the CPI path and leaves the declared
`system_program` account unused. A `system_program::transfer` CPI is the conventional
and better-audited construction.

**No entry bookkeeping.** Without per-player records, a player cannot prove they entered,
and the organiser cannot pay them.

**`entry_fee` is unvalidated.** A zero fee makes entry free while still incrementing
`total_entried`, which would corrupt any future per-player accounting.

**The scaffold program must not be deployed.** `programs/sol-rally-program` declares the
same `declare_id!` as the real program. Both are workspace members. Shipping both in one
binary is confusing at best.

### Frontend

**No wallet connection means no transaction handling in the browser today.** There is no
key material, no signature request, and no RPC call in `wasm/web/`. When wallet
integration lands, the standard constraints apply: never handle a private key in page
code, always let the wallet own signing, and verify the program ID client-side before
signing any transaction.

**Runtime error disclosure.** The error hooks render the last 10 lines of WASM stderr
into the page. This is deliberate and useful for debugging, but it will surface internal
paths and assertion text in production. Consider gating it behind a debug flag before a
public release.

### Deployment hygiene

`.vercelignore` excludes build trees, the engine source, CI config and the Anchor
workspace from the Vercel upload. The uploaded artifact is the static site only. The
excluded list includes `solrally-program` — note the actual directory is
`sol-rally-program`, so that entry does not match and the directory (including
`node_modules` and `target/`) is currently **not** excluded. Worth fixing before the
next deploy; it is only harmless because Vercel ignores non-`.vercelignore`-relevant
files by size and the upload is scoped to the `deploy/` staging copy.

---

## Getting Started

Two independent halves. You can play the game without touching the Solana program, and
you can work on the program without building the engine.

### Prerequisites

**For the game (playing locally against the existing build):**

- Python 3 — for `wasm/run_server.py`. No packages required.
- A browser with WebGL and cross-origin isolation support: Chrome, Edge, Firefox or
  Safari from a recent version.
- The prebuilt artifacts in `wasm/web/game/` (~616 MB). These are committed, so a clone
  is enough to play.

**For rebuilding the engine:**

- Emscripten SDK at `~/emsdk`, installed via `wasm/get_emsdk.sh`.
- A working `embuilder` for `sdl2`, `sdl2_ttf`, `sdl2_image`, `sdl2_image_jpg`,
  `sdl2_image_png`.
- CMake 3.5 or newer.
- Native build tools (Xcode command line tools on macOS).
- On macOS: `brew install` the dependencies below.

**For asset packing:**

```sh
brew install imagemagick vorbis-tools pngquant advancecomp libjpeg-turbo optipng
```

**For the Solana program:**

- Rust 1.89.0 — pinned by `sol-rally-program/rust-toolchain.toml`, installed
  automatically by `rustup`.
- Anchor CLI 0.32.1.
- Solana CLI and `solana-test-validator`.
- Yarn 1.x.

### Installation

Clone the repository and start the dev server:

```sh
git clone git@github.com:brunonzwili-lgtm/sol-rally-v3.git
cd sol-rally-v3
python3 wasm/run_server.py
```

Then open <http://localhost:8000>.

To rebuild the engine from source, run the full chain:

```sh
# 1. Install the Emscripten SDK to ~/emsdk
wasm/get_emsdk.sh

# 2. Cross-compile native dependencies for wasm32
wasm/build_deps.sh

# 3. Compile the engine
wasm/build.sh Release

# 4. Generate and pack the asset archives
wasm/pack_assets.sh ./stk-assets

# 5. Serve
python3 wasm/run_server.py
```

For an incremental rebuild after editing C++ — one `.cpp` change still relinks a 15 MB
binary — use the detached helper, which survives shell timeouts:

```sh
python3 wasm/build_detached.py    # logs to /tmp/stk_build.log
```

`wasm/rebuild.sh` is a convenience wrapper that assumes fixed absolute paths; prefer
`wasm/build.sh`, which resolves paths relative to itself.

For the Solana program:

```sh
cd sol-rally-program
yarn install
anchor build
solana-test-validator          # separate terminal
anchor deploy
anchor test
```

### Environment Variables

Sol Rally has no `.env` files and no build-time environment variable interpolation.
Configuration is runtime JSON, read by `load_config()` in `script.js` from
`wasm/web/config.json`:

```json
{
  "ws_enabled": false,
  "ws_proxy": "wss://example.com/"
}
```

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `ws_enabled` | boolean | `false` | Enable the WebSocket proxy for online multiplayer |
| `ws_proxy` | string | `wss://example.com/` | Upstream WebSocket URL the proxy forwards to |

`ws_enabled: false` keeps multiplayer disabled. The upstream port warns that some
options — anything touching online multiplayer — can hang the game, so enabling this
without a working proxy is not recommended.

Build-time environment:

| Variable | Used by | Meaning |
| --- | --- | --- |
| `EMSDK_DIR` | `wasm/build.sh` | Path to the Emscripten SDK. Defaults to `$HOME/emsdk` |
| `BUILD_TYPE` | `wasm/build.sh` argv | `Release` or `Debug`. Defaults to `Release` |
| `ASSETS_DIR` | `wasm/pack_assets.sh` argv | Path to the `stk-assets` source tree |
| `ANCHOR_PROGRAM_ID` | Anchor CLI | Overrides the program ID from `Anchor.toml` |

Deployment variables are held in `wasm/vercel.json` and the local `vercel` CLI's
project linkage (`.vercel/`), which is gitignored.

### Running Locally

```sh
# Start the server (defaults to port 8000)
python3 wasm/run_server.py 8000

# Or, detached so it survives shell timeouts and Ctrl-C
python3 wasm/serve.py
```

Open <http://localhost:8000>, pick a texture quality, and press `▶ START GAME`.

What you will see on a first run:

```
Loading game code...
[START GAME becomes enabled]
Downloading manifest for data_low.tar.gz...
Downloading data_low.tar.gz... (chunk 1/5, 0/85MiB)
...
Decompressing data_low.tar.gz...
Saving data_low.tar.gz to the cache...
Extracting data_low.tar.gz...
Extracted 3695 files and 281 directories
Launching game...
```

A second run skips straight past download and extraction, reading from IndexedDB.

If the game fails to start, the loading overlay reappears with the error and the last
runtime stderr lines rather than leaving a black canvas. The full log is on the browser
console.

### Building for Production

The production artifact is a static site. There is no bundler, no transpiler and no
server-side render — `wasm/web/` is the deploy root.

```sh
# 1. Build the engine
wasm/build.sh Release

# 2. Pack assets (skips tiers whose .manifest already exists)
wasm/pack_assets.sh ./stk-assets

# 3. Stage the site
mkdir -p deploy
rsync -a wasm/web/ deploy/

# 4. Deploy
python3 deploy_vercel.py
```

`wasm/vercel.json` sets `buildCommand` to the empty string, because there is nothing to
build at deploy time. It also defines the cache policy:

| Path pattern | `Cache-Control` | Reason |
| --- | --- | --- |
| `/game/supertuxkart.{js,wasm}` | `no-store, no-cache, must-revalidate` | A stale binary paired with a new loader aborts |
| `/game/data_*.tar.gz.manifest` | `no-store, no-cache, must-revalidate` | Manifest must match the current chunks |
| `/game/data_*` | `public, max-age=3600, must-revalidate` | Chunks are content-stable within a pack |
| `/*.{html,json,js}` | `no-store, no-cache, must-revalidate` | Shell and loader must never be stale |

Both deploy scripts (`deploy_vercel.py` at the root and `wasm/deploy_vercel.py`)
double-fork and `setsid` before invoking `vercel deploy --prod --yes --archive=tgz`,
because the ~390 MB upload outlasts any shell timeout. Logs go to
`/tmp/vercel_deploy.log`.

Note: `wasm/web/index.html` loads `/game/stk20.js`, while `wasm/build.sh` produces
`supertuxkart.js` and `wasm/patch_js.py` patches `supertuxkart.js`. `stk20.js` and
`stk20.wasm` are manually-placed versioned copies. This works, but the relationship
between the build output and what the page loads is implicit and undocumented in the
scripts. See [Known Limitations](#known-limitations).

### Testing

**The Solana program.** `anchor test` runs `ts-mocha` against `sol-rally-program.ts`:

```sh
cd sol-rally-program
anchor test
```

The committed test calls `initialize()` on `solRallyProgram` — the **scaffold** crate,
not the program that contains the Grand Prix logic. The real program has **no test
coverage**. This is the highest-value gap to close.

**Linting.** Prettier for the TypeScript workspace:

```sh
cd sol-rally-program
yarn lint         # check
yarn lint:fix     # auto-fix
```

**The engine.** The upstream C++ test suite runs through the native CMake build, not
the WASM build:

```sh
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build
ctest --test-dir build
```

**The web layer.** There is no test runner. `script.js` has been hardened against a
specific set of real failures — stale manifests, overrun chunks, truncated downloads,
mismatched `data_version`, silent aborts — and each of those is guarded by an explicit
check, but none is covered by an automated test.

### Deployment

**Vercel (production, `https://solrally.vercel.app/`).** From the `deploy/` staging
directory:

```sh
python3 deploy_vercel.py        # detached; logs to /tmp/vercel_deploy.log
```

`.vercelignore` at the repository root excludes build trees, the engine source, CI
config and the Solana workspace from the upload. `deploy/.gitignore` excludes
`.vercel/`, keeping the project linkage out of version control.

**GitHub Pages.** `.github/workflows/pages.yml` triggers on pushes to `main`, uploads
`wasm/web` as a Pages artifact and deploys. Note that `wasm/web` is ~616 MB, which
exceeds GitHub's practical Pages artifact limits — expect this workflow to fail on a
full asset set. See [Known Limitations](#known-limitations).

**On-chain.** The Anchor program is configured for `localnet` only. `Anchor.toml`
declares `programs.localnet`; there is no devnet or mainnet cluster entry and no
committed deployment script beyond the empty `migrations/deploy.ts`.

```sh
solana-keygen new --outdir ~/.config/solana
solana config set --url http://localhost:8899
cd sol-rally-program && anchor deploy
```

---

## Project Structure

```
sol-rally-v3/
├── src/                          Sol Rally C++ engine source
│   ├── graphics/irr_driver.cpp   ← window class/caption set to "Sol Rally"
│   ├── states_screens/           GUI screens (.stkgui in data/gui/screens)
│   └── ...                       ~30 further source directories
├── data/                         GUI definitions, tracks, challenges, translations
│   └── gui/screens/user_screen.stkgui
├── lib/                          Vendored dependencies (astc-encoder, shaderc, ...)
├── cmake/                        Toolchain files (cctools, llvm-mingw, ...)
├── android/                      Android build + generate_assets.sh
├── windows/ switch/              Platform build trees
├── tools/                        Packaging, installers, asset tooling
├── stk-assets/                   Source asset tree (SVN checkout)
│   ├── karts/ tracks/ textures/ models/ music/ sfx/
│   └── .svn/                     ← assets are versioned under SVN, not git
├── wasm/                         ★ The Sol Rally web layer
│   ├── build.sh                  Engine build
│   ├── build_deps.sh             Cross-compile deps for wasm32
│   ├── build_detached.py         Detached incremental rebuild
│   ├── rebuild.sh                Path-hardcoded rebuild wrapper
│   ├── pack_assets.sh            Asset archive generation
│   ├── patch_js.py + fragments/  Post-process the Emscripten loader
│   ├── serve.py                  Detached dev server
│   ├── run_server.py             Threaded dev server, COOP/COEP headers
│   ├── deploy_vercel.py          Detached Vercel deploy
│   ├── vercel.json               Cache + isolation headers
│   ├── get_emsdk.sh              Emscripten installer
│   ├── build/ prefix/ emsdk/     Cross-compiled deps (gitignored)
│   └── web/                      ← WEB SERVER ROOT / deploy artifact
│       ├── index.html            Landing page + loading overlay
│       ├── script.js             Loader: assets, cache, IndexedDB, error hooks
│       ├── config.json           Runtime config (ws_enabled, ws_proxy)
│       ├── _headers              COOP/COEP for Netlify-style hosts
│       ├── solrally_64.png       Favicon / brand mark
│       ├── images/karts/         4 kart thumbnails (landing page only)
│       ├── images/tracks/        Track hero image
│       └── game/                 ~616 MB
│           ├── stk20.js          ← loader actually loaded by index.html
│           ├── stk20.wasm        (unused — see Known Limitations)
│           ├── supertuxkart.wasm ← 15.3 MB, loaded with ?v=<data_version>
│           ├── supertuxkart.js
│           ├── data_low.tar.gz.{manifest,00..04}
│           └── data_low/         3,695 extracted files (tracked)
├── sol-rally-program/            ★ Anchor workspace
│   ├── Anchor.toml  Cargo.toml  rust-toolchain.toml
│   └── programs/
│       ├── sol_rally/lib.rs      Grand Prix implementation
│       └── sol-rally-program/    Scaffold to delete
├── deploy/                       Vercel staging copy (~117 MB)
│   ├── index.html  script.js  config.json  vercel.json
│   └── game/  images/
├── deploy_vercel.py              Detached Vercel deploy (root)
├── .vercelignore                 Upload exclusions
├── .github/workflows/            pages.yml + upstream STK CI
│   ├── pages.yml                 ← the only workflow targeting `main`
│   └── linux.yml apple.yml windows.yml switch.yml
│                                   ↑ these target `master` and never run here
├── COPYING                       GPLv3 (code) + mixed asset licences
├── INSTALL.md  NETWORKING.md  CHANGELOG.md
└── README.md
```

### Build system files

| Path | Role |
| --- | --- |
| `CMakeLists.txt` | Root CMake project (38 KB), shared by all platforms |
| `sources.cmake` | Source file list |
| `wasm/build_deps.sh` | Builds SDL2, OpenAL, zlib, libpng, libjpeg, HarfBuzz for `wasm32` |
| `wasm/build.sh` | `emcmake cmake` + `make`, then copies and patches output |
| `wasm/patch_js.py` | Applies `wasm/fragments/*` to the generated loader |
| `wasm/pack_assets.sh` | Generates `data_{low,mid,high}` archives |
| `android/generate_assets.sh` | Resizes textures per quality tier |

---

## Current Status

### Working end to end

- Sol Rally runs in the browser via Emscripten at `https://solrally.vercel.app/`.
- Texture quality selection across low/mid/high tiers.
- Chunked asset download with live progress and integrity checks.
- IndexedDB asset caching across visits, invalidated by `data_version`.
- Persistent saves via `IDBFS`.
- Runtime error surfacing instead of a silent black canvas.
- Static deployment to Vercel; the site is live.
- Landing page renders.

### Compiles but incomplete

- The Anchor program builds and its four instructions are callable.
- `init_gp` and `cancel_gp` behave as intended.
- `enter_gp` collects fees, but records no per-player identity.
- `resolve_gp` pays the organiser, not winners. See
  [Prize Pool and Settlement](#prize-pool-and-settlement).

### Not implemented

| Feature | State |
| --- | --- |
| Wallet connection | Absent. No `@solana/*` dependency in `wasm/web/` |
| Program called from the game | Absent. No IDL client |
| MagicBlock integration | Absent. Branding only |
| Solana-themed karts | Absent. Landing page shows static PNGs; game ships `tux`, `beastie`, `emule`, `kiki` |
| Solana-themed tracks | Absent. Game ships the stock Sol Rally set |
| Mainnet Race / Slot Sprint / MEV Battle modes | Absent |
| On-chain leaderboards | Absent |
| Multiplayer | Absent. `ws_enabled` is `false` |

### Code health

- `programs/sol-rally-program/` is an `anchor init` scaffold that should be deleted. It
  duplicates the program ID and is the target of the only committed test.
- No test covers `sol_rally`. The test suite as written proves the scaffold works.
- `wasm/web/game/stk20.wasm` is dead weight — `stk20.js` loads
  `supertuxkart.wasm?v=<data_version>`. `stk20.wasm` is 15,287,498 bytes and never
  fetched.
- `.vercelignore` lists `solrally-program`; the directory is `sol-rally-program`.
- The landing page advertises four features that do not exist.
- `wasm/rebuild.sh`, `wasm/serve.py` and both `deploy_vercel.py` scripts hardcode
  `/Users/h/Documents/stk-code-wasm`. They will not work on another machine or CI.

---

## Roadmap

Ordered by what unblocks the most downstream work.

**1. Make the program safe for value.**
Delete the scaffold crate. Add `PlayerEntry` PDAs recording entrant and position. Add
winner accounts to `ResolveGp` and pay them. Add a race-state enum so entries cannot be
accepted after resolution. Validate `entry_fee`. This is the prerequisite for every
other on-chain feature — nothing should accept real value before it.

**2. Wire the frontend to the program.**
Add `@solana/web3.js` and a wallet adapter as ES modules from a CDN, matching the
existing no-bundler approach. Load the IDL. Gate Grand Prix entry behind a wallet.
Handle rejection, wrong network, and insufficient balance as first-class states. Add a
client-side program ID check before any signature request.

**3. Reconcile the landing page with reality.**
Either build the Solana karts and tracks, or mark the four feature cards with the same
`SOON` badge Grand Prix already carries. The current page is the project's most visible
inaccuracy.

**4. Replace the real-world karts and tracks.**
Add `validator`, `bonk`, `jupiter` and `sol` to `stk-assets/karts/`, and `sol-dunes`,
`orca-lagoon`, `marinade-trench`, `pyth-peak` and `tensor-town` to `stk-assets/tracks/`.
Repack all three tiers.

**5. Evaluate MagicBlock.**
Decide whether real-time state belongs in ephemeral accounts. Prototype with a single
variable — one kart's position — before committing to a schema. Measure whether Solana
block rate is actually adequate; if not, say so and change the design.

**6. On-chain leaderboard.**
Natural next step once settlement is correct, and cheap once results are recorded.

**7. Cleanup.**
Hardcoded absolute paths in the helper scripts. Remove `stk20.wasm`. Fix the
`.vercelignore` typo. Get GitHub Pages working or delete `pages.yml` — it currently
cannot succeed with a 616 MB artifact.

---

## Known Limitations

**Engine**
- OpenGL ES 2 only. GLES 3 does not work on this port; the legacy renderer is still in
  use, which is the main reason performance is not better.
- Assets are downloaded in full before the game starts. There is no lazy loading during
  gameplay, so first load takes as long as the archive takes.
- Multiplayer is not implemented. The upstream port warns that any online-multiplayer
  option can hang the game.
- `emscripten_sleep` is stubbed, so any code path that depends on real thread suspension
  will not work.
- No WebGPU path.

**Assets**
- First visit downloads 86–249 MiB depending on quality tier.
- `stk-assets` is an SVN checkout carrying its own `.svn/` directory. Git does not track
  asset history.
- Three quality tiers must be regenerated whenever assets change; `pack_assets.sh`
  skips any tier whose `.manifest` exists, so stale manifests persist unless deleted.

**Build and deploy**
- `wasm/rebuild.sh`, `wasm/serve.py`, `wasm/build_detached.py` and both
  `deploy_vercel.py` scripts hardcode `/Users/h/Documents/stk-code-wasm`.
- `wasm/build.sh` requires a pre-existing `~/emsdk`; it does not install one.
- `wasm/web/game` is ~616 MB and committed to git. The repository grows with every
  asset repack.
- `.github/workflows/pages.yml` uploads `wasm/web` to GitHub Pages. At 616 MB this
  exceeds what Pages can serve; the workflow will fail.
- The four upstream STK CI workflows (`linux.yml`, `apple.yml`, `windows.yml`,
  `switch.yml`) trigger on `master`. This repository's default branch is `main`, so
  **none of them run**.
- `index.html` loads `stk20.js`, but the build produces `supertuxkart.js`. The
  relationship is maintained by hand and is not expressed in any script.
- `wasm/web/game/stk20.wasm` (15.3 MB) is committed and never loaded.

**Solana**
- Localnet only. No devnet or mainnet configuration is committed.
- `resolve_gp` pays the organiser, not the winners.
- No per-entry records, so results cannot be attributed to players.
- No entry window; entries can be accepted after resolution is intended.
- `entry_fee` is unvalidated.
- The only committed test targets the scaffold program, not the real one.
- No wallet client in the browser, so the program is unreachable from the game.

**Cross-cutting**
- The landing page advertises features that are not implemented.
- No end-to-end test covering asset download, extraction or cache invalidation.
- Runtime error disclosure shows internal stderr in production.

---

## Contributing

Contributions are welcome, particularly on the gaps listed in
[Current Status](#current-status).

**Before you start**

- Open an issue before large changes. The engine is based on an open-source kart racing
  foundation; diverging from the original structure increases maintenance costs, so
  engine changes need discussion.
- If you are changing engine code, note whether the change could benefit the broader
  kart racing community.

**Ground rules**

- **Do not accept real value on-chain until the settlement fix in
  [Prize Pool and Settlement](#prize-pool-and-settlement) lands.** The current
  `resolve_gp` pays the organiser.
- **Test the program.** `sol_rally` currently has no coverage. Adding a test for any
  change there is expected, not optional.
- **Bump `data_version` in `wasm/web/script.js`** whenever you change assets or the
  engine binary. Without it, players keep a stale IndexedDB cache and never see your
  change.
- **Do not hand-edit `wasm/web/game/data_low/`.** It is generated output. Change
  `stk-assets` and re-run `wasm/pack_assets.sh`.
- **Regenerate all three quality tiers** when assets change, and delete the existing
  `.manifest` files first or `pack_assets.sh` will skip them.
- **Never remove `Cross-Origin-Opener-Policy` or `Cross-Origin-Embedder-Policy`**
  without also rebuilding the engine without pthreads. The game will not start.

**Style**

- C++ follows the existing codebase conventions. Do not reformat unrelated lines.
- Rust: `cargo fmt` and `cargo clippy`. The release profile uses `lto = "fat"`,
  `codegen-units = 1` and `overflow-checks = true` — keep them.
- TypeScript: `yarn lint` in `sol-rally-program`.
- No new runtime dependencies in `wasm/web/` without discussion. The page has no
  bundler; everything is a CDN ES module, and each one must be COEP-compatible.

**Reporting bugs**

Include the browser and version, the URL, the texture quality selected, whether it is a
first load or a cached load, and the console output. If the game failed to start, the
loading overlay will show the last runtime stderr lines — copy those in.

---

## Acknowledgements

**The original kart racing engine** — the game engine, all gameplay code, and every kart,
track, model, texture, sound and music file. Sol Rally is built on this foundation and
would not be possible without it.

Licensed under the GNU General Public License; the asset files are released under a
mixture of licences. See [`COPYING`](./COPYING) for the full terms and
`stk-assets/licenses.txt` for per-asset attribution.

**Emscripten** — the C/C++ to WebAssembly toolchain that makes a native engine
browser-runnable, including the `IDBFS` filesystem and `SharedArrayBuffer`-based pthreads.

**Irrlicht** — the graphics engine underlying Sol Rally.

**SDL2**, **OpenAL**, **zlib**, **libpng**, **libjpeg-turbo**, **HarfBuzz**, **Ogg**,
**Vorbis** — the native libraries cross-compiled for `wasm32`.

**pako** — zlib decompression in the browser.

**Solana**, **Anchor**, **Solana Program Library** — the settlement layer.

**MagicBlock** — the intended real-time engine for on-chain game state. Not yet
integrated.

**The Solana and WebAssembly developer communities** — for the accumulated knowledge on
Emscripten threading, cross-origin isolation, and large-asset delivery that this project
depends on.

---

## License

**The Sol Rally source code in this repository is free software**, licensed under the
**GNU General Public License v3.0 or later**. You may redistribute it and modify it
under the terms of that licence.

**The data files** — textures, models, sounds, music and other assets — are released
under a mixture of licences including, but not limited to, the GNU General Public
License. See [`COPYING`](./COPYING) for the authoritative text and
`stk-assets/licenses.txt` for the per-asset breakdown.

**The `sol-rally-program` workspace** (`sol-rally-program/**`) is the Sol Rally
contribution and is declared `ISC` in `sol-rally-program/package.json`. You may
reconcile this with the repository's GPLv3 terms as you prefer; if in doubt, GPLv3
applies to the repository as a whole.

Third-party dependencies retain their own licences. `stk-assets/check_licenses.php`
regenerates the asset attribution report.