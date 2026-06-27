// Simplex-style 2D noise for terrain generation
const PERM = new Uint8Array(512);
const GRAD = [
  [1, 1], [-1, 1], [1, -1], [-1, -1],
  [1, 0], [-1, 0], [0, 1], [0, -1],
];

function seedPermutation(seed) {
  const p = new Uint8Array(256);
  for (let i = 0; i < 256; i++) p[i] = i;
  let s = seed | 0;
  for (let i = 255; i > 0; i--) {
    s = (s * 1664525 + 1013904223) | 0;
    const j = (s >>> 0) % (i + 1);
    [p[i], p[j]] = [p[j], p[i]];
  }
  for (let i = 0; i < 512; i++) PERM[i] = p[i & 255];
}

function fade(t) {
  return t * t * t * (t * (t * 6 - 15) + 10);
}

function lerp(a, b, t) {
  return a + t * (b - a);
}

function grad2(hash, x, y) {
  const g = GRAD[hash & 7];
  return g[0] * x + g[1] * y;
}

function noise2D(x, y) {
  const xi = Math.floor(x) & 255;
  const yi = Math.floor(y) & 255;
  const xf = x - Math.floor(x);
  const yf = y - Math.floor(y);
  const u = fade(xf);
  const v = fade(yf);

  const aa = PERM[PERM[xi] + yi];
  const ab = PERM[PERM[xi] + yi + 1];
  const ba = PERM[PERM[xi + 1] + yi];
  const bb = PERM[PERM[xi + 1] + yi + 1];

  const x1 = lerp(grad2(aa, xf, yf), grad2(ba, xf - 1, yf), u);
  const x2 = lerp(grad2(ab, xf, yf - 1), grad2(bb, xf - 1, yf - 1), u);
  return lerp(x1, x2, v);
}

export class NoiseGenerator {
  constructor(seed = 42) {
    this.seed = seed;
    seedPermutation(seed);
  }

  fbm(x, y, octaves = 4, persistence = 0.5, lacunarity = 2) {
    let total = 0;
    let amplitude = 1;
    let frequency = 1;
    let max = 0;
    for (let i = 0; i < octaves; i++) {
      total += noise2D(x * frequency, y * frequency) * amplitude;
      max += amplitude;
      amplitude *= persistence;
      frequency *= lacunarity;
    }
    return total / max;
  }

  continentalness(worldX, worldZ) {
    const macro = this.fbm(worldX * 0.0032 + 180, worldZ * 0.0032 + 180, 5, 0.52, 2);
    const medium = this.fbm(worldX * 0.0085, worldZ * 0.0085, 3, 0.45, 2) * 0.3;
    return macro + medium;
  }

  isLand(worldX, worldZ) {
    return this.continentalness(worldX, worldZ) > -0.08;
  }

  isShallowOcean(worldX, worldZ) {
    const cont = this.continentalness(worldX, worldZ);
    return cont > -0.28 && cont <= -0.08;
  }

  terrainHeight(worldX, worldZ) {
    const cont = this.continentalness(worldX, worldZ);

    if (cont < -0.28) {
      const depth = (cont + 1) * 0.5;
      return Math.floor(12 + depth * 8);
    }

    if (cont < -0.08) {
      const shore = (cont + 0.28) / 0.2;
      const detail = this.fbm(worldX * 0.025, worldZ * 0.025, 3, 0.4, 2) * 3;
      return Math.floor(18 + shore * 16 + detail);
    }

    const landFactor = Math.min(1, (cont + 0.08) / 0.45);
    const gentleHills = this.fbm(worldX * 0.006, worldZ * 0.006, 4, 0.5, 2) * 6;
    const micro = this.fbm(worldX * 0.03, worldZ * 0.03, 2, 0.4, 2) * 1.5;
    let height = 34 + landFactor * 10 + gentleHills + micro;

    const settleStrength = this.settlementAt(worldX, worldZ);
    if (settleStrength > 0.2) {
      const flatTarget = 36 + gentleHills * 0.4;
      height = height * (1 - settleStrength * 0.85) + flatTarget * (settleStrength * 0.85);
    }

    return Math.floor(Math.max(28, Math.min(48, height)));
  }

  settlementAt(worldX, worldZ) {
    const grid = 192;
    const cellX = Math.floor(worldX / grid);
    const cellZ = Math.floor(worldZ / grid);
    const cell = this.fbm(cellX * 0.85 + 500, cellZ * 0.85 + 500, 2);
    if (cell < 0.15) return 0;

    const lx = ((worldX % grid) + grid) % grid;
    const lz = ((worldZ % grid) + grid) % grid;
    const dx = Math.min(lx, grid - lx) / (grid * 0.5);
    const dz = Math.min(lz, grid - lz) / (grid * 0.5);
    const edge = Math.min(dx, dz);
    return Math.max(0, Math.min(1, edge * ((cell - 0.15) / 0.35)));
  }

