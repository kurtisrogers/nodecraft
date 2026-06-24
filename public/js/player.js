import * as THREE from 'three';
import { BlockId, HOTBAR_BLOCKS, isSolid } from './blocks.js';
import { WORLD_HEIGHT } from './world.js';

const GRAVITY = -28;
const JUMP_VELOCITY = 9;
const WALK_SPEED = 5;
const SPRINT_SPEED = 9;
const MOUSE_SENSITIVITY = 0.002;
const PLAYER_HEIGHT = 1.7;
const PLAYER_WIDTH = 0.6;

export class Player {
  constructor(camera, world) {
    this.camera = camera;
    this.world = world;
    this.position = new THREE.Vector3(0, 40, 0);
    this.velocity = new THREE.Vector3();
    this.onGround = false;
    this.selectedBlock = BlockId.GRASS;
    this.hotbarIndex = 0;
    this.keys = {};
    this.pointerLocked = false;
    this.yaw = 0;
    this.pitch = 0;
  }

  spawn() {
    const spawnX = 0;
    const spawnZ = 0;
    const height = this.world.getSpawnHeight(spawnX, spawnZ);
    this.position.set(spawnX + 0.5, height, spawnZ + 0.5);
    this.velocity.set(0, 0, 0);
    this.updateCamera();
  }

