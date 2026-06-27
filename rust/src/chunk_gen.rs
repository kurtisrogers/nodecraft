use crate::blocks::BlockId;
use crate::config::{CHUNK_SIZE, WORLD_HEIGHT};
use crate::noise::NoiseGenerator;
use crate::structures::place_settlement;
use crate::world::VoxelWorld;

pub fn ensure_settlements_near(world: &mut VoxelWorld, world_x: i32, world_z: i32, radius: i32) {
    const GRID: i32 = 192;
    let min_cell_x = (world_x - radius).div_euclid(GRID);
    let max_cell_x = (world_x + radius).div_euclid(GRID);
    let min_cell_z = (world_z - radius).div_euclid(GRID);
    let max_cell_z = (world_z + radius).div_euclid(GRID);

    let mut pending: Vec<(i32, i32, i32, i32, i32)> = Vec::new();
    for cell_x in min_cell_x..=max_cell_x {
        for cell_z in min_cell_z..=max_cell_z {
            let key = (cell_x, cell_z);
            if world.placed_settlements.contains(&key) {
                continue;
            }
            let cell = world.noise.fbm(
                cell_x as f32 * 0.85 + 500.0,
                cell_z as f32 * 0.85 + 500.0,
                2,
                0.5,
                2.0,
            );
            if cell < 0.15 {
                continue;
            }
            let center_x = cell_x * GRID + GRID / 2;
            let center_z = cell_z * GRID + GRID / 2;
            let dist = (((center_x - world_x).pow(2) + (center_z - world_z).pow(2)) as f32).sqrt();
            if dist > radius as f32 {
                continue;
            }
            if !world.noise.is_land(center_x, center_z) {
                continue;
            }
            let surface_y = world.noise.terrain_height(center_x, center_z);
            if surface_y <= crate::config::SEA_LEVEL + 1 {
                continue;
            }
            pending.push((center_x, center_z, surface_y, cell_x, cell_z));
        }
    }

    for (center_x, center_z, surface_y, cell_x, cell_z) in pending {
        world.load_chunks_around(center_x, world_z);
        world.load_chunks_around(world_x, center_z);
        world.load_chunks_around(center_x, center_z);
        place_settlement(world, center_x, center_z, surface_y);
        world.placed_settlements.insert((cell_x, cell_z));
        mark_settlement_dirty(world, center_x, center_z);
    }
}

fn mark_settlement_dirty(world: &mut VoxelWorld, center_x: i32, center_z: i32) {
    let center_chunk_x = center_x.div_euclid(CHUNK_SIZE);
    let center_chunk_z = center_z.div_euclid(CHUNK_SIZE);
    for dx in -2i32..=2 {
        for dz in -2i32..=2 {
            if let Some(chunk) = world.chunks.get_mut(&(center_chunk_x + dx, center_chunk_z + dz)) {
                chunk.dirty = true;
            }
        }
    }
}

