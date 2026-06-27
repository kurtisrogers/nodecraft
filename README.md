# Nodecraft

**[Play the browser demo →](https://kurtisrogers.github.io/nodecraft/)** · **Native Rust build in [`rust/`](rust/)**

A Minecraft-like voxel sandbox — **Rust + Bevy** native client, plus the original **Node.js + Three.js** web client.

![Nodecraft](https://img.shields.io/badge/rust-stable-orange) ![Nodecraft](https://img.shields.io/badge/node-%3E%3D18-brightgreen) ![License](https://img.shields.io/badge/license-MIT-blue)

## Quick Start

### Rust (recommended — best performance)

```bash
cd rust
cargo run --release
```

See [`rust/README.md`](rust/README.md) for system dependencies and multiplayer server.

### Browser (GitHub Pages / Node.js)

```bash
npm install
npm start
```

Open [http://localhost:3000](http://localhost:3000). GitHub Pages runs single-player only.

## Features

### World & Building
- **Procedural terrain** — infinite world generated with Perlin noise
- **Biomes** — grasslands, deserts, snowy peaks, and oceans
- **Trees** — procedurally placed oak trees
- **Block interaction** — break blocks (LMB) and place blocks (RMB)
- **13+ block types** — grass, dirt, stone, wood, glass, crafting table, and more

### Crafting & Inventory
- **Full inventory system** — 36 slots with stackable items (press `E`)
- **Block drops** — breaking blocks adds resources to your inventory
- **Crafting recipes** — wood → planks → sticks, crafting table, glass, and more
- **Recipe panel** — click available recipes to craft instantly

### Mobs
- **Pigs & Cows** — passive mobs that wander during the day
- **Zombies** — hostile mobs that chase players at night
- **Combat** — attack mobs with LMB to collect drops (pork, beef, leather, rotten flesh)
- **Day/night cycle** — sky color changes, zombie spawning at night

### Multiplayer
- **WebSocket server** — automatic multiplayer when running `npm start`
- **Shared world** — all players see the same terrain, block changes, and mobs
- **Player avatars** — see other players with name tags in the world
- **Real-time sync** — movement, block placement, and mob state synced across clients
- **Not available on GitHub Pages** — requires the Node.js server for WebSocket support

### Mobile
- **Touch controls** — virtual joystick, drag-to-look, on-screen action buttons
- **Works on phones & tablets** — iOS Safari, Android Chrome, and GitHub Pages
- **Tap hotbar slots** to switch blocks; bag button opens inventory & crafting

## Controls (desktop)

| Key | Action |
|-----|--------|
| `W A S D` | Move |
| `Space` | Jump |
| `Shift` | Sprint |
| `Mouse` | Look around |
| `LMB` | Break block / Attack mob |
| `RMB` | Place block |
| `E` | Open inventory & crafting |
| `1-9` | Select hotbar slot |

## Controls (browser)

| Control | Action |
|---------|--------|
| Joystick (left) | Move |
| Drag right side | Look around |
| ⬆ | Jump |
| ⚡ | Sprint |
| ⛏ | Break / Attack |
| ▣ | Place block |
| 🎒 | Inventory & crafting |
| Hotbar tap | Select block |

## Crafting Recipes

| Recipe | Ingredients | Output |
|--------|-------------|--------|
| Planks | 1 Wood | 4 Planks |
| Sticks | 2 Planks | 4 Sticks |
| Crafting Table | 4 Planks (2×2) | 1 Crafting Table |
| Glass | 4 Sand (2×2) | 4 Glass |
| Cobblestone | 1 Stone | 1 Cobblestone |

## Tech Stack

- **Rust + Bevy** — native desktop client (`rust/`)
- **Node.js** + **Express** + **ws** — web server + browser multiplayer
- **Three.js** — WebGL browser client (`public/`)
- **Tokio + Axum** — Rust WebSocket server (`rust/src/bin/server.rs`)

## Architecture

```
rust/                  # Native Rust + Bevy client (recommended)
  src/main.rs
  src/world.rs         # Chunks, terrain, villages
  src/bin/server.rs    # Multiplayer server

public/                # Browser client (GitHub Pages)
  js/main.js
  js/world.js

src/                   # Node.js server (legacy web multiplayer)
  server.js
  gameServer.js
```

## License

MIT
