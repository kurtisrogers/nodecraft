import { BlockId, isSolid, isTransparent } from './blocks.js';
import { NoiseGenerator } from './noise.js';
import { getRenderSettings } from './config.js';

export const CHUNK_SIZE = 16;
export const WORLD_HEIGHT = 64;
export const SEA_LEVEL = 30;

export class Chunk {
  constructor(chunkX, chunkZ, noise) {
    this.chunkX = chunkX;
    this.chunkZ = chunkZ;
    this.noise = noise;
    this.blocks = new Uint8Array(CHUNK_SIZE * WORLD_HEIGHT * CHUNK_SIZE);
    this.dirty = true;
    this.mesh = null;
    this.generate();
  }

  index(x, y, z) {
    return x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE;
  }

  getBlock(x, y, z) {
    if (x < 0 || x >= CHUNK_SIZE || y < 0 || y >= WORLD_HEIGHT || z < 0 || z >= CHUNK_SIZE) {
      return BlockId.AIR;
    }
    return this.blocks[this.index(x, y, z)];
  }

  setBlock(x, y, z, blockId) {
    if (x < 0 || x >= CHUNK_SIZE || y < 0 || y >= WORLD_HEIGHT || z < 0 || z >= CHUNK_SIZE) {
      return false;
    }
    this.blocks[this.index(x, y, z)] = blockId;
    this.dirty = true;
    return true;
  }

  worldToLocal(worldX, worldY, worldZ) {
    const chunkX = Math.floor(worldX / CHUNK_SIZE);
    const chunkZ = Math.floor(worldZ / CHUNK_SIZE);
    return {
      x: worldX - chunkX * CHUNK_SIZE,
      y: worldY,
      z: worldZ - chunkZ * CHUNK_SIZE,
    };
  }

  generate() {
    const worldXBase = this.chunkX * CHUNK_SIZE;
    const worldZBase = this.chunkZ * CHUNK_SIZE;

    for (let x = 0; x < CHUNK_SIZE; x++) {
      for (let z = 0; z < CHUNK_SIZE; z++) {
        const worldX = worldXBase + x;
        const worldZ = worldZBase + z;
        const height = this.noise.terrainHeight(worldX, worldZ);
        const biome = this.noise.biome(worldX, worldZ);
        const isDesert = biome.temperature > 0.3 && biome.moisture < -0.1;
        const isSnow = biome.temperature < -0.3;

        for (let y = 0; y < WORLD_HEIGHT; y++) {
          let block = BlockId.AIR;

          if (y === 0) {
            block = BlockId.BEDROCK;
          } else if (y < height - 4) {
            block = BlockId.STONE;
          } else if (y < height - 1) {
            block = isDesert ? BlockId.SAND : BlockId.DIRT;
          } else if (y < height) {
            if (isDesert) block = BlockId.SAND;
            else if (isSnow) block = BlockId.SNOW;
            else block = BlockId.GRASS;
          } else if (y <= SEA_LEVEL) {
            block = BlockId.WATER;
          }

          this.blocks[this.index(x, y, z)] = block;
        }

        if (height > SEA_LEVEL + 1 && !isDesert && !isSnow) {
          if (this.noise.shouldPlaceTree(worldX, worldZ)) {
            const clear = this.countTreeClearance(x, z, height);
            if (clear >= 20) {
              this.generateTree(x, height, z);
            }
          }
        }

        if (this.noise.isVolcanic(worldX, worldZ)) {
          this.generateVolcanicFeatures(x, z, height);
        } else {
          this.generateUndergroundLava(x, z, height);
        }
      }
    }
  }

  generateVolcanicFeatures(x, z, height) {
    const poolRoll = this.noise.roll(this.chunkX * CHUNK_SIZE + x, this.chunkZ * CHUNK_SIZE + z, 31);
    if (poolRoll > 0.92 && height > SEA_LEVEL + 2) {
      const y = height;
      for (let dx = -1; dx <= 1; dx++) {
        for (let dz = -1; dz <= 1; dz++) {
          if (Math.abs(dx) + Math.abs(dz) > 1) continue;
          const lx = x + dx;
          const lz = z + dz;
          if (lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE) continue;
          this.setBlock(lx, y, lz, BlockId.LAVA);
          if (y > 1) this.setBlock(lx, y - 1, lz, BlockId.STONE);
        }
      }
      return;
    }
    this.generateUndergroundLava(x, z, height);
  }

