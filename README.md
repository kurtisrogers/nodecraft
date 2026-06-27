# Nodecraft

**[Play the browser demo →](https://kurtisrogers.github.io/nodecraft/)**

A Minecraft-like voxel sandbox built with **Rust + Bevy** — play in the browser (WASM) or run the native desktop build for best performance.

![Rust](https://img.shields.io/badge/rust-stable-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Quick start

### Browser (WASM)

The live demo at [kurtisrogers.github.io/nodecraft](https://kurtisrogers.github.io/nodecraft/) is built automatically on every push to `main`.

Local dev:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd rust
env -u NO_COLOR trunk serve --no-default-features
```

Open http://127.0.0.1:8080

### Desktop (native — best performance)

```bash
cd rust
cargo run --release
```

See [`rust/README.md`](rust/README.md) for Linux system dependencies and the optional multiplayer server.

## Features

- Procedural terrain with islands, biomes, caves, and lava
- Villages with wooden houses, doors, and wheat farms
- Mobs: pigs, cows, sheep, chickens, and zombies (hostile at night)
- Break and place blocks, 36-slot inventory with hotbar
- Day/night cycle

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

## Tech stack

- **Rust + Bevy** — game client (native + WASM)
- **Trunk** — WASM build for GitHub Pages
- **Tokio + Axum** — optional WebSocket server (`rust/src/bin/server.rs`)

## Project layout

```
rust/
  src/lib.rs           # Game entry (native + WASM)
  src/main.rs          # Desktop binary
  src/world.rs         # Terrain, chunks, villages
  src/mobs.rs          # Mob AI
  src/bin/server.rs    # Optional multiplayer server
  index.html           # WASM shell
  Trunk.toml           # WASM build config
```

## License

MIT
