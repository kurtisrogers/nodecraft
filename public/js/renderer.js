import * as THREE from 'three';
import { getBlockColor, isTransparent, BLOCKS } from './blocks.js';
import { CHUNK_SIZE, WORLD_HEIGHT } from './world.js';

const FACE_DIRECTIONS = [
  { dir: [0, 1, 0], face: 'top' },
  { dir: [0, -1, 0], face: 'bottom' },
  { dir: [1, 0, 0], face: 'side' },
  { dir: [-1, 0, 0], face: 'side' },
  { dir: [0, 0, 1], face: 'side' },
  { dir: [0, 0, -1], face: 'side' },
];

const REMESH_BUDGET_DESKTOP = 4;
const REMESH_BUDGET_MOBILE = 2;

function addFace(vertices, normals, colors, x, y, z, face, blockId, dir) {
  const color = new THREE.Color(getBlockColor(blockId, face));
  const shade = face === 'top' ? 1.0 : face === 'bottom' ? 0.6 : 0.8;
  color.multiplyScalar(shade);
  if (BLOCKS[blockId]?.emissive) {
    color.multiplyScalar(1.5);
  }

  const positions = {
    top: [
      [x, y + 1, z], [x + 1, y + 1, z], [x + 1, y + 1, z + 1], [x, y + 1, z + 1],
    ],
    bottom: [
      [x, y, z + 1], [x + 1, y, z + 1], [x + 1, y, z], [x, y, z],
    ],
    '+x': [
      [x + 1, y, z], [x + 1, y, z + 1], [x + 1, y + 1, z + 1], [x + 1, y + 1, z],
    ],
    '-x': [
      [x, y, z + 1], [x, y, z], [x, y + 1, z], [x, y + 1, z + 1],
    ],
    '+z': [
      [x + 1, y, z + 1], [x, y, z + 1], [x, y + 1, z + 1], [x + 1, y + 1, z + 1],
    ],
    '-z': [
      [x, y, z], [x + 1, y, z], [x + 1, y + 1, z], [x, y + 1, z],
    ],
  };

  let key;
  if (face === 'top') key = 'top';
  else if (face === 'bottom') key = 'bottom';
  else if (dir[0] === 1) key = '+x';
  else if (dir[0] === -1) key = '-x';
  else if (dir[2] === 1) key = '+z';
  else key = '-z';

  const verts = positions[key];
  const normal = new THREE.Vector3(...dir);

  const indices = [0, 1, 2, 0, 2, 3];
  for (const i of indices) {
    vertices.push(...verts[i]);
    normals.push(normal.x, normal.y, normal.z);
    colors.push(color.r, color.g, color.b);
  }
}

export class ChunkMesher {
  constructor(world) {
    this.world = world;
  }

  getNeighborBlock(chunk, lx, ly, lz) {
    if (ly < 0 || ly >= WORLD_HEIGHT) return 0;

    let chunkX = chunk.chunkX;
    let chunkZ = chunk.chunkZ;
    let nx = lx;
    let nz = lz;

    if (nx < 0) {
      chunkX--;
      nx += CHUNK_SIZE;
    } else if (nx >= CHUNK_SIZE) {
      chunkX++;
      nx -= CHUNK_SIZE;
    }

    if (nz < 0) {
      chunkZ--;
      nz += CHUNK_SIZE;
    } else if (nz >= CHUNK_SIZE) {
      chunkZ++;
      nz -= CHUNK_SIZE;
    }

    const worldX = chunkX * CHUNK_SIZE + nx;
    const worldZ = chunkZ * CHUNK_SIZE + nz;
    return this.world.peekBlock(worldX, ly, worldZ);
  }

  buildChunkMesh(chunk, material) {
    const vertices = [];
    const normals = [];
    const colors = [];

    for (let x = 0; x < CHUNK_SIZE; x++) {
      for (let y = 0; y < WORLD_HEIGHT; y++) {
        for (let z = 0; z < CHUNK_SIZE; z++) {
          const blockId = chunk.getBlock(x, y, z);
          if (blockId === 0) continue;

          for (const { dir, face } of FACE_DIRECTIONS) {
            const neighborId = this.getNeighborBlock(chunk, x + dir[0], y + dir[1], z + dir[2]);
            if (isTransparent(neighborId)) {
              addFace(vertices, normals, colors, x, y, z, face, blockId, dir);
            }
          }
        }
      }
    }

    if (vertices.length === 0) return null;

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3));
    geometry.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
    geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
    geometry.computeBoundingSphere();

    const worldXBase = chunk.chunkX * CHUNK_SIZE;
    const worldZBase = chunk.chunkZ * CHUNK_SIZE;
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(worldXBase, 0, worldZBase);
    mesh.frustumCulled = false;
    return mesh;
  }
}

export class WorldRenderer {
  constructor(scene, world, options = {}) {
    this.scene = scene;
    this.world = world;
    this.mesher = new ChunkMesher(world);
    this.chunkMeshes = new Map();
    this.remeshQueue = [];
    this.remeshQueued = new Set();
    this.centerChunkX = null;
    this.centerChunkZ = null;
    this.remeshBudget = options.mobile ? REMESH_BUDGET_MOBILE : REMESH_BUDGET_DESKTOP;
    this.sharedMaterial = new THREE.MeshLambertMaterial({
      vertexColors: true,
      side: THREE.FrontSide,
      polygonOffset: true,
      polygonOffsetFactor: 1,
      polygonOffsetUnits: 1,
    });
  }

  enqueueChunk(chunkX, chunkZ) {
    const key = `${chunkX},${chunkZ}`;
    if (this.remeshQueued.has(key)) return;
    this.remeshQueued.add(key);
    this.remeshQueue.push(key);
  }