  countTreeClearance(x, z, groundY) {
    let score = 0;
    for (let dx = -2; dx <= 2; dx++) {
      for (let dz = -2; dz <= 2; dz++) {
        const lx = x + dx;
        const lz = z + dz;
        if (lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE) continue;
        const foot = this.getBlock(lx, groundY + 1, lz);
        const head = this.getBlock(lx, groundY + 2, lz);
        if (foot === BlockId.AIR) score++;
        if (head === BlockId.AIR) score++;
      }
    }
    return score;
  }

  generateUndergroundLava(x, z, height) {
    const worldX = this.chunkX * CHUNK_SIZE + x;
    const worldZ = this.chunkZ * CHUNK_SIZE + z;
    const pool = this.noise.lavaPoolChance(worldX, worldZ);
    if (pool < 0.62) return;

    const depth = 2 + Math.floor(this.noise.roll(worldX, worldZ, 19) * 6);
    const radius = this.noise.roll(worldX, worldZ, 23) > 0.7 ? 2 : 1;

    for (let dx = -radius; dx <= radius; dx++) {
      for (let dz = -radius; dz <= radius; dz++) {
        if (dx * dx + dz * dz > radius * radius) continue;
        const lx = x + dx;
        const lz = z + dz;
        if (lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE) continue;
        for (let dy = 0; dy < 2; dy++) {
          const ly = depth + dy;
          if (ly >= height || ly >= WORLD_HEIGHT - 1) continue;
          if (this.getBlock(lx, ly, lz) === BlockId.STONE) {
            this.setBlock(lx, ly, lz, dy === 0 ? BlockId.LAVA : BlockId.AIR);
          }
        }
      }
    }
  }

  generateTree(x, groundY, z) {
    if (this.getBlock(x, groundY, z) !== BlockId.GRASS && this.getBlock(x, groundY, z) !== BlockId.DIRT) {
      return;
    }

    const worldX = this.chunkX * CHUNK_SIZE + x;
    const worldZ = this.chunkZ * CHUNK_SIZE + z;
    const variant = this.noise.roll(worldX, worldZ, 11);
    const trunkHeight = variant > 0.7 ? 6 : variant > 0.35 ? 5 : 4;
    for (let y = 0; y < trunkHeight; y++) {
      if (groundY + y < WORLD_HEIGHT) {
        this.setBlock(x, groundY + y, z, BlockId.WOOD);
      }
    }
    const leafStart = groundY + trunkHeight - 2;
    for (let dy = 0; dy < 4; dy++) {
      for (let dx = -2; dx <= 2; dx++) {
        for (let dz = -2; dz <= 2; dz++) {
          if (Math.abs(dx) === 2 && Math.abs(dz) === 2) continue;
          if (dy === 3 && (Math.abs(dx) > 1 || Math.abs(dz) > 1)) continue;
          const lx = x + dx;
          const ly = leafStart + dy;
          const lz = z + dz;
          if (this.getBlock(lx, ly, lz) === BlockId.AIR) {
            this.setBlock(lx, ly, lz, BlockId.LEAVES);
          }
        }
      }
    }
  }
}

export class World {
  constructor(seed = 42, renderSettings = getRenderSettings()) {
    this.seed = seed;
    this.noise = new NoiseGenerator(seed);
    this.chunks = new Map();
    this.renderDistance = renderSettings.renderDistance;
    this.modifications = new Map();
  }

  modKey(x, y, z) {
    return `${x},${y},${z}`;
  }

  applyModifications(changes) {
    for (const { x, y, z, blockId } of changes) {
      this.modifications.set(this.modKey(x, y, z), blockId);
      this.setBlockLocal(x, y, z, blockId);
    }
  }

  setBlockLocal(worldX, worldY, worldZ, blockId) {
    if (worldY < 0 || worldY >= WORLD_HEIGHT) return false;
    const { chunkX, chunkZ } = this.worldToChunk(worldX, worldZ);
    const chunk = this.getChunk(chunkX, chunkZ);
    const local = chunk.worldToLocal(worldX, worldY, worldZ);
    const result = chunk.setBlock(local.x, local.y, local.z, blockId);
    const neighbors = [];
    if (local.x === 0) neighbors.push([chunkX - 1, chunkZ]);
    if (local.x === CHUNK_SIZE - 1) neighbors.push([chunkX + 1, chunkZ]);
    if (local.z === 0) neighbors.push([chunkX, chunkZ - 1]);
    if (local.z === CHUNK_SIZE - 1) neighbors.push([chunkX, chunkZ + 1]);
    for (const [nx, nz] of neighbors) {
      const neighbor = this.chunks.get(this.chunkKey(nx, nz));
      if (neighbor) neighbor.dirty = true;
    }
    return result;
  }