pub fn generate_chunk(chunk_x: i32, chunk_z: i32, noise: &NoiseGenerator) -> ChunkData {
    let mut blocks = vec![BlockId::Air; (CHUNK_SIZE * WORLD_HEIGHT * CHUNK_SIZE) as usize];
    let world_x_base = chunk_x * CHUNK_SIZE;
    let world_z_base = chunk_z * CHUNK_SIZE;

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = world_x_base + x;
            let world_z = world_z_base + z;
            let height = noise.terrain_height(world_x, world_z);
            let biome = noise.biome(world_x, world_z);
            let is_land = noise.is_land(world_x, world_z);
            let is_shallow = noise.is_shallow_ocean(world_x, world_z);
            let is_desert = is_land && biome.temperature > 0.3 && biome.moisture < -0.1;
            let is_snow = is_land && biome.temperature < -0.3;

            for y in 0..WORLD_HEIGHT {
                let block = if y == 0 {
                    BlockId::Bedrock
                } else if y < height - 4 {
                    if is_shallow { BlockId::Sand } else { BlockId::Stone }
                } else if y < height - 1 {
                    if is_desert || is_shallow { BlockId::Sand } else { BlockId::Dirt }
                } else if y < height {
                    if is_desert || is_shallow {
                        BlockId::Sand
                    } else if is_snow {
                        BlockId::Snow
                    } else if is_land {
                        BlockId::Grass
                    } else {
                        BlockId::Sand
                    }
                } else if y <= crate::config::SEA_LEVEL && height <= crate::config::SEA_LEVEL {
                    BlockId::Water
                } else {
                    BlockId::Air
                };
                set_block_local(&mut blocks, x, y, z, block);
            }

            let surface_y = height - 1;
            if surface_y >= 1 && is_land && height > crate::config::SEA_LEVEL + 1 && !is_desert && !is_snow {
                if !noise.is_in_settlement(world_x, world_z) && noise.should_place_tree(world_x, world_z) {
                    if count_tree_clearance(&blocks, x, z, surface_y) >= 18 {
                        generate_tree(&mut blocks, x, surface_y, z, world_x, world_z, noise);
                    }
                } else if noise.should_place_bush(world_x, world_z) {
                    generate_bush(&mut blocks, x, surface_y, z);
                }
                let above = surface_y + 1;
                if above < WORLD_HEIGHT && get_block_local(&blocks, x, above, z) == BlockId::Air {
                    if noise.should_place_flower(world_x, world_z) {
                        set_block_local(&mut blocks, x, above, z, BlockId::Flower);
                    } else if noise.should_place_tall_grass(world_x, world_z) {
                        set_block_local(&mut blocks, x, above, z, BlockId::TallGrass);
                    }
                }
            }

            if is_land && noise.is_volcanic(world_x, world_z) {
                generate_volcanic(&mut blocks, x, z, height, world_x, world_z, noise);
            }
            carve_underground(&mut blocks, x, z, height, world_x, world_z, noise);
        }
    }

    ChunkData {
        chunk_x,
        chunk_z,
        blocks,
        dirty: true,
    }
}

pub struct ChunkData {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub blocks: Vec<BlockId>,
    pub dirty: bool,
}

pub fn chunk_index(x: i32, y: i32, z: i32) -> usize {
    (x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE) as usize
}

pub fn get_block_local(blocks: &[BlockId], x: i32, y: i32, z: i32) -> BlockId {
    if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= WORLD_HEIGHT || z < 0 || z >= CHUNK_SIZE {
        return BlockId::Air;
    }
    blocks[chunk_index(x, y, z)]
}

pub fn set_block_local(blocks: &mut [BlockId], x: i32, y: i32, z: i32, block: BlockId) {
    if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= WORLD_HEIGHT || z < 0 || z >= CHUNK_SIZE {
        return;
    }
    blocks[chunk_index(x, y, z)] = block;
}

fn count_tree_clearance(blocks: &[BlockId], x: i32, z: i32, surface_y: i32) -> i32 {
    let mut score = 0;
    for dx in -2i32..=2 {
        for dz in -2i32..=2 {
            let foot = get_block_local(blocks, x + dx, surface_y + 1, z + dz);
            let head = get_block_local(blocks, x + dx, surface_y + 2, z + dz);
            if matches!(foot, BlockId::Air | BlockId::TallGrass | BlockId::Flower) {
                score += 1;
            }
            if head == BlockId::Air {
                score += 1;
            }
        }
    }
    score
}

