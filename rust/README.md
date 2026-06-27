# Nodecraft (Rust)

Rust + Bevy client with native desktop and WASM browser builds.

## Requirements

- Rust stable (`rustup default stable`)
- Linux deps for native builds:

```bash
sudo apt install libasound2-dev libudev-dev libxkbcommon-dev \
  libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev pkg-config
```

## Native desktop

```bash
cargo run --release
```

## Browser / WASM

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
env -u NO_COLOR trunk serve --no-default-features
```

Open http://127.0.0.1:8080

## GitHub Pages

Every push to `main` builds the WASM demo and deploys it to:

**https://kurtisrogers.github.io/nodecraft/**

## Multiplayer server (optional)

```bash
cargo run --release --bin nodecraft-server --features server --no-default-features
```

Listens on port 3000. The Rust client does not connect to it yet — server is kept for future networking work.

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
- Mobs: pigs, cows, sheep, chickens, zombies (hostile at night)
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
