use crate::blocks::BlockId;
use crate::chunk_gen::{generate_chunk, get_block_local, set_block_local, ChunkData};
use crate::config::{CHUNK_SIZE, RENDER_DISTANCE, SEA_LEVEL, WORLD_HEIGHT};
use crate::noise::NoiseGenerator;
use std::collections::HashMap;

pub struct VoxelWorld {
    pub seed: u32,
    pub noise: NoiseGenerator,
    pub chunks: HashMap<(i32, i32), ChunkData>,
    pub modifications: HashMap<(i32, i32, i32), BlockId>,
    pub placed_settlements: std::collections::HashSet<(i32, i32)>,
    pub render_distance: i32,
}

impl VoxelWorld {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            noise: NoiseGenerator::new(seed),
            chunks: HashMap::new(),
            modifications: HashMap::new(),
            placed_settlements: std::collections::HashSet::new(),
            render_distance: RENDER_DISTANCE,
        }
    }

    pub fn get_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> &mut ChunkData {
        let key = (chunk_x, chunk_z);
        if !self.chunks.contains_key(&key) {
            let chunk = generate_chunk(chunk_x, chunk_z, &self.noise);
            self.chunks.insert(key, chunk);
            self.mark_neighbors_dirty(chunk_x, chunk_z);
        }
        self.chunks.get_mut(&key).unwrap()
    }

    fn mark_neighbors_dirty(&mut self, chunk_x: i32, chunk_z: i32) {
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                if let Some(chunk) = self.chunks.get_mut(&(chunk_x + dx, chunk_z + dz)) {
                    chunk.dirty = true;
                }
            }
        }
    }

    pub fn peek_block(&self, world_x: i32, world_y: i32, world_z: i32) -> BlockId {
        if world_y < 0 || world_y >= WORLD_HEIGHT {
            return BlockId::Air;
        }
        if let Some(&block) = self.modifications.get(&(world_x, world_y, world_z)) {
            return block;
        }
        let chunk_x = world_x.div_euclid(CHUNK_SIZE);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE);
        let Some(chunk) = self.chunks.get(&(chunk_x, chunk_z)) else {
            return BlockId::Air;
        };
        let lx = world_x.rem_euclid(CHUNK_SIZE);
        let lz = world_z.rem_euclid(CHUNK_SIZE);
        get_block_local(&chunk.blocks, lx, world_y, lz)
    }

    pub fn get_block(&mut self, world_x: i32, world_y: i32, world_z: i32) -> BlockId {
        if world_y < 0 || world_y >= WORLD_HEIGHT {
            return BlockId::Air;
        }
        if let Some(&block) = self.modifications.get(&(world_x, world_y, world_z)) {
            return block;
        }
        let chunk_x = world_x.div_euclid(CHUNK_SIZE);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE);
        let chunk = self.get_chunk(chunk_x, chunk_z);
        let lx = world_x.rem_euclid(CHUNK_SIZE);
        let lz = world_z.rem_euclid(CHUNK_SIZE);
        get_block_local(&chunk.blocks, lx, world_y, lz)
    }

    pub fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, block: BlockId) {
        if world_y < 0 || world_y >= WORLD_HEIGHT {
            return;
        }
        if block == BlockId::Air {
            self.modifications.remove(&(world_x, world_y, world_z));
        } else {
            self.modifications.insert((world_x, world_y, world_z), block);
        }
        let chunk_x = world_x.div_euclid(CHUNK_SIZE);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE);
        let chunk = self.get_chunk(chunk_x, chunk_z);
        let lx = world_x.rem_euclid(CHUNK_SIZE);
        let lz = world_z.rem_euclid(CHUNK_SIZE);
        set_block_local(&mut chunk.blocks, lx, world_y, lz, block);
        chunk.dirty = true;

        if lx == 0 {
            self.mark_chunk_dirty(chunk_x - 1, chunk_z);
        }
        if lx == CHUNK_SIZE - 1 {
            self.mark_chunk_dirty(chunk_x + 1, chunk_z);
        }
        if lz == 0 {
            self.mark_chunk_dirty(chunk_x, chunk_z - 1);
        }
        if lz == CHUNK_SIZE - 1 {
            self.mark_chunk_dirty(chunk_x, chunk_z + 1);
        }
        if lx == 0 && lz == 0 {
            self.mark_chunk_dirty(chunk_x - 1, chunk_z - 1);
        }
        if lx == 0 && lz == CHUNK_SIZE - 1 {
            self.mark_chunk_dirty(chunk_x - 1, chunk_z + 1);
        }
        if lx == CHUNK_SIZE - 1 && lz == 0 {
            self.mark_chunk_dirty(chunk_x + 1, chunk_z - 1);
        }
        if lx == CHUNK_SIZE - 1 && lz == CHUNK_SIZE - 1 {
            self.mark_chunk_dirty(chunk_x + 1, chunk_z + 1);
        }
    }

    fn mark_chunk_dirty(&mut self, chunk_x: i32, chunk_z: i32) {
        if let Some(chunk) = self.chunks.get_mut(&(chunk_x, chunk_z)) {
            chunk.dirty = true;
        }
    }

    pub fn load_chunks_around(&mut self, world_x: i32, world_z: i32) -> Vec<(i32, i32)> {
        let center_x = world_x.div_euclid(CHUNK_SIZE);
        let center_z = world_z.div_euclid(CHUNK_SIZE);
        let mut loaded = Vec::new();
        for dx in -self.render_distance..=self.render_distance {
            for dz in -self.render_distance..=self.render_distance {
                if dx * dx + dz * dz > self.render_distance * self.render_distance {
                    continue;
                }
                let cx = center_x + dx;
                let cz = center_z + dz;
                self.get_chunk(cx, cz);
                loaded.push((cx, cz));
            }
        }
        loaded
    }

    pub fn unload_distant_chunks(&mut self, world_x: i32, world_z: i32) {
        let center_x = world_x.div_euclid(CHUNK_SIZE);
        let center_z = world_z.div_euclid(CHUNK_SIZE);
        let max_dist = self.render_distance + 2;
        let keys: Vec<_> = self.chunks.keys().copied().collect();
        for (cx, cz) in keys {
            let dist = (cx - center_x).abs().max((cz - center_z).abs());
            if dist > max_dist {
                self.chunks.remove(&(cx, cz));
            }
        }
    }

    pub fn is_volume_clear(&mut self, x: f32, y: f32, z: f32, half: f32, height: f32) -> bool {
        let min_x = (x - half).floor() as i32;
        let max_x = (x + half).floor() as i32;
        let min_y = y.floor() as i32;
        let max_y = (y + height).floor() as i32;
        let min_z = (z - half).floor() as i32;
        let max_z = (z + half).floor() as i32;

        for bx in min_x..=max_x {
            for by in min_y..=max_y {
                for bz in min_z..=max_z {
                    if self.get_block(bx, by, bz).solid() {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn find_safe_spawn(&mut self, preferred_x: i32, preferred_z: i32) -> (f32, f32, f32) {
        if let Some((cx, cz)) = self.noise.settlement_center_near(preferred_x, preferred_z, 320) {
            crate::chunk_gen::ensure_settlements_near(self, cx, cz, 64);
            if let Some(pos) = self.find_spawn_in_area(cx, cz, 18) {
                return pos;
            }
        }
        self.find_spawn_in_area(preferred_x, preferred_z, 40)
            .unwrap_or((0.5, 40.0, 0.5))
    }

    fn find_spawn_in_area(&mut self, preferred_x: i32, preferred_z: i32, max_radius: i32) -> Option<(f32, f32, f32)> {
        let mut best: Option<(i32, i32, i32, i32)> = None;
        for r in (0..=max_radius).step_by(2) {
            let count = if r == 0 { 1 } else { 12 };
            for a in 0..count {
                let (x, z) = if r == 0 {
                    (preferred_x, preferred_z)
                } else {
                    let angle = (a as f32 / count as f32) * std::f32::consts::TAU;
                    (
                        (preferred_x as f32 + angle.cos() * r as f32).round() as i32,
                        (preferred_z as f32 + angle.sin() * r as f32).round() as i32,
                    )
                };
                self.get_chunk(x.div_euclid(CHUNK_SIZE), z.div_euclid(CHUNK_SIZE));
                let surface_y = self.walkable_surface_y(x, z)?;
                if surface_y <= SEA_LEVEL + 1 || !self.noise.is_land(x, z) {
                    continue;
                }
                if self.get_block(x, surface_y, z) == BlockId::Lava {
                    continue;
                }
                let openness = self.count_open_space(x, z, surface_y);
                if openness < 40 {
                    continue;
                }
                let spawn_y = surface_y + 1;
                if !self.is_volume_clear(x as f32 + 0.5, spawn_y as f32, z as f32 + 0.5, 0.3, 1.7) {
                    continue;
                }
                let dist = (x - preferred_x).abs() + (z - preferred_z).abs();
                let village_bonus = if self.noise.is_in_settlement(x, z) { 200 } else { 0 };
                let score = openness * 5 - dist + village_bonus;
                if best.map(|b| score > b.3).unwrap_or(true) {
                    best = Some((x, z, spawn_y, score));
                }
            }
        }
        best.map(|(x, z, y, _)| (x as f32 + 0.5, y as f32, z as f32 + 0.5))
    }

    fn walkable_surface_y(&mut self, world_x: i32, world_z: i32) -> Option<i32> {
        for y in (0..WORLD_HEIGHT).rev() {
            let block = self.get_block(world_x, y, world_z);
            if matches!(
                block,
                BlockId::Grass | BlockId::Dirt | BlockId::Sand | BlockId::Snow | BlockId::Planks
            ) {
                return Some(y);
            }
        }
        None
    }

    fn count_open_space(&mut self, x: i32, z: i32, surface_y: i32) -> i32 {
        let mut score = 0;
        for dx in -2..=2 {
            for dz in -2..=2 {
                let foot = self.get_block(x + dx, surface_y + 1, z + dz);
                let head = self.get_block(x + dx, surface_y + 2, z + dz);
                if !foot.solid() && foot != BlockId::Lava && foot != BlockId::Water {
                    score += 1;
                }
                if !head.solid() && head != BlockId::Lava && head != BlockId::Water {
                    score += 1;
                }
            }
        }
        score
    }
}
