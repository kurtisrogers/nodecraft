import * as THREE from 'three';
import { BlockId, isSolid, isLava } from './blocks.js';
import { WORLD_HEIGHT } from './world.js';
import { Inventory } from './inventory.js';
import { isBlockItem } from './items.js';

const GRAVITY = -28;
const JUMP_VELOCITY = 10;
const WALK_SPEED = 7;
const SPRINT_SPEED = 12;
const MOUSE_SENSITIVITY = 0.002;
const PLAYER_HEIGHT = 1.7;
const PLAYER_WIDTH = 0.6;
const EYE_HEIGHT = PLAYER_HEIGHT - 0.2;

export class Player {
  constructor(camera, world) {
    this.camera = camera;
    this.world = world;
    this.position = new THREE.Vector3(0, 40, 0);
    this.velocity = new THREE.Vector3();
    this.onGround = false;
    this.inventory = new Inventory();
    this.selectedBlock = BlockId.DIRT;
    this.hotbarIndex = 0;
    this.keys = {};
    this.pointerLocked = false;
    this.yaw = 0;
    this.pitch = 0;
    this.id = null;
    this.name = 'Player';
    this.health = 20;
    this.maxHealth = 20;
    this.attackCooldown = 0;
    this.mobileActive = false;
    this.touchMove = { x: 0, z: 0 };
    this.touchJump = false;
    this.touchSprint = false;
    this.lavaDamageTimer = 0;
    this._headPos = new THREE.Vector3();
    this._lookTarget = new THREE.Vector3();
  }

  spawn() {
    const spawn = this.world.findSafeSpawn(0, 0);
    this.position.set(spawn.x, spawn.y, spawn.z);
    this.velocity.set(0, 0, 0);
    this.onGround = false;
    this.ensureVolumeClear();
    this.updateCamera();
  }

  ensureVolumeClear() {
    const half = PLAYER_WIDTH / 2;
    for (let i = 0; i < 6; i++) {
      if (this.world.isVolumeClear(this.position.x, this.position.y, this.position.z, half, PLAYER_HEIGHT)) {
        return;
      }
      if (!this.resolvePenetration(half)) {
        this.position.y += 1;
      }
    }
  }

  isPointBlocked(x, y, z, radius = 0.12) {
    const minX = Math.floor(x - radius);
    const maxX = Math.floor(x + radius);
    const minY = Math.floor(y - radius);
    const maxY = Math.floor(y + radius);
    const minZ = Math.floor(z - radius);
    const maxZ = Math.floor(z + radius);

    for (let bx = minX; bx <= maxX; bx++) {
      for (let by = minY; by <= maxY; by++) {
        for (let bz = minZ; bz <= maxZ; bz++) {
          if (isSolid(this.world.peekBlock(bx, by, bz))) return true;
        }
      }
    }
    return false;
  }

