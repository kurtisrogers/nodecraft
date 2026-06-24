import { MessageType } from './shared/protocol.js';

const BlockId = {
  AIR: 0,
  BEDROCK: 8,
};

const MOB_DEFS = {
  pig: { health: 10, hostile: false, speed: 2, drops: [{ itemId: 101, count: 1, chance: 1 }] },
  cow: { health: 10, hostile: false, speed: 1.5, drops: [{ itemId: 104, count: 1, chance: 1 }, { itemId: 102, count: 1, chance: 0.5 }] },
  zombie: { health: 20, hostile: true, speed: 3.5, drops: [{ itemId: 103, count: 1, chance: 0.5 }] },
};

export class GameServer {
  constructor(seed = 42) {
    this.seed = seed;
    this.players = new Map();
    this.blockChanges = new Map();
    this.mobs = new Map();
    this.nextMobId = 1;
    this.nextPlayerId = 1;
    this.dayTime = 0;
    this.spawnTimer = 0;
  }

  blockKey(x, y, z) {
    return `${x},${y},${z}`;
  }

  addPlayer(name) {
    const id = `p${this.nextPlayerId++}`;
    const player = {
      id,
      name,
      x: 0.5,
      y: 40,
      z: 0.5,
      yaw: 0,
      pitch: 0,
    };
    this.players.set(id, player);
    return player;
  }

  removePlayer(id) {
    this.players.delete(id);
  }

  updatePlayer(id, data) {
    const player = this.players.get(id);
    if (!player) return;
    Object.assign(player, data);
  }

  setBlock(x, y, z, blockId) {
    const key = this.blockKey(x, y, z);
    if (blockId === BlockId.AIR) {
      this.blockChanges.delete(key);
    } else {
      this.blockChanges.set(key, blockId);
    }
    return { x, y, z, blockId };
  }

  getBlockChanges() {
    const changes = [];
    for (const [key, blockId] of this.blockChanges) {
      const [x, y, z] = key.split(',').map(Number);
      changes.push({ x, y, z, blockId });
    }
    return changes;
  }

  get isNight() {
    const cycle = (this.dayTime % 120) / 120;
    return cycle > 0.5;
  }

  spawnMob(type, x, y, z) {
    const id = this.nextMobId++;
    const def = MOB_DEFS[type];
    const mob = {
      id,
      type,
      x,
      y,
      z,
      health: def.health,
      alive: true,
      wanderTimer: 2,
      wanderAngle: Math.random() * Math.PI * 2,
      vx: 0,
      vz: 0,
      vy: 0,
    };
    this.mobs.set(id, mob);
    return mob;
  }

  trySpawnMob(nearX, nearZ) {
    if (this.mobs.size >= 30) return;
    const angle = Math.random() * Math.PI * 2;
    const dist = 15 + Math.random() * 20;
    const x = nearX + Math.cos(angle) * dist;
    const z = nearZ + Math.sin(angle) * dist;
    const y = 35;

    const types = this.isNight ? ['zombie', 'pig', 'cow'] : ['pig', 'cow'];
    const type = types[Math.floor(Math.random() * types.length)];
    this.spawnMob(type, x, y, z);
  }

