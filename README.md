# Nodecraft

A Minecraft-like voxel sandbox game built with **Node.js** and **Three.js**.

![Nodecraft](https://img.shields.io/badge/node-%3E%3D18-brightgreen) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Procedural terrain** — infinite world generated with Perlin noise
- **Biomes** — grasslands, deserts, snowy peaks, and oceans
- **Trees** — procedurally placed oak trees
- **First-person controls** — WASD movement, mouse look, jumping, sprinting
- **Block interaction** — break blocks (LMB) and place blocks (RMB)
- **13 block types** — grass, dirt, stone, wood, leaves, sand, glass, and more
- **Chunk system** — 16×64×16 chunks with dynamic loading/unloading
- **Hotbar** — 9-slot inventory with scroll wheel and number keys

## Quick Start

```bash
npm install
npm start
```

Open [http://localhost:3000](http://localhost:3000) in your browser and click to play.

## Controls

| Key | Action |
|-----|--------|
| `W A S D` | Move |
| `Space` | Jump |
| `Shift` | Sprint |
| `Mouse` | Look around |
| `LMB` | Break block |
| `RMB` | Place block |
| `1-9` | Select block type |
| `Scroll` | Cycle hotbar |

## Architecture

```
src/
  server.js          # Express static file server
public/
  index.html         # Game page
  css/style.css      # HUD styling
  js/
    main.js          # Game loop & initialization
    world.js         # Chunk & world management
    blocks.js        # Block definitions
    noise.js         # Terrain noise generator
    renderer.js      # Three.js mesh builder
    player.js        # Player physics & controls
```

## Tech Stack

- **Node.js** + **Express** — HTTP server
- **Three.js** — WebGL 3D rendering
- **Vanilla JS** — No build step required

## License

MIT