  setupControls(domElement) {
    document.addEventListener('keydown', (e) => {
      if (e.code === 'KeyE') {
        this.onToggleInventory?.();
        return;
      }
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
      if (document.body.classList.contains('mobile')) return;
      if (!this.pointerLocked && !this.onInventoryOpen?.()) {
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
      if (e.button === 0) this.onPrimaryAction?.();
      if (e.button === 2) this.onPlaceBlock?.();
    });

    document.addEventListener('contextmenu', (e) => e.preventDefault());

    document.addEventListener('wheel', (e) => {
      if (!this.pointerLocked) return;
      this.hotbarIndex = (this.hotbarIndex + (e.deltaY > 0 ? 1 : -1) + 9) % 9;
      this.updateSelectedBlock();
    });
  }

  addLookDelta(dx, dy) {
    this.yaw += dx;
    this.pitch += dy;
    this.pitch = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, this.pitch));
  }

  isControlling() {
    return this.pointerLocked || this.mobileActive;
  }

  updateSelectedBlock() {
    const itemId = this.inventory.getHotbarItem(this.hotbarIndex);
    if (itemId && isBlockItem(itemId)) {
      this.selectedBlock = itemId;
    }
    this.onHotbarChange?.(this.hotbarIndex, this.selectedBlock);
  }

  getForward() {
    return new THREE.Vector3(-Math.sin(this.yaw), 0, -Math.cos(this.yaw));
  }

  getRight() {
    return new THREE.Vector3(Math.cos(this.yaw), 0, -Math.sin(this.yaw));
  }

  updateCamera() {
    const feet = this.position;
    const head = this._headPos;
    head.set(feet.x, feet.y + EYE_HEIGHT, feet.z);

    if (this.isPointBlocked(head.x, head.y, head.z)) {
      let placed = false;
      for (let t = 0.85; t >= 0.3; t -= 0.1) {
        head.set(feet.x, feet.y + EYE_HEIGHT * t, feet.z);
        if (!this.isPointBlocked(head.x, head.y, head.z)) {
          placed = true;
          break;
        }
      }
      if (!placed) {
        head.set(feet.x, feet.y + 0.45, feet.z);
      }
    }

    this.camera.position.copy(head);
    const lookDir = new THREE.Vector3(
      -Math.sin(this.yaw) * Math.cos(this.pitch),
      Math.sin(this.pitch),
      -Math.cos(this.yaw) * Math.cos(this.pitch)
    );
    this._lookTarget.copy(head).add(lookDir);
    this.camera.lookAt(this._lookTarget);
  }

  getLookDirection() {
    const direction = new THREE.Vector3();
    this.camera.getWorldDirection(direction);
    return direction;
  }

  raycast(maxDistance = 6) {
    const origin = this.camera.position.clone();
    const direction = this.getLookDirection();

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

    let prevX = x;
    let prevY = y;
    let prevZ = z;
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
    if (this.attackCooldown > 0) this.attackCooldown -= dt;

    if (!this.isControlling()) {
      this.updateCamera();
      return;
    }

    const sprint = this.keys['ShiftLeft'] || this.keys['ShiftRight'] || this.touchSprint;
    const speed = sprint ? SPRINT_SPEED : WALK_SPEED;
    const forward = this.getForward();
    const right = this.getRight();
    const move = new THREE.Vector3();

    if (this.keys['KeyW']) move.add(forward);
    if (this.keys['KeyS']) move.sub(forward);
    if (this.keys['KeyA']) move.sub(right);
    if (this.keys['KeyD']) move.add(right);

    if (this.touchMove.x !== 0 || this.touchMove.z !== 0) {
      move.add(forward.clone().multiplyScalar(-this.touchMove.z));
      move.add(right.clone().multiplyScalar(this.touchMove.x));
    }

    if (move.length() > 0) {
      move.normalize().multiplyScalar(speed);
    }

    this.velocity.x = move.x;
    this.velocity.z = move.z;

    if (this.onGround && (this.keys['Space'] || this.touchJump)) {
      this.velocity.y = JUMP_VELOCITY;
      this.onGround = false;
    }

    if (this.position.y < -10) {
      this.spawn();
      return;
    }

    this.checkLavaDamage(dt);

    this.velocity.y += GRAVITY * dt;
    this.moveWithCollision(dt);
    this.updateCamera();
    this.onMove?.();
  }

  checkLavaDamage(dt) {
    const px = Math.floor(this.position.x);
    const py = Math.floor(this.position.y);
    const pz = Math.floor(this.position.z);
    let inLava = false;

    for (let y = py; y < py + 2; y++) {
      if (isLava(this.world.getBlock(px, y, pz))) {
        inLava = true;
        break;
      }
    }

    if (!inLava) {
      this.lavaDamageTimer = 0;
      return;
    }

    this.velocity.y = Math.max(this.velocity.y, -3);

    this.lavaDamageTimer += dt;
    if (this.lavaDamageTimer >= 0.5) {
      this.lavaDamageTimer = 0;
      this.health -= 4;
      if (this.health <= 0) {
        this.health = this.maxHealth;
        this.spawn();
      }
    }
  }

  moveWithCollision(dt) {
    const half = PLAYER_WIDTH / 2;

    if (!this.world.isVolumeClear(this.position.x, this.position.y, this.position.z, half, PLAYER_HEIGHT)) {
      for (let i = 0; i < 4 && this.resolvePenetration(half); i++) {}
    }

    this.position.x += this.velocity.x * dt;
    this.collideAxis('x', half);

    this.position.z += this.velocity.z * dt;
    this.collideAxis('z', half);

    this.position.y += this.velocity.y * dt;
    this.onGround = false;
    this.collideAxis('y', half);
  }

  resolvePenetration(half) {
    const minX = Math.floor(this.position.x - half);
    const maxX = Math.floor(this.position.x + half);
    const minY = Math.floor(this.position.y);
    const maxY = Math.floor(this.position.y + PLAYER_HEIGHT);
    const minZ = Math.floor(this.position.z - half);
    const maxZ = Math.floor(this.position.z + half);

    for (let x = minX; x <= maxX; x++) {
      for (let y = minY; y <= maxY; y++) {
        for (let z = minZ; z <= maxZ; z++) {
          if (!isSolid(this.world.peekBlock(x, y, z))) continue;

          const overlapLeft = (this.position.x + half) - x;
          const overlapRight = (x + 1) - (this.position.x - half);
          const overlapBottom = (this.position.y + PLAYER_HEIGHT) - y;
          const overlapTop = (y + 1) - this.position.y;
          const overlapFront = (this.position.z + half) - z;
          const overlapBack = (z + 1) - (this.position.z - half);

          const minOverlap = Math.min(
            overlapLeft,
            overlapRight,
            overlapBottom,
            overlapTop,
            overlapFront,
            overlapBack
          );

          if (minOverlap <= 0) continue;

          if (minOverlap === overlapLeft) this.position.x -= overlapLeft + 0.001;
          else if (minOverlap === overlapRight) this.position.x += overlapRight + 0.001;
          else if (minOverlap === overlapBottom) this.position.y -= overlapBottom + 0.001;
          else if (minOverlap === overlapTop) {
            this.position.y += overlapTop + 0.001;
            this.onGround = true;
          } else if (minOverlap === overlapFront) this.position.z -= overlapFront + 0.001;
          else this.position.z += overlapBack + 0.001;

          return true;
        }
      }
    }
    return false;
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
          if (!isSolid(this.world.peekBlock(x, y, z))) continue;

          if (axis === 'x') {
            if (this.velocity.x > 0) this.position.x = x - half - 0.001;
            else if (this.velocity.x < 0) this.position.x = x + 1 + half + 0.001;
            else continue;
            this.velocity.x = 0;
          } else if (axis === 'z') {
            if (this.velocity.z > 0) this.position.z = z - half - 0.001;
            else if (this.velocity.z < 0) this.position.z = z + 1 + half + 0.001;
            else continue;
            this.velocity.z = 0;
          } else {
            if (this.velocity.y > 0) {
              this.position.y = y - PLAYER_HEIGHT - 0.001;
            } else if (this.velocity.y < 0) {
              this.position.y = y + 1 + 0.001;
              this.onGround = true;
            } else continue;
            this.velocity.y = 0;
          }
          return;
        }
      }
    }
  }
}