  setupControls(domElement) {
    document.addEventListener('keydown', (e) => {
      this.keys[e.code] = true;
      if (e.code.startsWith('Digit') && e.code !== 'Digit0') {
        const num = parseInt(e.code.replace('Digit', ''), 10);
        if (num >= 1 && num <= 9) {
          this.hotbarIndex = num - 1;
          this.updateSelectedBlock();
        }
      }
    });

    document.addEventListener('keyup', (e) => {
      this.keys[e.code] = false;
    });

    document.addEventListener('mousemove', (e) => {
      if (!this.pointerLocked) return;
      this.yaw -= e.movementX * MOUSE_SENSITIVITY;
      this.pitch -= e.movementY * MOUSE_SENSITIVITY;
      this.pitch = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, this.pitch));
    });

    domElement.addEventListener('click', () => {
      if (!this.pointerLocked) {
        domElement.requestPointerLock();
      }
    });

    document.addEventListener('pointerlockchange', () => {
      this.pointerLocked = document.pointerLockElement === domElement;
      document.body.classList.toggle('playing', this.pointerLocked);
    });

    document.addEventListener('mousedown', (e) => {
      if (!this.pointerLocked) return;
      e.preventDefault();
      if (e.button === 0) this.onBreakBlock?.();
      if (e.button === 2) this.onPlaceBlock?.();
    });

    document.addEventListener('contextmenu', (e) => e.preventDefault());

    document.addEventListener('wheel', (e) => {
      if (!this.pointerLocked) return;
      this.hotbarIndex = (this.hotbarIndex + (e.deltaY > 0 ? 1 : -1) + 9) % 9;
      this.updateSelectedBlock();
    });
  }

  updateSelectedBlock() {
    this.selectedBlock = HOTBAR_BLOCKS[this.hotbarIndex];
    this.onHotbarChange?.(this.hotbarIndex, this.selectedBlock);
  }

  getForward() {
    return new THREE.Vector3(
      -Math.sin(this.yaw),
      0,
      -Math.cos(this.yaw)
    );
  }

  getRight() {
    return new THREE.Vector3(
      Math.cos(this.yaw),
      0,
      -Math.sin(this.yaw)
    );
  }

  updateCamera() {
    this.camera.position.copy(this.position);
    this.camera.position.y += PLAYER_HEIGHT - 0.2;
    const lookDir = new THREE.Vector3(
      -Math.sin(this.yaw) * Math.cos(this.pitch),
      Math.sin(this.pitch),
      -Math.cos(this.yaw) * Math.cos(this.pitch)
    );
    this.camera.lookAt(this.camera.position.clone().add(lookDir));
  }

  raycast(maxDistance = 6) {
    const origin = this.camera.position.clone();
    const direction = new THREE.Vector3();
    this.camera.getWorldDirection(direction);

    let x = Math.floor(origin.x);
    let y = Math.floor(origin.y);
    let z = Math.floor(origin.z);

    const stepX = direction.x > 0 ? 1 : -1;
    const stepY = direction.y > 0 ? 1 : -1;
    const stepZ = direction.z > 0 ? 1 : -1;

    const tDeltaX = direction.x !== 0 ? Math.abs(1 / direction.x) : Infinity;
    const tDeltaY = direction.y !== 0 ? Math.abs(1 / direction.y) : Infinity;
    const tDeltaZ = direction.z !== 0 ? Math.abs(1 / direction.z) : Infinity;

    let tMaxX = direction.x !== 0
      ? (stepX > 0 ? (x + 1 - origin.x) : (origin.x - x)) * tDeltaX
      : Infinity;
    let tMaxY = direction.y !== 0
      ? (stepY > 0 ? (y + 1 - origin.y) : (origin.y - y)) * tDeltaY
      : Infinity;
    let tMaxZ = direction.z !== 0
      ? (stepZ > 0 ? (z + 1 - origin.z) : (origin.z - z)) * tDeltaZ
      : Infinity;

    let prevX = x, prevY = y, prevZ = z;
    let dist = 0;

    while (dist < maxDistance) {
      const block = this.world.getBlock(x, y, z);
      if (isSolid(block)) {
        return {
          block: { x, y, z },
          face: { x: prevX, y: prevY, z: prevZ },
          distance: dist,
        };
      }

      prevX = x;
      prevY = y;
      prevZ = z;

      if (tMaxX < tMaxY && tMaxX < tMaxZ) {
        dist = tMaxX;
        tMaxX += tDeltaX;
        x += stepX;
      } else if (tMaxY < tMaxZ) {
        dist = tMaxY;
        tMaxY += tDeltaY;
        y += stepY;
      } else {
        dist = tMaxZ;
        tMaxZ += tDeltaZ;
        z += stepZ;
      }

      if (y < 0 || y >= WORLD_HEIGHT) break;
    }
    return null;
  }

  update(dt) {
    if (!this.pointerLocked) {
      this.updateCamera();
      return;
    }

    const speed = this.keys['ShiftLeft'] || this.keys['ShiftRight'] ? SPRINT_SPEED : WALK_SPEED;
    const forward = this.getForward();
    const right = this.getRight();
    const move = new THREE.Vector3();

    if (this.keys['KeyW']) move.add(forward);
    if (this.keys['KeyS']) move.sub(forward);
    if (this.keys['KeyA']) move.sub(right);
    if (this.keys['KeyD']) move.add(right);

    if (move.length() > 0) {
      move.normalize().multiplyScalar(speed * dt);
    }

    this.velocity.x = move.x;
    this.velocity.z = move.z;

    if (this.onGround && this.keys['Space']) {
      this.velocity.y = JUMP_VELOCITY;
      this.onGround = false;
    }

    this.velocity.y += GRAVITY * dt;
    this.moveWithCollision(dt);
    this.updateCamera();
  }

  moveWithCollision(dt) {
    const half = PLAYER_WIDTH / 2;

    this.position.x += this.velocity.x * dt;
    this.collideAxis('x', half);

    this.position.z += this.velocity.z * dt;
    this.collideAxis('z', half);

    this.position.y += this.velocity.y * dt;
    this.onGround = false;
    this.collideAxis('y', half);
  }

  collideAxis(axis, half) {
    const minX = Math.floor(this.position.x - half);
    const maxX = Math.floor(this.position.x + half);
    const minY = Math.floor(this.position.y);
    const maxY = Math.floor(this.position.y + PLAYER_HEIGHT);
    const minZ = Math.floor(this.position.z - half);
    const maxZ = Math.floor(this.position.z + half);

    for (let x = minX; x <= maxX; x++) {
      for (let y = minY; y <= maxY; y++) {
        for (let z = minZ; z <= maxZ; z++) {
          if (!isSolid(this.world.getBlock(x, y, z))) continue;

          if (axis === 'x') {
            if (this.velocity.x > 0) this.position.x = x - half - 0.001;
            else if (this.velocity.x < 0) this.position.x = x + 1 + half + 0.001;
            this.velocity.x = 0;
          } else if (axis === 'z') {
            if (this.velocity.z > 0) this.position.z = z - half - 0.001;
            else if (this.velocity.z < 0) this.position.z = z + 1 + half + 0.001;
            this.velocity.z = 0;
          } else {
            if (this.velocity.y > 0) {
              this.position.y = y - PLAYER_HEIGHT - 0.001;
            } else if (this.velocity.y < 0) {
              this.position.y = y + 1 + 0.001;
              this.onGround = true;
            }
            this.velocity.y = 0;
          }
        }
      }
    }
  }
}
