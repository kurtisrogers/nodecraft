pub struct NoiseGenerator {
    perm: [u8; 512],
}

const GRAD: [[f32; 2]; 8] = [
    [1.0, 1.0],
    [-1.0, 1.0],
    [1.0, -1.0],
    [-1.0, -1.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, 1.0],
    [0.0, -1.0],
];

impl NoiseGenerator {
    pub fn new(seed: u32) -> Self {
        let mut p = [0u8; 256];
        for (i, v) in p.iter_mut().enumerate() {
            *v = i as u8;
        }
        let mut s = seed;
        for i in (1..256).rev() {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let j = (s as usize) % (i + 1);
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        Self { perm }
    }

    fn perm_at(&self, i: usize) -> u8 {
        self.perm[i & 511]
    }

    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + t * (b - a)
    }

    fn grad2(hash: u8, x: f32, y: f32) -> f32 {
        let g = GRAD[(hash as usize) & 7];
        g[0] * x + g[1] * y
    }

    pub fn noise2d(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32 as usize & 255;
        let yi = y.floor() as i32 as usize & 255;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let u = Self::fade(xf);
        let v = Self::fade(yf);

        let aa = self.perm_at(self.perm_at(xi) as usize + yi) as usize;
        let ab = self.perm_at(self.perm_at(xi) as usize + yi + 1) as usize;
        let ba = self.perm_at(self.perm_at(xi + 1) as usize + yi) as usize;
        let bb = self.perm_at(self.perm_at(xi + 1) as usize + yi + 1) as usize;

        let x1 = Self::lerp(
            Self::grad2(aa as u8, xf, yf),
            Self::grad2(ba as u8, xf - 1.0, yf),
            u,
        );
        let x2 = Self::lerp(
            Self::grad2(ab as u8, xf, yf - 1.0),
            Self::grad2(bb as u8, xf - 1.0, yf - 1.0),
            u,
        );
        Self::lerp(x1, x2, v)
    }

