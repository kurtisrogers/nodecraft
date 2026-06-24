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

  terrainHeight(worldX, worldZ) {
    const scale = 0.02;
    const height =
      this.fbm(worldX * scale, worldZ * scale, 5, 0.5, 2) * 24 +
      this.fbm(worldX * scale * 2, worldZ * scale * 2, 3, 0.4, 2) * 8;
    return Math.floor(32 + height);
  }

  biome(worldX, worldZ) {
    const moisture = this.fbm(worldX * 0.01 + 100, worldZ * 0.01 + 100, 3);
    const temperature = this.fbm(worldX * 0.008, worldZ * 0.008, 3);
    return { moisture, temperature };
  }

  treeChance(worldX, worldZ) {
    return this.fbm(worldX * 0.1 + 500, worldZ * 0.1 + 500, 2);
  }
}
