use crate::blocks::BlockId;
use crate::config::{CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};
use crate::noise::NoiseGenerator;
use crate::structures::place_settlement;
use crate::structures::place_volcano;
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
            if cell < 0.18 {
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
            if surface_y <= SEA_LEVEL + 2 {
                continue;
            }
            if world.noise.local_flatness(center_x, center_z) < 0.7 {
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

pub fn ensure_volcanoes_near(world: &mut VoxelWorld, world_x: i32, world_z: i32, radius: i32) {
    const GRID: i32 = 384;
    let min_cell_x = (world_x - radius).div_euclid(GRID);
    let max_cell_x = (world_x + radius).div_euclid(GRID);
    let min_cell_z = (world_z - radius).div_euclid(GRID);
    let max_cell_z = (world_z + radius).div_euclid(GRID);

    let mut pending: Vec<(i32, i32, i32, i32)> = Vec::new();
    for cell_x in min_cell_x..=max_cell_x {
        for cell_z in min_cell_z..=max_cell_z {
            let key = (cell_x, cell_z);
            if world.placed_volcanoes.contains(&key) {
                continue;
            }
            if world.noise.volcano_cell_score(cell_x, cell_z) < 0.24 {
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
            if world.noise.settlement_at(center_x, center_z) > 0.2 {
                continue;
            }
            let peaks = world.noise.peaks_layer(center_x, center_z);
            if peaks < 0.08 {
                continue;
            }
            pending.push((center_x, center_z, cell_x, cell_z));
        }
    }

    for (center_x, center_z, cell_x, cell_z) in pending {
        world.load_chunks_around(center_x, world_z);
        world.load_chunks_around(world_x, center_z);
        world.load_chunks_around(center_x, center_z);
        place_volcano(world, center_x, center_z);
        world.placed_volcanoes.insert((cell_x, cell_z));
        mark_feature_dirty(world, center_x, center_z, 3);
    }
}

fn mark_feature_dirty(world: &mut VoxelWorld, center_x: i32, center_z: i32, chunk_ring: i32) {
    let center_chunk_x = center_x.div_euclid(CHUNK_SIZE);
    let center_chunk_z = center_z.div_euclid(CHUNK_SIZE);
    for dx in -chunk_ring..=chunk_ring {
        for dz in -chunk_ring..=chunk_ring {
            if let Some(chunk) = world.chunks.get_mut(&(center_chunk_x + dx, center_chunk_z + dz)) {
                chunk.dirty = true;
            }
        }
    }
}

fn mark_settlement_dirty(world: &mut VoxelWorld, center_x: i32, center_z: i32) {
    mark_feature_dirty(world, center_x, center_z, 2);
}

pub fn try_place_underground_lava(
    blocks: &mut [BlockId],
    x: i32,
    y: i32,
    z: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
    surface_y: i32,
) {
    if y < 6 || y > 18 {
        return;
    }
    if get_block_local(blocks, x, y, z) != BlockId::Air {
        return;
    }
    if get_block_local(blocks, x, y - 1, z) != BlockId::Stone {
        return;
    }
    if count_cave_openness(blocks, x, y, z) < 8 {
        return;
    }
    if !noise.is_deep_cavern(world_x, y, world_z, surface_y) {
        return;
    }
    let lava_roll = noise.roll(world_x + y * 3, world_z + y * 5, 29);
    if lava_roll > 0.07 {
        return;
    }
    set_block_local(blocks, x, y, z, BlockId::Lava);
}

pub fn generate_chunk(chunk_x: i32, chunk_z: i32, noise: &NoiseGenerator) -> ChunkData {
    let mut blocks = vec![BlockId::Air; (CHUNK_SIZE * WORLD_HEIGHT * CHUNK_SIZE) as usize];
    let world_x_base = chunk_x * CHUNK_SIZE;
    let world_z_base = chunk_z * CHUNK_SIZE;

    let mut heights = [[0i32; CHUNK_SIZE as usize]; CHUNK_SIZE as usize];

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = world_x_base + x;
            let world_z = world_z_base + z;
            let height = noise.terrain_height(world_x, world_z);
            heights[x as usize][z as usize] = height;
            fill_terrain_column(&mut blocks, x, z, height, world_x, world_z, noise);
        }
    }

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = world_x_base + x;
            let world_z = world_z_base + z;
            let height = heights[x as usize][z as usize];
            carve_underground(&mut blocks, x, z, height, world_x, world_z, noise);
        }
    }

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = world_x_base + x;
            let world_z = world_z_base + z;
            let height = heights[x as usize][z as usize];
            place_surface_decorations(&mut blocks, x, z, height, world_x, world_z, noise);
        }
    }

    ChunkData {
        chunk_x,
        chunk_z,
        blocks,
        dirty: true,
    }
}

