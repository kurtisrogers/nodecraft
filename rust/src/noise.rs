use crate::config::SEA_LEVEL;

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

    /// Blend of axis-aligned 2D slices — good enough for underground cave worms.
    pub fn noise3d(&self, x: f32, y: f32, z: f32) -> f32 {
        let a = self.noise2d(x * 0.98 + 17.0, y * 0.98 + 41.0);
        let b = self.noise2d(y * 0.98 + 83.0, z * 0.98 + 29.0);
        let c = self.noise2d(z * 0.98 + 131.0, x * 0.98 + 59.0);
        (a + b + c) / 3.0
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

    pub fn fbm3d(
        &self,
        x: f32,
        y: f32,
        z: f32,
        octaves: u32,
        persistence: f32,
        lacunarity: f32,
    ) -> f32 {
        let mut total = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max = 0.0;
        for _ in 0..octaves {
            total += self.noise3d(x * frequency, y * frequency, z * frequency) * amplitude;
            max += amplitude;
            amplitude *= persistence;
            frequency *= lacunarity;
        }
        total / max
    }

    /// Large-scale land vs ocean — slow, smooth continents (Minecraft-style continentalness).
    pub fn continentalness(&self, world_x: i32, world_z: i32) -> f32 {
        let wx = world_x as f32;
        let wz = world_z as f32;
        let macro_n = self.fbm(wx * 0.0025 + 180.0, wz * 0.0025 + 180.0, 5, 0.5, 2.0);
        let medium = self.fbm(wx * 0.006, wz * 0.006, 3, 0.42, 2.0) * 0.22;
        macro_n + medium
    }

    pub fn is_land(&self, world_x: i32, world_z: i32) -> bool {
        self.continentalness(world_x, world_z) > -0.12
    }

    pub fn is_shallow_ocean(&self, world_x: i32, world_z: i32) -> bool {
        let cont = self.continentalness(world_x, world_z);
        cont > -0.32 && cont <= -0.12
    }

    pub fn is_beach(&self, world_x: i32, world_z: i32, height: i32) -> bool {
        self.is_land(world_x, world_z) && height >= SEA_LEVEL - 1 && height <= SEA_LEVEL + 3
    }

    /// Rolling hills — medium frequency, low amplitude (Minecraft-style erosion layer).
    fn erosion_layer(&self, world_x: i32, world_z: i32) -> f32 {
        let wx = world_x as f32;
        let wz = world_z as f32;
        self.fbm(wx * 0.0045, wz * 0.0045, 4, 0.48, 2.0)
    }

    /// Gentle peaks — only positive ridges, keeps topology readable.
    fn peaks_layer(&self, world_x: i32, world_z: i32) -> f32 {
        let wx = world_x as f32;
        let wz = world_z as f32;
        let n = self.fbm(wx * 0.003 + 60.0, wz * 0.003 + 60.0, 3, 0.5, 2.0);
        (n * 0.5 + 0.25).max(0.0)
    }

    pub fn terrain_height(&self, world_x: i32, world_z: i32) -> i32 {
        let wx = world_x as f32;
        let wz = world_z as f32;
        let cont = self.continentalness(world_x, world_z);

        if cont < -0.32 {
            let depth = ((cont + 1.0) * 0.5).clamp(0.0, 1.0);
            return (10.0 + depth * 10.0).floor() as i32;
        }

        if cont < -0.12 {
            let shore = ((cont + 0.32) / 0.2).clamp(0.0, 1.0);
            let shelf = self.fbm(wx * 0.018, wz * 0.018, 2, 0.4, 2.0) * 1.5;
            return (SEA_LEVEL as f32 - 4.0 + shore * 10.0 + shelf).floor() as i32;
        }

        let land_strength = ((cont + 0.12) / 0.5).clamp(0.0, 1.0);
        let erosion = self.erosion_layer(world_x, world_z);
        let peaks = self.peaks_layer(world_x, world_z);
        let mut height =
            SEA_LEVEL as f32 + 5.0 + land_strength * 6.0 + erosion * 5.5 + peaks * 4.0;

        let settle = self.settlement_at(world_x, world_z);
        if settle > 0.15 {
            let village_base = SEA_LEVEL as f32 + 8.0;
            height = height * (1.0 - settle * 0.92) + village_base * (settle * 0.92);
        }

        height.clamp(SEA_LEVEL as f32 + 2.0, SEA_LEVEL as f32 + 22.0).floor() as i32
    }

    /// How flat the terrain is in a 2-block radius — used to avoid trees on cliffs.
    pub fn local_flatness(&self, world_x: i32, world_z: i32) -> f32 {
        let center = self.terrain_height(world_x, world_z);
        let mut max_delta = 0;
        for dx in -2i32..=2 {
            for dz in -2i32..=2 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let h = self.terrain_height(world_x + dx, world_z + dz);
                max_delta = max_delta.max((h - center).abs());
            }
        }
        (1.0 - max_delta as f32 / 5.0).clamp(0.0, 1.0)
    }

    pub fn biome(&self, world_x: i32, world_z: i32) -> BiomeSample {
        let wx = world_x as f32;
        let wz = world_z as f32;
        BiomeSample {
            moisture: self.fbm(wx * 0.006 + 100.0, wz * 0.006 + 100.0, 3, 0.5, 2.0),
            temperature: self.fbm(wx * 0.005, wz * 0.005, 3, 0.5, 2.0),
        }
    }

    pub fn settlement_at(&self, world_x: i32, world_z: i32) -> f32 {
        const GRID: i32 = 192;
        let cell_x = world_x.div_euclid(GRID);
        let cell_z = world_z.div_euclid(GRID);
        let cell = self.fbm(cell_x as f32 * 0.85 + 500.0, cell_z as f32 * 0.85 + 500.0, 2, 0.5, 2.0);
        if cell < 0.18 {
            return 0.0;
        }
        let lx = world_x.rem_euclid(GRID);
        let lz = world_z.rem_euclid(GRID);
        let dx = lx.min(GRID - lx) as f32 / (GRID as f32 * 0.48);
        let dz = lz.min(GRID - lz) as f32 / (GRID as f32 * 0.48);
        let edge = dx.min(dz);
        (edge * ((cell - 0.18) / 0.32)).clamp(0.0, 1.0)
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
                if cell < 0.18 {
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
        self.settlement_at(world_x, world_z) > 0.4
    }

    pub fn roll(&self, world_x: i32, world_z: i32, salt: i32) -> f32 {
        let h = (world_x as u32)
            .wrapping_mul(374761393)
            .wrapping_add((world_z as u32).wrapping_mul(668265263))
            .wrapping_add((salt as u32).wrapping_mul(1013904223));
        let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        ((h ^ (h >> 16)) % 10000) as f32 / 10000.0
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
        if self.local_flatness(world_x, world_z) < 0.55 {
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
            roll < 0.14
        } else if self.is_meadow(world_x, world_z) {
            roll < 0.06
        } else {
            roll < 0.03
        }
    }

    pub fn should_place_bush(&self, world_x: i32, world_z: i32) -> bool {
        self.is_land(world_x, world_z)
            && self.local_flatness(world_x, world_z) > 0.5
            && self.roll(world_x, world_z, 13) < 0.05
    }

    pub fn should_place_tall_grass(&self, world_x: i32, world_z: i32) -> bool {
        self.is_land(world_x, world_z) && self.roll(world_x, world_z, 17) < 0.12
    }

    pub fn should_place_flower(&self, world_x: i32, world_z: i32) -> bool {
        self.is_land(world_x, world_z) && self.roll(world_x, world_z, 19) < 0.04
    }

    /// Minecraft-style cheese caves + horizontal worm tunnels, with a protected surface cap.
    pub fn is_cave(&self, world_x: i32, y: i32, world_z: i32, surface_y: i32) -> bool {
        if y <= 2 {
            return false;
        }
        let surface_buffer = if surface_y <= SEA_LEVEL + 2 {
            12
        } else {
            10
        };
        if y >= surface_y - surface_buffer {
            return false;
        }

        let fx = world_x as f32 * 0.045;
        let fy = y as f32 * 0.045;
        let fz = world_z as f32 * 0.045;

        let cheese = self.fbm3d(fx, fy, fz, 3, 0.5, 2.0);
        if cheese < -0.08 {
            return true;
        }

        let worm_a = self.noise3d(fx * 1.6 + 20.0, fy * 1.6, fz * 1.6);
        let worm_b = self.noise3d(fx * 1.6, fy * 1.6 + 40.0, fz * 1.6 + 80.0);
        let tunnel = worm_a * worm_a + worm_b * worm_b;
        tunnel < 0.012 && y < surface_y - surface_buffer - 2
    }

    pub fn is_deep_cavern(&self, world_x: i32, y: i32, world_z: i32, surface_y: i32) -> bool {
        if y > 18 || y < 6 || y >= surface_y - 14 {
            return false;
        }
        let fx = world_x as f32 * 0.035;
        let fy = y as f32 * 0.035;
        let fz = world_z as f32 * 0.035;
        self.fbm3d(fx, fy, fz, 2, 0.5, 2.0) < -0.18
    }
}

#[derive(Clone, Copy)]
pub struct BiomeSample {
    pub moisture: f32,
    pub temperature: f32,
}
