# Nodecraft

**[Play the demo →](https://kurtisrogers.github.io/nodecraft/)**

A Minecraft-like voxel sandbox built with **Rust + Bevy** — native desktop and WASM browser builds.

![Nodecraft](https://img.shields.io/badge/rust-stable-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Quick Start

### Desktop (best performance)

```bash
cd rust
cargo run --release
```

### Browser (WASM)

Every push to `main` deploys the WASM build to GitHub Pages automatically.

Local dev:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd rust
env -u NO_COLOR trunk serve --no-default-features
```

Open http://127.0.0.1:8080

See [`rust/README.md`](rust/README.md) for system dependencies and the optional multiplayer server.

## Features

### World & Building
- Procedural terrain with islands, biomes, caves, and lava
- Villages with wooden houses, doors, and wheat farms
- Break blocks (LMB) and place blocks (RMB)
- 19 block types

### Mobs
- Pigs, cows, sheep, chickens — wander during the day
- Zombies — hostile at night

### Inventory
- 36-slot inventory with hotbar (press `E`, keys `1-9`)

## Controls

| Key | Action |
|-----|--------|
| Click | Lock cursor |
| `Esc` | Release cursor |
| `W A S D` | Move |
| `Space` | Jump |
| `Shift` | Sprint |
| `LMB` | Attack mob / break block |
| `RMB` | Place block |
| `E` | Inventory |
| `1-9` | Hotbar |

## Tech Stack

- **Rust + Bevy** — game client (native + WASM)
- **Trunk** — WASM build for GitHub Pages
- **Tokio + Axum** — optional WebSocket server (`rust/src/bin/server.rs`)

## Project layout

```
rust/
  src/lib.rs           # Game (native + WASM)
  src/main.rs          # Desktop entry
  src/world.rs         # Terrain, chunks, villages
  src/mobs.rs          # Mob AI
  src/bin/server.rs    # Multiplayer server
  index.html           # WASM shell
```

## License

MIT