fn fill_terrain_column(
    blocks: &mut [BlockId],
    x: i32,
    z: i32,
    height: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
) {
    let biome = noise.biome(world_x, world_z);
    let is_land = noise.is_land(world_x, world_z);
    let is_shallow = noise.is_shallow_ocean(world_x, world_z);
    let is_beach = noise.is_beach(world_x, world_z, height);
    let is_desert = is_land && biome.temperature > 0.3 && biome.moisture < -0.1;
    let is_snow = is_land && biome.temperature < -0.3;

    for y in 0..WORLD_HEIGHT {
        let block = if y == 0 {
            BlockId::Bedrock
        } else if y < height - 4 {
            if is_shallow || is_beach {
                BlockId::Sand
            } else {
                BlockId::Stone
            }
        } else if y < height - 1 {
            if is_desert || is_shallow || is_beach {
                BlockId::Sand
            } else {
                BlockId::Dirt
            }
        } else if y < height {
            if is_desert || is_shallow || is_beach {
                BlockId::Sand
            } else if is_snow {
                BlockId::Snow
            } else if is_land {
                BlockId::Grass
            } else {
                BlockId::Sand
            }
        } else if y <= SEA_LEVEL && height <= SEA_LEVEL {
            BlockId::Water
        } else {
            BlockId::Air
        };
        set_block_local(blocks, x, y, z, block);
    }
}

fn place_surface_decorations(
    blocks: &mut [BlockId],
    x: i32,
    z: i32,
    heightmap_y: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
) {
    let Some(surface_y) = find_surface_y(blocks, x, z, heightmap_y) else {
        return;
    };

    if surface_y <= SEA_LEVEL + 1 || !noise.is_land(world_x, world_z) {
        return;
    }

    let surface_block = get_block_local(blocks, x, surface_y, z);
    if !matches!(surface_block, BlockId::Grass | BlockId::Dirt | BlockId::Sand | BlockId::Snow) {
        return;
    }

    if !is_stable_surface(blocks, x, surface_y, z) {
        return;
    }

    let biome = noise.biome(world_x, world_z);
    let is_desert = biome.temperature > 0.3 && biome.moisture < -0.1;
    let is_snow = biome.temperature < -0.3;

    if !is_desert && !is_snow && !noise.is_in_settlement(world_x, world_z) && !noise.is_in_volcano(world_x, world_z) {
        // Trees and bushes are placed in world_gen::decorate_chunk_vegetation.
    }

    let above = surface_y + 1;
    if above < WORLD_HEIGHT && get_block_local(blocks, x, above, z) == BlockId::Air {
        if noise.should_place_flower(world_x, world_z) {
            set_block_local(blocks, x, above, z, BlockId::Flower);
        } else if noise.should_place_tall_grass(world_x, world_z) {
            set_block_local(blocks, x, above, z, BlockId::TallGrass);
        }
    }
}

fn find_surface_y(blocks: &[BlockId], x: i32, z: i32, hint: i32) -> Option<i32> {
    let start = (hint + 4).min(WORLD_HEIGHT - 1);
    let end = (hint - 8).max(1);
    for y in (end..=start).rev() {
        let block = get_block_local(blocks, x, y, z);
        if block.solid() && block != BlockId::Bedrock {
            return Some(y);
        }
    }
    None
}

fn is_stable_surface(blocks: &[BlockId], x: i32, surface_y: i32, z: i32) -> bool {
    for dx in -1i32..=1 {
        for dz in -1i32..=1 {
            let ny = surface_y;
            let neighbor = get_block_local(blocks, x + dx, ny, z + dz);
            if !neighbor.solid() || neighbor == BlockId::Water || neighbor == BlockId::Lava {
                return false;
            }
            if get_block_local(blocks, x + dx, ny + 1, z + dz) == BlockId::Air
                && get_block_local(blocks, x + dx, ny - 1, z + dz) == BlockId::Air
            {
                return false;
            }
        }
    }
    true
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

fn carve_underground(
    blocks: &mut [BlockId],
    x: i32,
    z: i32,
    surface_y: i32,
    world_x: i32,
    world_z: i32,
    noise: &NoiseGenerator,
) {
    let carve_top = (surface_y - 10).min(WORLD_HEIGHT - 2);
    for y in 2..carve_top {
        let block = get_block_local(blocks, x, y, z);
        if !matches!(block, BlockId::Stone | BlockId::Dirt | BlockId::Sand) {
            continue;
        }
        if noise.is_cave(world_x, y, world_z, surface_y) {
            set_block_local(blocks, x, y, z, BlockId::Air);
        }
    }

    for y in 6..=18 {
        if y >= carve_top {
            break;
        }
        try_place_underground_lava(blocks, x, y, z, world_x, world_z, noise, surface_y);
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