  chunkKey(chunkX, chunkZ) {
    return `${chunkX},${chunkZ}`;
  }

  worldToChunk(worldX, worldZ) {
    return {
      chunkX: Math.floor(worldX / CHUNK_SIZE),
      chunkZ: Math.floor(worldZ / CHUNK_SIZE),
    };
  }

  getChunk(chunkX, chunkZ) {
    const key = this.chunkKey(chunkX, chunkZ);
    if (!this.chunks.has(key)) {
      const chunk = new Chunk(chunkX, chunkZ, this.noise);
      this.chunks.set(key, chunk);
      this.markNeighborChunksDirty(chunkX, chunkZ);
      return chunk;
    }
    return this.chunks.get(key);
  }

  markNeighborChunksDirty(chunkX, chunkZ) {
    const offsets = [
      [-1, 0], [1, 0], [0, -1], [0, 1],
      [-1, -1], [-1, 1], [1, -1], [1, 1],
    ];
    for (const [dx, dz] of offsets) {
      const neighbor = this.chunks.get(this.chunkKey(chunkX + dx, chunkZ + dz));
      if (neighbor) neighbor.dirty = true;
    }
  }

  isVolumeClear(x, y, z, half = 0.3, height = 1.7) {
    const minX = Math.floor(x - half);
    const maxX = Math.floor(x + half);
    const minY = Math.floor(y);
    const maxY = Math.floor(y + height);
    const minZ = Math.floor(z - half);
    const maxZ = Math.floor(z + half);

    for (let bx = minX; bx <= maxX; bx++) {
      for (let by = minY; by <= maxY; by++) {
        for (let bz = minZ; bz <= maxZ; bz++) {
          if (isSolid(this.getBlock(bx, by, bz))) return false;
        }
      }
    }
    return true;
  }

  peekBlock(worldX, worldY, worldZ) {
    if (worldY < 0 || worldY >= WORLD_HEIGHT) return BlockId.AIR;
    const mod = this.modifications.get(this.modKey(worldX, worldY, worldZ));
    if (mod !== undefined) return mod;
    const { chunkX, chunkZ } = this.worldToChunk(worldX, worldZ);
    const chunk = this.chunks.get(this.chunkKey(chunkX, chunkZ));
    if (!chunk) return BlockId.AIR;
    const local = chunk.worldToLocal(worldX, worldY, worldZ);
    return chunk.getBlock(local.x, local.y, local.z);
  }

  getBlock(worldX, worldY, worldZ) {
    if (worldY < 0 || worldY >= WORLD_HEIGHT) return BlockId.AIR;
    const mod = this.modifications.get(this.modKey(worldX, worldY, worldZ));
    if (mod !== undefined) return mod;
    const { chunkX, chunkZ } = this.worldToChunk(worldX, worldZ);
    const chunk = this.getChunk(chunkX, chunkZ);
    const local = chunk.worldToLocal(worldX, worldY, worldZ);
    return chunk.getBlock(local.x, local.y, local.z);
  }

  setBlock(worldX, worldY, worldZ, blockId) {
    if (blockId === BlockId.AIR) {
      this.modifications.delete(this.modKey(worldX, worldY, worldZ));
    } else {
      this.modifications.set(this.modKey(worldX, worldY, worldZ), blockId);
    }
    return this.setBlockLocal(worldX, worldY, worldZ, blockId);
  }

  isBlockSolid(worldX, worldY, worldZ) {
    return isSolid(this.getBlock(worldX, worldY, worldZ));
  }

  isBlockTransparent(worldX, worldY, worldZ) {
    return isTransparent(this.getBlock(worldX, worldY, worldZ));
  }

  loadChunksAround(worldX, worldZ) {
    const { chunkX: centerX, chunkZ: centerZ } = this.worldToChunk(worldX, worldZ);
    const pending = [];
    for (let dx = -this.renderDistance; dx <= this.renderDistance; dx++) {
      for (let dz = -this.renderDistance; dz <= this.renderDistance; dz++) {
        if (dx * dx + dz * dz > this.renderDistance * this.renderDistance) continue;
        pending.push([centerX + dx, centerZ + dz]);
      }
    }

    // Load every chunk in range before meshing so borders see real neighbors.
    const loaded = [];
    for (const [cx, cz] of pending) {
      loaded.push(this.getChunk(cx, cz));
    }
    return loaded;
  }

