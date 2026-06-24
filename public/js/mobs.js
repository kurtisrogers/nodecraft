import * as THREE from 'three';
import { isSolid } from './blocks.js';
import { WORLD_HEIGHT } from './world.js';

export const MobType = {
  PIG: 'pig',
  COW: 'cow',
  ZOMBIE: 'zombie',
};

export const MOB_DEFS = {
  [MobType.PIG]: {
    name: 'Pig',
    health: 10,
    speed: 2,
    hostile: false,
    color: 0xffb6c1,
    size: { w: 0.9, h: 0.9, d: 1.4 },
    drops: [{ itemId: 101, count: 1, chance: 1 }],
  },
  [MobType.COW]: {
    name: 'Cow',
    health: 10,
    speed: 1.5,
    hostile: false,
    color: 0x8b7355,
    size: { w: 1.0, h: 1.4, d: 1.6 },
    drops: [{ itemId: 104, count: 1, chance: 1 }, { itemId: 102, count: 1, chance: 0.5 }],
  },
  [MobType.ZOMBIE]: {
    name: 'Zombie',
    health: 20,
    speed: 3.5,
    hostile: true,
    color: 0x4a7c4e,
    size: { w: 0.6, h: 1.8, d: 0.6 },
    drops: [{ itemId: 103, count: 1, chance: 0.5 }],
  },
};

function createMobMesh(type) {
  const def = MOB_DEFS[type];
  const group = new THREE.Group();

  const bodyGeo = new THREE.BoxGeometry(def.size.w, def.size.h * 0.6, def.size.d);
  const bodyMat = new THREE.MeshLambertMaterial({ color: def.color });
  const body = new THREE.Mesh(bodyGeo, bodyMat);
  body.position.y = def.size.h * 0.3;
  group.add(body);

  const headGeo = new THREE.BoxGeometry(def.size.w * 0.7, def.size.h * 0.35, def.size.w * 0.7);
  const headMat = new THREE.MeshLambertMaterial({
    color: type === MobType.ZOMBIE ? 0x5a8a5e : def.color,
  });
  const head = new THREE.Mesh(headGeo, headMat);
  head.position.y = def.size.h * 0.75;
  head.position.z = def.size.d * 0.15;
  group.add(head);

  if (type === MobType.PIG) {
    const snout = new THREE.Mesh(
      new THREE.BoxGeometry(0.2, 0.15, 0.15),
      new THREE.MeshLambertMaterial({ color: 0xff9999 })
    );
    snout.position.set(0, def.size.h * 0.7, def.size.d * 0.45);
    group.add(snout);
  }

  group.userData.mobType = type;
  return group;
}

export class Mob {
  constructor(id, type, x, y, z) {
    this.id = id;
    this.type = type;
    this.position = new THREE.Vector3(x, y, z);
    this.velocity = new THREE.Vector3();
    this.health = MOB_DEFS[type].health;
    this.maxHealth = MOB_DEFS[type].health;
    this.alive = true;
    this.wanderTimer = 0;
    this.wanderDir = new THREE.Vector3();
    this.target = null;
    this.mesh = createMobMesh(type);
    this.mesh.position.copy(this.position);
  }

  getDef() {
    return MOB_DEFS[this.type];
  }

  takeDamage(amount) {
    this.health -= amount;
    if (this.health <= 0) {
      this.alive = false;
      return true;
    }
    return false;
  }

  update(dt, world, playerPos, isNight) {
    if (!this.alive) return;

    const def = this.getDef();
    let moveDir = new THREE.Vector3();

    if (def.hostile && isNight) {
      const toPlayer = playerPos.clone().sub(this.position);
      toPlayer.y = 0;
      const dist = toPlayer.length();
      if (dist < 24 && dist > 0.5) {
        moveDir = toPlayer.normalize();
      }
    } else {
      this.wanderTimer -= dt;
      if (this.wanderTimer <= 0) {
        this.wanderTimer = 2 + Math.random() * 3;
        const angle = Math.random() * Math.PI * 2;
        this.wanderDir.set(Math.cos(angle), 0, Math.sin(angle));
      }
      moveDir = this.wanderDir;
    }

    this.velocity.x = moveDir.x * def.speed;
    this.velocity.z = moveDir.z * def.speed;
    this.velocity.y -= 20 * dt;

    this.position.x += this.velocity.x * dt;
    this.collideAxis(world, 'x');
    this.position.z += this.velocity.z * dt;
    this.collideAxis(world, 'z');
    this.position.y += this.velocity.y * dt;
    this.collideAxis(world, 'y');

    this.mesh.position.copy(this.position);
    if (moveDir.lengthSq() > 0.01) {
      this.mesh.rotation.y = Math.atan2(moveDir.x, moveDir.z);
    }
  }

  collideAxis(world, axis) {
    const def = this.getDef();
    const hw = def.size.w / 2;
    const hd = def.size.d / 2;
    const minX = Math.floor(this.position.x - hw);
    const maxX = Math.floor(this.position.x + hw);
    const minY = Math.floor(this.position.y);
    const maxY = Math.floor(this.position.y + def.size.h);
    const minZ = Math.floor(this.position.z - hd);
    const maxZ = Math.floor(this.position.z + hd);

    for (let x = minX; x <= maxX; x++) {
      for (let y = minY; y <= maxY; y++) {
        for (let z = minZ; z <= maxZ; z++) {
          if (!isSolid(world.getBlock(x, y, z))) continue;
          if (axis === 'x') {
            if (this.velocity.x > 0) this.position.x = x - hw - 0.01;
            else if (this.velocity.x < 0) this.position.x = x + 1 + hw + 0.01;
            this.velocity.x = 0;
          } else if (axis === 'z') {
            if (this.velocity.z > 0) this.position.z = z - hd - 0.01;
            else if (this.velocity.z < 0) this.position.z = z + 1 + hd + 0.01;
            this.velocity.z = 0;
          } else {
            if (this.velocity.y < 0) {
              this.position.y = y + 1;
              this.velocity.y = 0;
            } else if (this.velocity.y > 0) {
              this.position.y = y - def.size.h - 0.01;
              this.velocity.y = 0;
            }
          }
        }
      }
    }

    if (this.position.y < 0) this.position.y = WORLD_HEIGHT;
  }