  remeshChunkImmediate(chunkX, chunkZ) {
    const key = `${chunkX},${chunkZ}`;
    this.remeshQueued.delete(key);
    this.remeshQueue = this.remeshQueue.filter((k) => k !== key);

    const chunk = this.world.chunks.get(this.world.chunkKey(chunkX, chunkZ));
    if (!chunk) return;

    this.removeChunkMesh(key);
    const mesh = this.mesher.buildChunkMesh(chunk, this.sharedMaterial);
    if (mesh) {
      this.scene.add(mesh);
      this.chunkMeshes.set(key, mesh);
    }
    chunk.dirty = false;
    chunk.mesh = mesh;
  }

  flushBorderRing(centerChunkX, centerChunkZ, radius = 1) {
    for (let dx = -radius; dx <= radius; dx++) {
      for (let dz = -radius; dz <= radius; dz++) {
        const chunkX = centerChunkX + dx;
        const chunkZ = centerChunkZ + dz;
        const key = `${chunkX},${chunkZ}`;
        const chunk = this.world.chunks.get(this.world.chunkKey(chunkX, chunkZ));
        if (!chunk) continue;
        if (chunk.dirty || !this.chunkMeshes.has(key)) {
          this.remeshChunkImmediate(chunkX, chunkZ);
        }
      }
    }
  }

  update(playerX, playerZ) {
    const chunkX = Math.floor(playerX / CHUNK_SIZE);
    const chunkZ = Math.floor(playerZ / CHUNK_SIZE);
    const movedChunk = chunkX !== this.centerChunkX || chunkZ !== this.centerChunkZ;

    if (movedChunk) {
      this.centerChunkX = chunkX;
      this.centerChunkZ = chunkZ;

      const chunks = this.world.loadChunksAround(playerX, playerZ);
      this.world.unloadDistantChunks(playerX, playerZ);

      const activeKeys = new Set();
      for (const chunk of chunks) {
        const key = `${chunk.chunkX},${chunk.chunkZ}`;
        activeKeys.add(key);
        if (chunk.dirty || !this.chunkMeshes.has(key)) {
          this.enqueueChunk(chunk.chunkX, chunk.chunkZ);
        }
      }

      for (const [key, mesh] of this.chunkMeshes) {
        if (!activeKeys.has(key)) {
          this.scene.remove(mesh);
          mesh.geometry.dispose();
          this.chunkMeshes.delete(key);
        }
      }

      this.flushBorderRing(chunkX, chunkZ, 1);
    }

    this.processRemeshQueue();
  }

  processRemeshQueue() {
    const budget = this.remeshQueue.length > 16
      ? this.remeshBudget * 3
      : this.remeshBudget;
    let processed = 0;

    while (this.remeshQueue.length > 0 && processed < budget) {
      const key = this.remeshQueue.shift();
      this.remeshQueued.delete(key);
      if (!key) continue;

      const [chunkX, chunkZ] = key.split(',').map(Number);
      const chunk = this.world.chunks.get(this.world.chunkKey(chunkX, chunkZ));
      if (!chunk) continue;

      this.removeChunkMesh(key);
      const mesh = this.mesher.buildChunkMesh(chunk, this.sharedMaterial);
      if (mesh) {
        this.scene.add(mesh);
        this.chunkMeshes.set(key, mesh);
      }
      chunk.dirty = false;
      chunk.mesh = mesh;
      processed++;
    }
  }

  removeChunkMesh(key) {
    const existing = this.chunkMeshes.get(key);
    if (existing) {
      this.scene.remove(existing);
      existing.geometry.dispose();
      this.chunkMeshes.delete(key);
    }
  }

  rebuildChunkAt(worldX, worldZ) {
    const { chunkX, chunkZ } = this.world.worldToChunk(worldX, worldZ);
    const chunk = this.world.getChunk(chunkX, chunkZ);
    chunk.dirty = true;
    this.remeshChunkImmediate(chunkX, chunkZ);

    const local = chunk.worldToLocal(worldX, 0, worldZ);
    if (local.x === 0) this.enqueueNeighbor(chunkX - 1, chunkZ, true);
    if (local.x === CHUNK_SIZE - 1) this.enqueueNeighbor(chunkX + 1, chunkZ, true);
    if (local.z === 0) this.enqueueNeighbor(chunkX, chunkZ - 1, true);
    if (local.z === CHUNK_SIZE - 1) this.enqueueNeighbor(chunkX, chunkZ + 1, true);
  }

  enqueueNeighbor(chunkX, chunkZ, immediate = false) {
    const chunk = this.world.chunks.get(this.world.chunkKey(chunkX, chunkZ));
    if (!chunk) return;
    chunk.dirty = true;
    if (immediate) {
      this.remeshChunkImmediate(chunkX, chunkZ);
    } else {
      this.enqueueChunk(chunkX, chunkZ);
    }
  }

  dispose() {
    for (const [, mesh] of this.chunkMeshes) {
      this.scene.remove(mesh);
      mesh.geometry.dispose();
    }
    this.chunkMeshes.clear();
    this.sharedMaterial.dispose();
    this.remeshQueue = [];
    this.remeshQueued.clear();
  }
}

export class HighlightBox {
  constructor(scene) {
    const geometry = new THREE.BoxGeometry(1.005, 1.005, 1.005);
    const edges = new THREE.EdgesGeometry(geometry);
    this.mesh = new THREE.LineSegments(
      edges,
      new THREE.LineBasicMaterial({ color: 0x000000 })
    );
    this.mesh.visible = false;
    scene.add(this.mesh);
  }

  show(x, y, z) {
    this.mesh.position.set(x + 0.5, y + 0.5, z + 0.5);
    this.mesh.visible = true;
  }

  hide() {
    this.mesh.visible = false;
  }
}