  updateMobs(dt) {
    this.dayTime += dt;
    this.spawnTimer -= dt;
    if (this.spawnTimer <= 0) {
      this.spawnTimer = 5;
      if (Math.random() < 0.5 && this.players.size > 0) {
        const player = [...this.players.values()][0];
        this.trySpawnMob(player.x, player.z);
      }
    }

    for (const mob of this.mobs.values()) {
      if (!mob.alive) continue;
      const def = MOB_DEFS[mob.type];

      mob.wanderTimer -= dt;
      let mx = 0;
      let mz = 0;

      if (def.hostile && this.isNight && this.players.size > 0) {
        let closest = Infinity;
        let target = null;
        for (const player of this.players.values()) {
          const dx = player.x - mob.x;
          const dz = player.z - mob.z;
          const dist = Math.sqrt(dx * dx + dz * dz);
          if (dist < closest) {
            closest = dist;
            target = player;
          }
        }
        if (target && closest < 24 && closest > 0.5) {
          mx = (target.x - mob.x) / closest;
          mz = (target.z - mob.z) / closest;
        }
      } else if (mob.wanderTimer <= 0) {
        mob.wanderTimer = 2 + Math.random() * 3;
        mob.wanderAngle = Math.random() * Math.PI * 2;
      }

      if (mx === 0 && mz === 0) {
        mx = Math.cos(mob.wanderAngle);
        mz = Math.sin(mob.wanderAngle);
      }

      mob.vx = mx * def.speed;
      mob.vz = mz * def.speed;
      mob.vy -= 20 * dt;
      mob.x += mob.vx * dt;
      mob.z += mob.vz * dt;
      mob.y += mob.vy * dt;

      if (mob.y < 20) {
        mob.y = 35;
        mob.vy = 0;
      }
    }

    for (const [id, mob] of this.mobs) {
      if (!mob.alive) this.mobs.delete(id);
    }
  }

  attackMob(mobId, damage = 5) {
    const mob = this.mobs.get(mobId);
    if (!mob || !mob.alive) return null;
    mob.health -= damage;
    if (mob.health <= 0) {
      mob.alive = false;
      const def = MOB_DEFS[mob.type];
      const drops = [];
      for (const drop of def.drops) {
        if (Math.random() <= drop.chance) {
          drops.push({ itemId: drop.itemId, count: drop.count });
        }
      }
      return { killed: true, drops, mob: { ...mob } };
    }
    return { killed: false, mob: { ...mob } };
  }

  getState() {
    return {
      seed: this.seed,
      players: [...this.players.values()],
      blockChanges: this.getBlockChanges(),
      mobs: [...this.mobs.values()],
      dayTime: this.dayTime,
    };
  }

  welcomePayload(playerId) {
    return {
      playerId,
      seed: this.seed,
      players: [...this.players.values()],
      blockChanges: this.getBlockChanges(),
      mobs: [...this.mobs.values()],
      dayTime: this.dayTime,
    };
  }
}

export function handleMessage(server, ws, clientId, msg, broadcast) {
  switch (msg.type) {
    case MessageType.JOIN: {
      const player = server.addPlayer(msg.name || 'Player');
      ws.clientId = player.id;
      ws.send(JSON.stringify({ type: MessageType.WELCOME, ...server.welcomePayload(player.id) }));
      broadcast({ type: MessageType.PLAYER_JOIN, player }, player.id);
      break;
    }
    case MessageType.MOVE: {
      server.updatePlayer(clientId, {
        x: msg.x,
        y: msg.y,
        z: msg.z,
        yaw: msg.yaw,
        pitch: msg.pitch,
      });
      broadcast({ type: MessageType.PLAYER_MOVE, id: clientId, x: msg.x, y: msg.y, z: msg.z, yaw: msg.yaw, pitch: msg.pitch }, clientId);
      break;
    }
    case MessageType.BREAK_BLOCK: {
      if (msg.y === 0) break;
      const change = server.setBlock(msg.x, msg.y, msg.z, BlockId.AIR);
      broadcast({ type: MessageType.BLOCK_CHANGE, ...change });
      break;
    }
    case MessageType.PLACE_BLOCK: {
      const change = server.setBlock(msg.x, msg.y, msg.z, msg.blockId);
      broadcast({ type: MessageType.BLOCK_CHANGE, ...change });
      break;
    }
    case MessageType.ATTACK_MOB: {
      const result = server.attackMob(msg.mobId);
      if (!result) break;
      const mobData = { ...result.mob, alive: !result.killed };
      if (result.killed) {
        server.mobs.delete(msg.mobId);
        mobData.alive = false;
      }
      broadcast({
        type: MessageType.MOB_UPDATE,
        mob: mobData,
        drops: result.killed ? result.drops : undefined,
      });
      break;
    }
    default:
      break;
  }
}
