# Nodecraft

A Minecraft-like voxel sandbox game built with **Node.js** and **Three.js**.

![Nodecraft](https://img.shields.io/badge/node-%3E%3D18-brightgreen) ![License](https://img.shields.io/badge/license-MIT-blue)

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

## Quick Start

### Local (full game + multiplayer)

```bash
npm install
npm start
```

Open [http://localhost:3000](http://localhost:3000) in your browser. Open multiple tabs or share the URL with friends for multiplayer.

### GitHub Pages (single-player)

GitHub Pages only serves static files — no Node.js or WebSocket server — so the hosted build runs in **single-player mode** with crafting, mobs, and building fully playable.

1. Merge this repo to `main`
2. In your GitHub repo: **Settings → Pages → Build and deployment → Source: GitHub Actions**
3. The included workflow deploys the `public/` folder automatically on push to `main`

Your game will be live at `https://<username>.github.io/nodecraft/`

To test static mode locally: `npm start` then open `http://localhost:3000/?static`

## Controls

### Desktop

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
| `Scroll` | Cycle hotbar |

### Mobile

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

## Architecture

```
src/
  server.js          # Express + WebSocket server
  gameServer.js      # Authoritative game state (players, mobs, blocks)
  shared/protocol.js # Network message types
public/
  js/
    main.js          # Game loop & system integration
    world.js         # Chunk & terrain generation
    inventory.js     # Inventory management
    crafting.js      # Crafting recipes
    items.js         # Item & drop definitions
    mobs.js          # Mob AI, rendering, spawning
    network.js       # WebSocket client
    remotePlayers.js # Other player rendering
    ui.js            # Inventory & crafting UI
```

## Tech Stack

- **Node.js** + **Express** + **ws** — HTTP and WebSocket server
- **Three.js** — WebGL 3D rendering
- **Vanilla JS** — ES modules, no build step

## License

MIT
