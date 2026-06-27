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

const REMESH_BUDGET_DESKTOP = 3;
const REMESH_BUDGET_MOBILE = 2;

const FACE_VERTS = {
  top: [
    [0, 1, 0], [1, 1, 0], [1, 1, 1], [0, 1, 1],
  ],
  bottom: [
    [0, 0, 1], [1, 0, 1], [1, 0, 0], [0, 0, 0],
  ],
  '+x': [
    [1, 0, 0], [1, 0, 1], [1, 1, 1], [1, 1, 0],
  ],
  '-x': [
    [0, 0, 1], [0, 0, 0], [0, 1, 0], [0, 1, 1],
  ],
  '+z': [
    [1, 0, 1], [0, 0, 1], [0, 1, 1], [1, 1, 1],
  ],
  '-z': [
    [0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0],
  ],
};

const FACE_INDICES = [0, 1, 2, 0, 2, 3];
const _reusableColor = new THREE.Color();
const _chunkBounds = new THREE.Box3();

function faceKey(face, dir) {
  if (face === 'top') return 'top';
  if (face === 'bottom') return 'bottom';
  if (dir[0] === 1) return '+x';
  if (dir[0] === -1) return '-x';
  if (dir[2] === 1) return '+z';
  return '-z';
}

function addFace(vertices, normals, colors, x, y, z, face, blockId, dir) {
  _reusableColor.set(getBlockColor(blockId, face));
  const shade = face === 'top' ? 1.0 : face === 'bottom' ? 0.6 : 0.8;
  _reusableColor.multiplyScalar(shade);
  if (BLOCKS[blockId]?.emissive) {
    _reusableColor.multiplyScalar(1.5);
  }

  const verts = FACE_VERTS[faceKey(face, dir)];
  const nx = dir[0];
  const ny = dir[1];
  const nz = dir[2];

  for (const i of FACE_INDICES) {
    const v = verts[i];
    vertices.push(x + v[0], y + v[1], z + v[2]);
    normals.push(nx, ny, nz);
    colors.push(_reusableColor.r, _reusableColor.g, _reusableColor.b);
  }
}

function getChunkYRange(chunk) {
  let minY = WORLD_HEIGHT;
  let maxY = 0;
  let hasBlocks = false;

  for (let x = 0; x < CHUNK_SIZE; x++) {
    for (let z = 0; z < CHUNK_SIZE; z++) {
      for (let y = 0; y < WORLD_HEIGHT; y++) {
        if (chunk.getBlock(x, y, z) !== 0) {
          hasBlocks = true;
          if (y < minY) minY = y;
          if (y > maxY) maxY = y;
        }
      }
    }
  }

  if (!hasBlocks) return null;
  return { minY, maxY };
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
    const yRange = getChunkYRange(chunk);
    if (!yRange) return null;

    const { minY, maxY } = yRange;
    const vertices = [];
    const normals = [];
    const colors = [];

    for (let x = 0; x < CHUNK_SIZE; x++) {
      for (let y = minY; y <= maxY; y++) {
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

    const worldXBase = chunk.chunkX * CHUNK_SIZE;
    const worldZBase = chunk.chunkZ * CHUNK_SIZE;
    _chunkBounds.set(
      new THREE.Vector3(worldXBase, minY, worldZBase),
      new THREE.Vector3(worldXBase + CHUNK_SIZE, maxY + 1, worldZBase + CHUNK_SIZE)
    );
    geometry.boundingBox = _chunkBounds.clone();
    geometry.computeBoundingSphere();

    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(worldXBase, 0, worldZBase);
    mesh.frustumCulled = true;
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
    });
  }

  enqueueChunk(chunkX, chunkZ, priority = false) {
    const key = `${chunkX},${chunkZ}`;
    if (this.remeshQueued.has(key)) {
      if (priority) {
        this.remeshQueue = this.remeshQueue.filter((k) => k !== key);
        this.remeshQueue.unshift(key);
      }
      return;
    }
    this.remeshQueued.add(key);
    if (priority) {
      this.remeshQueue.unshift(key);
    } else {
      this.remeshQueue.push(key);
    }
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
          this.enqueueChunk(chunkX, chunkZ, true);
        }
      }
    }
    this.processRemeshQueue(this.remeshBudget * 2);
  }

  update(playerX, playerZ) {
    const chunkX = Math.floor(playerX / CHUNK_SIZE);
    const chunkZ = Math.floor(playerZ / CHUNK_SIZE);
    const movedChunk = chunkX !== this.centerChunkX || chunkZ !== this.centerChunkZ;

    if (movedChunk) {
      this.centerChunkX = chunkX;
      this.centerChunkZ = chunkZ;

      this.world.ensureSettlementsNear(playerX, playerZ, this.world.renderDistance * CHUNK_SIZE + 64);

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

  processRemeshQueue(extraBudget = 0) {
    const backlogBoost = this.remeshQueue.length > 12 ? this.remeshBudget : 0;
    const budget = this.remeshBudget + extraBudget + backlogBoost;
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
    if (local.x === 0) this.enqueueNeighbor(chunkX - 1, chunkZ);
    if (local.x === CHUNK_SIZE - 1) this.enqueueNeighbor(chunkX + 1, chunkZ);
    if (local.z === 0) this.enqueueNeighbor(chunkX, chunkZ - 1);
    if (local.z === CHUNK_SIZE - 1) this.enqueueNeighbor(chunkX, chunkZ + 1);
  }

  enqueueNeighbor(chunkX, chunkZ) {
    const chunk = this.world.chunks.get(this.world.chunkKey(chunkX, chunkZ));
    if (!chunk) return;
    chunk.dirty = true;
    this.enqueueChunk(chunkX, chunkZ, true);
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