  settlementCenterNear(worldX, worldZ, searchRadius = 256) {
    const grid = 192;
    const cx0 = Math.floor(worldX / grid);
    const cz0 = Math.floor(worldZ / grid);
    let best = null;
    let bestDist = Infinity;

    for (let dcx = -1; dcx <= 1; dcx++) {
      for (let dcz = -1; dcz <= 1; dcz++) {
        const cellX = cx0 + dcx;
        const cellZ = cz0 + dcz;
        const cell = this.fbm(cellX * 0.85 + 500, cellZ * 0.85 + 500, 2);
        if (cell < 0.15) continue;
        const centerX = cellX * grid + grid / 2;
        const centerZ = cellZ * grid + grid / 2;
        const dist = Math.hypot(centerX - worldX, centerZ - worldZ);
        if (dist < bestDist && dist <= searchRadius) {
          bestDist = dist;
          best = { x: centerX, z: centerZ, strength: cell };
        }
      }
    }
    return best;
  }

  isInSettlement(worldX, worldZ) {
    return this.settlementAt(worldX, worldZ) > 0.35;
  }

  biome(worldX, worldZ) {
    const moisture = this.fbm(worldX * 0.01 + 100, worldZ * 0.01 + 100, 3);
    const temperature = this.fbm(worldX * 0.008, worldZ * 0.008, 3);
    return { moisture, temperature };
  }

  hash2D(x, z) {
    let h = (x * 374761393 + z * 668265263 + this.seed * 1013904223) | 0;
    h = ((h ^ (h >>> 13)) * 1274126177) | 0;
    return (h ^ (h >>> 16)) >>> 0;
  }

  roll(worldX, worldZ, salt = 0) {
    return (this.hash2D(worldX + salt * 997, worldZ + salt * 131) % 10000) / 10000;
  }

  isVolcanic(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    const biome = this.biome(worldX, worldZ);
    return biome.temperature > 0.35 && biome.moisture < 0;
  }

  isForest(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    const biome = this.biome(worldX, worldZ);
    return biome.moisture > 0.05 && biome.temperature > -0.2 && biome.temperature < 0.35;
  }

  isMeadow(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    const biome = this.biome(worldX, worldZ);
    return biome.moisture > -0.05 && biome.temperature > -0.1 && biome.temperature < 0.3;
  }

  shouldPlaceTree(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    if (this.isInSettlement(worldX, worldZ)) return false;
    const biome = this.biome(worldX, worldZ);
    const isDesert = biome.temperature > 0.3 && biome.moisture < -0.1;
    const isSnow = biome.temperature < -0.3;
    if (isDesert || isSnow) return false;

    const roll = this.roll(worldX, worldZ, 7);
    if (this.isForest(worldX, worldZ)) return roll < 0.18;
    if (this.isMeadow(worldX, worldZ)) return roll < 0.08;
    return roll < 0.04;
  }

  shouldPlaceBush(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    return this.roll(worldX, worldZ, 13) < 0.07;
  }

  shouldPlaceTallGrass(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    return this.roll(worldX, worldZ, 17) < 0.16;
  }

  shouldPlaceFlower(worldX, worldZ) {
    if (!this.isLand(worldX, worldZ)) return false;
    return this.roll(worldX, worldZ, 19) < 0.05;
  }

  lavaPoolChance(worldX, worldZ) {
    return this.fbm(worldX * 0.06 + 2000, worldZ * 0.06 + 2000, 3);
  }

  caveDensity(worldX, y, worldZ) {
    const wx = worldX * 0.065;
    const wy = y * 0.085;
    const wz = worldZ * 0.065;
    const layerA = this.fbm(wx, wz + wy * 0.45, 3, 0.5, 2);
    const layerB = this.fbm(wx + wy + 40, wz + 120, 3, 0.48, 2);
    return layerA + layerB;
  }

  isCave(worldX, y, worldZ, surfaceY) {
    if (y <= 1 || y >= surfaceY - 3) return false;
    return this.caveDensity(worldX, y, worldZ) < 0.15;
  }

  isDeepCavern(worldX, y, worldZ, surfaceY) {
    if (y > 26 || y < 4 || y >= surfaceY - 6) return false;
    return this.caveDensity(worldX, y, worldZ) < -0.05;
  }
}