    pub fn fbm(&self, x: f32, y: f32, octaves: u32, persistence: f32, lacunarity: f32) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max = 0.0;
        for _ in 0..octaves {
            total += self.noise2d(x * frequency, y * frequency) * amplitude;
            max += amplitude;
            amplitude *= persistence;
            frequency *= lacunarity;
        }
        total / max
    }

    pub fn continentalness(&self, world_x: i32, world_z: i32) -> f32 {
        let wx = world_x as f32;
        let wz = world_z as f32;
        let macro_n = self.fbm(wx * 0.0032 + 180.0, wz * 0.0032 + 180.0, 5, 0.52, 2.0);
        let medium = self.fbm(wx * 0.0085, wz * 0.0085, 3, 0.45, 2.0) * 0.3;
        macro_n + medium
    }

    pub fn is_land(&self, world_x: i32, world_z: i32) -> bool {
        self.continentalness(world_x, world_z) > -0.08
    }

    pub fn is_shallow_ocean(&self, world_x: i32, world_z: i32) -> bool {
        let cont = self.continentalness(world_x, world_z);
        cont > -0.28 && cont <= -0.08
    }

    pub fn terrain_height(&self, world_x: i32, world_z: i32) -> i32 {
        let wx = world_x as f32;
        let wz = world_z as f32;
        let cont = self.continentalness(world_x, world_z);

        if cont < -0.28 {
            let depth = (cont + 1.0) * 0.5;
            return (12.0 + depth * 8.0).floor() as i32;
        }

        if cont < -0.08 {
            let shore = (cont + 0.28) / 0.2;
            let detail = self.fbm(wx * 0.025, wz * 0.025, 3, 0.4, 2.0) * 3.0;
            return (18.0 + shore * 16.0 + detail).floor() as i32;
        }

        let land_factor = ((cont + 0.08) / 0.45).min(1.0);
        let gentle_hills = self.fbm(wx * 0.006, wz * 0.006, 4, 0.5, 2.0) * 6.0;
        let micro = self.fbm(wx * 0.03, wz * 0.03, 2, 0.4, 2.0) * 1.5;
        let mut height = 34.0 + land_factor * 10.0 + gentle_hills + micro;

        let settle = self.settlement_at(world_x, world_z);
        if settle > 0.2 {
            let flat_target = 36.0 + gentle_hills * 0.4;
            height = height * (1.0 - settle * 0.85) + flat_target * (settle * 0.85);
        }

        height.clamp(28.0, 48.0).floor() as i32
    }

    pub fn biome(&self, world_x: i32, world_z: i32) -> BiomeSample {
        let wx = world_x as f32;
        let wz = world_z as f32;
        BiomeSample {
            moisture: self.fbm(wx * 0.01 + 100.0, wz * 0.01 + 100.0, 3, 0.5, 2.0),
            temperature: self.fbm(wx * 0.008, wz * 0.008, 3, 0.5, 2.0),
        }
    }

    pub fn settlement_at(&self, world_x: i32, world_z: i32) -> f32 {
        const GRID: i32 = 192;
        let cell_x = world_x.div_euclid(GRID);
        let cell_z = world_z.div_euclid(GRID);
        let cell = self.fbm(cell_x as f32 * 0.85 + 500.0, cell_z as f32 * 0.85 + 500.0, 2, 0.5, 2.0);
        if cell < 0.15 {
            return 0.0;
        }
        let lx = world_x.rem_euclid(GRID);
        let lz = world_z.rem_euclid(GRID);
        let dx = lx.min(GRID - lx) as f32 / (GRID as f32 * 0.5);
        let dz = lz.min(GRID - lz) as f32 / (GRID as f32 * 0.5);
        let edge = dx.min(dz);
        (edge * ((cell - 0.15) / 0.35)).clamp(0.0, 1.0)
    }

    pub fn settlement_center_near(&self, world_x: i32, world_z: i32, search_radius: i32) -> Option<(i32, i32)> {
        const GRID: i32 = 192;
        let cx0 = world_x.div_euclid(GRID);
        let cz0 = world_z.div_euclid(GRID);
        let mut best: Option<(i32, i32, f32)> = None;

        for dcx in -1..=1 {
            for dcz in -1..=1 {
                let cell_x = cx0 + dcx;
                let cell_z = cz0 + dcz;
                let cell = self.fbm(cell_x as f32 * 0.85 + 500.0, cell_z as f32 * 0.85 + 500.0, 2, 0.5, 2.0);
                if cell < 0.15 {
                    continue;
                }
                let center_x = cell_x * GRID + GRID / 2;
                let center_z = cell_z * GRID + GRID / 2;
                let dist = (((center_x - world_x).pow(2) + (center_z - world_z).pow(2)) as f32).sqrt();
                if dist <= search_radius as f32 {
                    if best.map(|b| dist < b.2).unwrap_or(true) {
                        best = Some((center_x, center_z, dist));
                    }
                }
            }
        }
        best.map(|(x, z, _)| (x, z))
    }

    pub fn is_in_settlement(&self, world_x: i32, world_z: i32) -> bool {
        self.settlement_at(world_x, world_z) > 0.35
    }

    pub fn roll(&self, world_x: i32, world_z: i32, salt: i32) -> f32 {
        let h = (world_x as u32)
            .wrapping_mul(374761393)
            .wrapping_add((world_z as u32).wrapping_mul(668265263))
            .wrapping_add((salt as u32).wrapping_mul(1013904223));
        let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        ((h ^ (h >> 16)) % 10000) as f32 / 10000.0
    }

    pub fn is_volcanic(&self, world_x: i32, world_z: i32) -> bool {
        if !self.is_land(world_x, world_z) {
            return false;
        }
        let b = self.biome(world_x, world_z);
        b.temperature > 0.35 && b.moisture < 0.0
    }

    pub fn is_forest(&self, world_x: i32, world_z: i32) -> bool {
        if !self.is_land(world_x, world_z) {
            return false;
        }
        let b = self.biome(world_x, world_z);
        b.moisture > 0.05 && b.temperature > -0.2 && b.temperature < 0.35
    }

    pub fn is_meadow(&self, world_x: i32, world_z: i32) -> bool {
        if !self.is_land(world_x, world_z) {
            return false;
        }
        let b = self.biome(world_x, world_z);
        b.moisture > -0.05 && b.temperature > -0.1 && b.temperature < 0.3
    }

    pub fn should_place_tree(&self, world_x: i32, world_z: i32) -> bool {
        if !self.is_land(world_x, world_z) || self.is_in_settlement(world_x, world_z) {
            return false;
        }
        let b = self.biome(world_x, world_z);
        if b.temperature > 0.3 && b.moisture < -0.1 {
            return false;
        }
        if b.temperature < -0.3 {
            return false;
        }
        let roll = self.roll(world_x, world_z, 7);
        if self.is_forest(world_x, world_z) {
            roll < 0.18
        } else if self.is_meadow(world_x, world_z) {
            roll < 0.08
        } else {
            roll < 0.04
        }
    }

    pub fn should_place_bush(&self, world_x: i32, world_z: i32) -> bool {
        self.is_land(world_x, world_z) && self.roll(world_x, world_z, 13) < 0.07
    }

    pub fn should_place_tall_grass(&self, world_x: i32, world_z: i32) -> bool {
        self.is_land(world_x, world_z) && self.roll(world_x, world_z, 17) < 0.16
    }

    pub fn should_place_flower(&self, world_x: i32, world_z: i32) -> bool {
        self.is_land(world_x, world_z) && self.roll(world_x, world_z, 19) < 0.05
    }

    pub fn cave_density(&self, world_x: i32, y: i32, world_z: i32) -> f32 {
        let wx = world_x as f32 * 0.065;
        let wy = y as f32 * 0.085;
        let wz = world_z as f32 * 0.065;
        let layer_a = self.fbm(wx, wz + wy * 0.45, 3, 0.5, 2.0);
        let layer_b = self.fbm(wx + wy + 40.0, wz + 120.0, 3, 0.48, 2.0);
        layer_a + layer_b
    }

    pub fn is_cave(&self, world_x: i32, y: i32, world_z: i32, surface_y: i32) -> bool {
        if y <= 1 || y >= surface_y - 3 {
            return false;
        }
        self.cave_density(world_x, y, world_z) < 0.15
    }

    pub fn is_deep_cavern(&self, world_x: i32, y: i32, world_z: i32, surface_y: i32) -> bool {
        if y > 26 || y < 4 || y >= surface_y - 6 {
            return false;
        }
        self.cave_density(world_x, y, world_z) < -0.05
    }
}

#[derive(Clone, Copy)]
pub struct BiomeSample {
    pub moisture: f32,
    pub temperature: f32,
}
