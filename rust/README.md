# Nodecraft (Rust)

Native **Rust + Bevy** rebuild of Nodecraft — same procedural world, villages, caves, and block gameplay, compiled to native machine code for much better FPS than the browser client.

## Requirements

- Rust stable (1.85+ recommended) — `rustup default stable`
- Linux dev libraries for Bevy:

```bash
sudo apt install libasound2-dev libudev-dev libxkbcommon-dev \
  libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev pkg-config
```

## Run the game

```bash
cd rust
cargo run --release
```

First compile takes a few minutes; subsequent builds are incremental.

## Run the multiplayer server

```bash
cd rust
cargo run --release --bin nodecraft-server
```

WebSocket server on `http://localhost:3000/ws` — block-change overlay protocol compatible with the JS client's model (seed + delta sync).

## Controls

| Key | Action |
|-----|--------|
| Click | Lock cursor / start |
| `Esc` | Release cursor |
| `WASD` | Move |
| `Space` | Jump |
| `Shift` | Sprint |
| `LMB` | Break block |
| `RMB` | Place block |
| `E` | Inventory |
| `1-9` | Hotbar |

## What's ported

- Procedural terrain (islands, biomes, gentle hills, settlements)
- Underground caves + lava lakes
- Villages (wooden houses with doors, wheat farms)
- 19 block types with vertex-color meshing
- First-person movement + collision
- Inventory + hotbar
- Day/night sky cycle
- Chunk meshing with Y-range optimization + remesh budget

## Project layout

```
src/
  main.rs          # Bevy app entry
  world.rs         # Chunk storage + block API
  chunk_gen.rs     # Terrain, trees, caves
  noise.rs         # Procedural noise (ported from JS)
  structures.rs    # Village generation
  meshing.rs       # Chunk mesh builder + sync
  player.rs        # Movement, raycast, interaction
  inventory.rs     # 36-slot inventory
  weather.rs       # Day/night
  ui.rs            # egui HUD
  bin/server.rs    # Tokio/Axum WebSocket server
```

## Performance vs JS

The Rust client runs entirely natively (no WebGL browser overhead):

- Chunk meshing on the CPU without JS GC pauses
- Frustum culling + bounded remesh queue
- Native GLFW/winit window at full desktop frame rate

Typical improvement: **3-10× FPS** vs the GitHub Pages build on the same hardware.