fn generate_tree(
    blocks: &mut [BlockId],
    x: i32,
    surface_y: i32,
    z: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
) {
    let variant = noise.roll(world_x, world_z, 11);
    let trunk_height = if variant > 0.7 { 6 } else if variant > 0.35 { 5 } else { 4 };
    for dy in 0..trunk_height {
        if surface_y + dy < WORLD_HEIGHT {
            set_block_local(blocks, x, surface_y + dy, z, BlockId::Wood);
        }
    }
    let leaf_start = surface_y + trunk_height - 2;
    for dy in 0..4 {
        for dx in -2i32..=2 {
            for dz in -2i32..=2 {
                if dx.abs() == 2 && dz.abs() == 2 {
                    continue;
                }
                if dy == 3 && (dx.abs() > 1 || dz.abs() > 1) {
                    continue;
                }
                let existing = get_block_local(blocks, x + dx, leaf_start + dy, z + dz);
                if matches!(existing, BlockId::Air | BlockId::TallGrass | BlockId::Flower) {
                    set_block_local(blocks, x + dx, leaf_start + dy, z + dz, BlockId::Leaves);
                }
            }
        }
    }
}

fn generate_bush(blocks: &mut [BlockId], x: i32, surface_y: i32, z: i32) {
    for dx in -1i32..=1 {
        for dz in -1i32..=1 {
            let ly = surface_y + 1;
            if ly < WORLD_HEIGHT && get_block_local(blocks, x + dx, ly, z + dz) == BlockId::Air {
                set_block_local(blocks, x + dx, ly, z + dz, BlockId::Leaves);
            }
        }
    }
}

fn generate_volcanic(
    blocks: &mut [BlockId],
    x: i32,
    z: i32,
    height: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
) {
    let pool_roll = noise.roll(world_x, world_z, 31);
    if pool_roll > 0.92 && height > crate::config::SEA_LEVEL + 2 {
        for dx in -1i32..=1 {
            for dz in -1i32..=1 {
                if dx.abs() + dz.abs() > 1 {
                    continue;
                }
                set_block_local(blocks, x + dx, height, z + dz, BlockId::Lava);
                if height > 1 {
                    set_block_local(blocks, x + dx, height - 1, z + dz, BlockId::Stone);
                }
            }
        }
    }
}

fn carve_underground(
    blocks: &mut [BlockId],
    x: i32,
    z: i32,
    surface_y: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
) {
    let carve_top = (surface_y - 3).min(WORLD_HEIGHT - 2);
    for y in 2..carve_top {
        if get_block_local(blocks, x, y, z) == BlockId::Stone && noise.is_cave(world_x, y, world_z, surface_y) {
            set_block_local(blocks, x, y, z, BlockId::Air);
        }
    }
    for y in 4..=26 {
        if y >= carve_top {
            break;
        }
        if get_block_local(blocks, x, y, z) != BlockId::Air {
            continue;
        }
        if get_block_local(blocks, x, y - 1, z) != BlockId::Stone {
            continue;
        }
        if count_cave_openness(blocks, x, y, z) < 5 {
            continue;
        }
        let deep = noise.is_deep_cavern(world_x, y, world_z, surface_y);
        let lava_roll = noise.roll(world_x + y * 3, world_z + y * 5, 29);
        let threshold = if deep { 0.08 } else { 0.035 };
        if lava_roll > threshold {
            continue;
        }
        set_block_local(blocks, x, y, z, BlockId::Lava);
        expand_lava_pool(blocks, x, y, z, if deep { 2 } else { 1 });
    }
}

fn count_cave_openness(blocks: &[BlockId], x: i32, y: i32, z: i32) -> i32 {
    let mut score = 0;
    for dx in -2i32..=2 {
        for dy in -1i32..=2 {
            for dz in -2i32..=2 {
                let block = get_block_local(blocks, x + dx, y + dy, z + dz);
                if matches!(block, BlockId::Air | BlockId::Lava) {
                    score += 1;
                }
            }
        }
    }
    score
}

fn expand_lava_pool(blocks: &mut [BlockId], x: i32, y: i32, z: i32, radius: i32) {
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if dx == 0 && dz == 0 {
                continue;
            }
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            if get_block_local(blocks, x + dx, y, z + dz) != BlockId::Air {
                continue;
            }
            if get_block_local(blocks, x + dx, y - 1, z + dz) != BlockId::Stone {
                continue;
            }
            set_block_local(blocks, x + dx, y, z + dz, BlockId::Lava);
        }
    }
}