  dispose(scene) {
    scene.remove(this.mesh);
    this.mesh.traverse((child) => {
      if (child.geometry) child.geometry.dispose();
      if (child.material) child.material.dispose();
    });
  }

  toJSON() {
    return {
      id: this.id,
      type: this.type,
      x: this.position.x,
      y: this.position.y,
      z: this.position.z,
      health: this.health,
      alive: this.alive,
    };
  }

  static fromJSON(data, scene) {
    const mob = new Mob(data.id, data.type, data.x, data.y, data.z);
    mob.health = data.health;
    mob.alive = data.alive;
    if (!mob.alive) {
      mob.dispose(scene);
      return null;
    }
    scene.add(mob.mesh);
    return mob;
  }
}

const DAY_LENGTH = 240;

export class MobManager {
  constructor(scene, world) {
    this.scene = scene;
    this.world = world;
    this.mobs = new Map();
    this.nextId = 1;
    this.spawnTimer = 0;
    this.dayTime = 0;
    this.authoritative = true;
  }

  get isNight() {
    const cycle = (this.dayTime % DAY_LENGTH) / DAY_LENGTH;
    return cycle > 0.5;
  }

  spawn(type, x, y, z) {
    const id = this.nextId++;
    const mob = new Mob(id, type, x, y, z);
    this.mobs.set(id, mob);
    this.scene.add(mob.mesh);
    return mob;
  }

  trySpawnNear(px, pz) {
    if (this.mobs.size >= 30) return;
    const angle = Math.random() * Math.PI * 2;
    const dist = 15 + Math.random() * 20;
    const x = px + Math.cos(angle) * dist;
    const z = pz + Math.sin(angle) * dist;
    const y = this.world.getSpawnHeight(Math.floor(x), Math.floor(z));

    const types = this.isNight
      ? [MobType.ZOMBIE, MobType.PIG, MobType.COW]
      : [MobType.PIG, MobType.COW];
    const weights = this.isNight ? [0.4, 0.3, 0.3] : [0.5, 0.5];
    const r = Math.random();
    let cumulative = 0;
    let type = types[0];
    for (let i = 0; i < types.length; i++) {
      cumulative += weights[i];
      if (r < cumulative) {
        type = types[i];
        break;
      }
    }

    this.spawn(type, x, y, z);
  }

  update(dt, playerPos) {
    this.dayTime += dt;
    if (this.authoritative) {
      this.spawnTimer -= dt;
      if (this.spawnTimer <= 0) {
        this.spawnTimer = 5;
        if (Math.random() < 0.6) this.trySpawnNear(playerPos.x, playerPos.z);
      }
    }

    for (const mob of this.mobs.values()) {
      if (mob.alive) {
        mob.update(dt, this.world, playerPos, this.isNight);
      }
    }

    for (const [id, mob] of this.mobs) {
      if (!mob.alive) {
        mob.dispose(this.scene);
        this.mobs.delete(id);
      }
    }
  }

  raycast(origin, direction, maxDist = 4) {
    let closest = null;
    let closestDist = maxDist;

    for (const mob of this.mobs.values()) {
      if (!mob.alive) continue;
      const def = mob.getDef();
      const center = mob.position.clone();
      center.y += def.size.h / 2;
      const toMob = center.clone().sub(origin);
      const proj = toMob.dot(direction);
      if (proj < 0 || proj > maxDist) continue;
      const closestPoint = origin.clone().add(direction.clone().multiplyScalar(proj));
      const dist = closestPoint.distanceTo(center);
      if (dist < Math.max(def.size.w, def.size.h) * 0.6 && proj < closestDist) {
        closestDist = proj;
        closest = mob;
      }
    }
    return closest;
  }

  attack(mobId, damage = 5) {
    const mob = this.mobs.get(mobId);
    if (!mob || !mob.alive) return null;
    const killed = mob.takeDamage(damage);
    return { mob, killed, drops: killed ? this.getDrops(mob) : [] };
  }

  getDrops(mob) {
    const drops = [];
    for (const drop of mob.getDef().drops) {
      if (Math.random() <= drop.chance) {
        drops.push({ itemId: drop.itemId, count: drop.count });
      }
    }
    return drops;
  }

  syncFromServer(mobDataList, scene) {
    const seen = new Set();
    for (const data of mobDataList) {
      seen.add(data.id);
      let mob = this.mobs.get(data.id);
      if (!data.alive) {
        if (mob) {
          mob.dispose(scene);
          this.mobs.delete(data.id);
        }
        continue;
      }
      if (!mob) {
        mob = Mob.fromJSON(data, scene);
        if (mob) this.mobs.set(data.id, mob);
      } else {
        mob.position.set(data.x, data.y, data.z);
        mob.health = data.health;
        mob.mesh.position.copy(mob.position);
      }
    }
    for (const [id, mob] of this.mobs) {
      if (!seen.has(id)) {
        mob.dispose(scene);
        this.mobs.delete(id);
      }
    }
    this.authoritative = false;
  }

  toJSON() {
    return [...this.mobs.values()].map((m) => m.toJSON());
  }
}
