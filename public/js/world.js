import { BlockId, isSolid, isTransparent } from './blocks.js';
import { NoiseGenerator } from './noise.js';

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
    return {
      x: ((worldX % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
      y: worldY,
      z: ((worldZ % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
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
          const treeVal = this.noise.treeChance(worldX, worldZ);
          if (treeVal > 0.55 && treeVal < 0.58) {
            this.generateTree(x, height, z);
          }
        }
      }
    }
  }

  generateTree(x, groundY, z) {
    const trunkHeight = 4 + Math.floor(Math.random() * 2);
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
  constructor(seed = 42) {
    this.seed = seed;
    this.noise = new NoiseGenerator(seed);
    this.chunks = new Map();
    this.renderDistance = 4;
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
      this.chunks.set(key, new Chunk(chunkX, chunkZ, this.noise));
    }
    return this.chunks.get(key);
  }

  getBlock(worldX, worldY, worldZ) {
    if (worldY < 0 || worldY >= WORLD_HEIGHT) return BlockId.AIR;
    const mod = this.modifications.get(this.modKey(worldX, worldY, worldZ));
    if (mod !== undefined) return mod;
    const { chunkX, chunkZ } = this.worldToChunk(worldX, worldZ);
    const chunk = this.chunks.get(this.chunkKey(chunkX, chunkZ));
    if (!chunk) return BlockId.AIR;
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
    const loaded = [];
    for (let dx = -this.renderDistance; dx <= this.renderDistance; dx++) {
      for (let dz = -this.renderDistance; dz <= this.renderDistance; dz++) {
        if (dx * dx + dz * dz > this.renderDistance * this.renderDistance) continue;
        const cx = centerX + dx;
        const cz = centerZ + dz;
        loaded.push(this.getChunk(cx, cz));
      }
    }
    return loaded;
  }

  unloadDistantChunks(worldX, worldZ) {
    const { chunkX: centerX, chunkZ: centerZ } = this.worldToChunk(worldX, worldZ);
    const maxDist = this.renderDistance + 2;
    for (const [key, chunk] of this.chunks) {
      const dist = Math.max(Math.abs(chunk.chunkX - centerX), Math.abs(chunk.chunkZ - centerZ));
      if (dist > maxDist) {
        this.chunks.delete(key);
      }
    }
  }

  getSpawnHeight(worldX, worldZ) {
    return this.noise.terrainHeight(worldX, worldZ) + 2;
  }
}
