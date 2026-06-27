# Nodecraft (Rust)

Native **Rust + Bevy** client, plus a **WASM build** deployed to GitHub Pages.

## Requirements

- Rust stable (`rustup default stable`)
- Linux deps for native builds:

```bash
sudo apt install libasound2-dev libudev-dev libxkbcommon-dev \
  libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev pkg-config
```

## Native desktop (best performance)

```bash
cd rust
cargo run --release
```

## Web / WASM (local)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd rust
env -u NO_COLOR trunk serve --no-default-features
```

Open http://127.0.0.1:8080

## GitHub Pages

Every push to `main` builds the Rust WASM demo via GitHub Actions and deploys it to:

**https://kurtisrogers.github.io/nodecraft/**

## Multiplayer server

```bash
cd rust
cargo run --release --bin nodecraft-server --features server --no-default-features
```

## Controls

| Key | Action |
|-----|--------|
| Click | Lock cursor |
| `Esc` | Release cursor |
| `WASD` | Move |
| `Space` | Jump |
| `Shift` | Sprint |
| `LMB` | Attack mob / break block |
| `RMB` | Place block |
| `E` | Inventory |
| `1-9` | Hotbar |

## Features

- Procedural terrain, caves, lava, villages (houses + wheat farms)
- Mobs: pigs, cows, sheep, chickens, zombies (night hostile)
- Day/night cycle
- Block inventory + hotbar
- Chunk meshing with Y-range optimization

## Project layout

```
src/lib.rs           # Game entry (native + WASM)
src/main.rs          # Native binary shim
src/world.rs         # Voxel chunks
src/mobs.rs          # Mob AI + combat
src/bin/server.rs    # WebSocket server
index.html           # Trunk WASM shell
Trunk.toml           # WASM build config
```
