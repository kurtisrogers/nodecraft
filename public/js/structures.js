import { BlockId } from './blocks.js';

/** Minecraft-style wooden house with door opening and glass windows. */
export function placeHouse(world, originX, originY, originZ) {
  const w = 7;
  const d = 7;
  const h = 4;

  for (let dy = 0; dy < h; dy++) {
    for (let dx = 0; dx < w; dx++) {
      for (let dz = 0; dz < d; dz++) {
        const wx = originX + dx;
        const wy = originY + dy;
        const wz = originZ + dz;
        const edgeX = dx === 0 || dx === w - 1;
        const edgeZ = dz === 0 || dz === d - 1;
        const isWall = edgeX || edgeZ;
        const isRoof = dy === h - 1;

        if (!isWall && !isRoof) continue;

        if (isRoof) {
          world.setBlock(wx, wy, wz, BlockId.WOOD);
          continue;
        }

        const doorX = dx === Math.floor(w / 2);
        const doorZ = dz === 0;
        if (doorZ && doorX && dy < 2) continue;

        const windowY = dy === 2;
        const windowX = edgeX && (dx === 1 || dx === w - 2);
        const windowZ = edgeZ && (dz === 1 || dz === d - 2);
        if (windowY && (windowX || windowZ)) {
          world.setBlock(wx, wy, wz, BlockId.GLASS);
          continue;
        }

        world.setBlock(wx, wy, wz, BlockId.PLANKS);
      }
    }
  }
}

/** Tilled soil ring with wheat — classic village crop plot. */
export function placeFarmPlot(world, originX, originY, originZ, size = 5) {
  for (let dx = 0; dx < size; dx++) {
    for (let dz = 0; dz < size; dz++) {
      const wx = originX + dx;
      const wz = originZ + dz;
      const edge = dx === 0 || dz === 0 || dx === size - 1 || dz === size - 1;
      if (edge) {
        world.setBlock(wx, originY, wz, BlockId.DIRT);
      } else if ((dx + dz) % 2 === 0) {
        world.setBlock(wx, originY, wz, BlockId.WHEAT);
      }
    }
  }
}

/** Flatten terrain under a settlement footprint. */
export function flattenArea(world, centerX, centerZ, radius, targetY) {
  for (let dx = -radius; dx <= radius; dx++) {
    for (let dz = -radius; dz <= radius; dz++) {
      const wx = centerX + dx;
      const wz = centerZ + dz;
      const dist = Math.sqrt(dx * dx + dz * dz);
      if (dist > radius) continue;

      for (let y = 0; y < 64; y++) {
        if (y < targetY) {
          const cur = world.peekBlock(wx, y, wz);
          if (
            cur === BlockId.AIR ||
            cur === BlockId.WATER ||
            cur === BlockId.LEAVES ||
            cur === BlockId.TALL_GRASS ||
            cur === BlockId.FLOWER ||
            cur === BlockId.WHEAT
          ) {
            world.setBlock(wx, y, wz, BlockId.DIRT);
          }
        } else if (y === targetY) {
          const cur = world.peekBlock(wx, y, wz);
          if (cur !== BlockId.WATER && cur !== BlockId.LAVA) {
            world.setBlock(wx, y, wz, BlockId.GRASS);
          }
        } else if (y > targetY) {
          const cur = world.peekBlock(wx, y, wz);
          if (
            cur !== BlockId.AIR &&
            cur !== BlockId.WATER &&
            cur !== BlockId.LAVA
          ) {
            world.setBlock(wx, y, wz, BlockId.AIR);
          }
        }
      }
    }
  }
}

export function placeSettlement(world, centerX, centerZ, surfaceY) {
  flattenArea(world, centerX, centerZ, 22, surfaceY);

  placeHouse(world, centerX - 10, surfaceY + 1, centerZ - 6);
  placeHouse(world, centerX + 2, surfaceY + 1, centerZ - 8);
  placeHouse(world, centerX - 4, surfaceY + 1, centerZ + 4);

  placeFarmPlot(world, centerX + 6, surfaceY + 1, centerZ + 2, 5);
  placeFarmPlot(world, centerX - 14, surfaceY + 1, centerZ + 4, 4);
}
