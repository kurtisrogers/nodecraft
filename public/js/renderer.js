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

  buildChunkMesh(chunk) {
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
    geometry.computeBoundingBox();

    const material = new THREE.MeshLambertMaterial({
      vertexColors: true,
      side: THREE.FrontSide,
    });

    const worldXBase = chunk.chunkX * CHUNK_SIZE;
    const worldZBase = chunk.chunkZ * CHUNK_SIZE;
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(worldXBase, 0, worldZBase);
    mesh.frustumCulled = false;
    return mesh;
  }
}

export class WorldRenderer {
  constructor(scene, world) {
    this.scene = scene;
    this.world = world;
    this.mesher = new ChunkMesher(world);
    this.chunkMeshes = new Map();
  }

  update(playerX, playerZ) {
    const chunks = this.world.loadChunksAround(playerX, playerZ);
    this.world.unloadDistantChunks(playerX, playerZ);

    const activeKeys = new Set(chunks.map((chunk) => `${chunk.chunkX},${chunk.chunkZ}`));

    // Mesh only after the full chunk ring is loaded so border faces are correct.
    for (const chunk of chunks) {
      const key = `${chunk.chunkX},${chunk.chunkZ}`;
      if (!chunk.dirty && this.chunkMeshes.has(key)) continue;

      this.removeChunkMesh(key);
      const mesh = this.mesher.buildChunkMesh(chunk);
      if (mesh) {
        this.scene.add(mesh);
        this.chunkMeshes.set(key, mesh);
      }
      chunk.dirty = false;
      chunk.mesh = mesh;
    }

    for (const [key, mesh] of this.chunkMeshes) {
      if (!activeKeys.has(key)) {
        this.scene.remove(mesh);
        mesh.geometry.dispose();
        mesh.material.dispose();
        this.chunkMeshes.delete(key);
      }
    }
  }

  removeChunkMesh(key) {
    const existing = this.chunkMeshes.get(key);
    if (existing) {
      this.scene.remove(existing);
      existing.geometry.dispose();
      existing.material.dispose();
      this.chunkMeshes.delete(key);
    }
  }

  rebuildChunkAt(worldX, worldZ) {
    const { chunkX, chunkZ } = this.world.worldToChunk(worldX, worldZ);
    const chunk = this.world.getChunk(chunkX, chunkZ);
    chunk.dirty = true;

    const local = chunk.worldToLocal(worldX, 0, worldZ);
    if (local.x === 0) this.world.getChunk(chunkX - 1, chunkZ).dirty = true;
    if (local.x === CHUNK_SIZE - 1) this.world.getChunk(chunkX + 1, chunkZ).dirty = true;
    if (local.z === 0) this.world.getChunk(chunkX, chunkZ - 1).dirty = true;
    if (local.z === CHUNK_SIZE - 1) this.world.getChunk(chunkX, chunkZ + 1).dirty = true;
  }

  dispose() {
    for (const [, mesh] of this.chunkMeshes) {
      this.scene.remove(mesh);
      mesh.geometry.dispose();
      mesh.material.dispose();
    }
    this.chunkMeshes.clear();
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