  unloadDistantChunks(worldX, worldZ) {
    const { chunkX: centerX, chunkZ: centerZ } = this.worldToChunk(worldX, worldZ);
    const maxDist = this.renderDistance + 2;
    const removed = [];

    for (const [key, chunk] of this.chunks) {
      const dist = Math.max(Math.abs(chunk.chunkX - centerX), Math.abs(chunk.chunkZ - centerZ));
      if (dist > maxDist) {
        removed.push(chunk);
        this.chunks.delete(key);
      }
    }

    for (const chunk of removed) {
      this.markNeighborChunksDirty(chunk.chunkX, chunk.chunkZ);
    }
  }

  getTopSolidBlockY(worldX, worldZ) {
    this.getChunk(Math.floor(worldX / CHUNK_SIZE), Math.floor(worldZ / CHUNK_SIZE));
    for (let y = WORLD_HEIGHT - 1; y >= 0; y--) {
      if (isSolid(this.getBlock(worldX, y, worldZ))) return y;
    }
    return 0;
  }

  getWalkableSurfaceY(worldX, worldZ) {
    this.getChunk(Math.floor(worldX / CHUNK_SIZE), Math.floor(worldZ / CHUNK_SIZE));
    for (let y = WORLD_HEIGHT - 1; y >= 0; y--) {
      const block = this.getBlock(worldX, y, worldZ);
      if (
        block === BlockId.GRASS ||
        block === BlockId.DIRT ||
        block === BlockId.SAND ||
        block === BlockId.SNOW ||
        block === BlockId.PLANKS
      ) {
        return y;
      }
    }
    return -1;
  }

  countOpenSpace(x, z, surfaceY) {
    let score = 0;
    for (let dx = -2; dx <= 2; dx++) {
      for (let dz = -2; dz <= 2; dz++) {
        const bx = x + dx;
        const bz = z + dz;
        const foot = this.getBlock(bx, surfaceY + 1, bz);
        const head = this.getBlock(bx, surfaceY + 2, bz);
        if (!isSolid(foot) && foot !== BlockId.LAVA && foot !== BlockId.WATER) score++;
        if (!isSolid(head) && head !== BlockId.LAVA && head !== BlockId.WATER) score++;
      }
    }
    return score;
  }

  findSafeSpawn(preferredX = 0, preferredZ = 0) {
    const candidates = [{ x: preferredX, z: preferredZ }];
    for (let r = 2; r <= 40; r += 2) {
      for (let a = 0; a < 12; a++) {
        const angle = (a / 12) * Math.PI * 2;
        candidates.push({
          x: Math.round(preferredX + Math.cos(angle) * r),
          z: Math.round(preferredZ + Math.sin(angle) * r),
        });
      }
    }

    let best = null;
    for (const { x, z } of candidates) {
      this.getChunk(Math.floor(x / CHUNK_SIZE), Math.floor(z / CHUNK_SIZE));
      const surfaceY = this.getWalkableSurfaceY(x, z);
      if (surfaceY < 0 || surfaceY <= SEA_LEVEL) continue;
      if (this.getBlock(x, surfaceY, z) === BlockId.LAVA) continue;

      const openness = this.countOpenSpace(x, z, surfaceY);
      if (openness < 40) continue;

      const spawnY = surfaceY + 1;
      if (!this.isVolumeClear(x + 0.5, spawnY, z + 0.5)) continue;

      const dist = Math.abs(x - preferredX) + Math.abs(z - preferredZ);
      const score = openness * 5 - dist;
      if (!best || score > best.score) {
        best = { x, z, y: spawnY, score };
      }
    }

    if (best) {
      return { x: best.x + 0.5, y: best.y, z: best.z + 0.5 };
    }

    const surfaceY = this.getWalkableSurfaceY(preferredX, preferredZ);
    const fallbackY = Math.max(surfaceY + 1, 2);
    if (this.isVolumeClear(preferredX + 0.5, fallbackY, preferredZ + 0.5)) {
      return {
        x: preferredX + 0.5,
        y: fallbackY,
        z: preferredZ + 0.5,
      };
    }

    const topY = this.getTopSolidBlockY(preferredX, preferredZ) + 1;
    return {
      x: preferredX + 0.5,
      y: topY,
      z: preferredZ + 0.5,
    };
  }

  getSpawnHeight(worldX, worldZ) {
    this.getChunk(Math.floor(worldX / CHUNK_SIZE), Math.floor(worldZ / CHUNK_SIZE));
    return this.getTopSolidBlockY(worldX, worldZ) + 1;
  }
}
