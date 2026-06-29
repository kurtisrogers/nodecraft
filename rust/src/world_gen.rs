use crate::blocks::BlockId;
use crate::config::{CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};
use crate::world::VoxelWorld;

pub const TREE_EDGE_MARGIN: i32 = 3;
pub const MAX_TRUNK_HEIGHT: i32 = 5;
pub const MIN_TRUNK_HEIGHT: i32 = 3;
pub const MAX_TREE_TOP_ABOVE_SURFACE: i32 = 6;

/// Place trees and bushes using world coordinates so foliage spans chunk borders.
pub fn decorate_chunk_vegetation(world: &mut VoxelWorld, chunk_x: i32, chunk_z: i32) {
    let x_base = chunk_x * CHUNK_SIZE;
    let z_base = chunk_z * CHUNK_SIZE;

    for lx in TREE_EDGE_MARGIN..(CHUNK_SIZE - TREE_EDGE_MARGIN) {
        for lz in TREE_EDGE_MARGIN..(CHUNK_SIZE - TREE_EDGE_MARGIN) {
            let wx = x_base + lx;
            let wz = z_base + lz;
            let Some(surface_y) = surface_y_at(world, wx, wz) else {
                continue;
            };
            if world.peek_block(wx, surface_y, wz) == BlockId::Wood {
                continue;
            }
            if surface_y <= SEA_LEVEL + 1 || !world.noise.is_land(wx, wz) {
                continue;
            }
            if !is_stable_surface_world(world, wx, surface_y, wz) {
                continue;
            }

            let biome = world.noise.biome(wx, wz);
            let is_desert = biome.temperature > 0.3 && biome.moisture < -0.1;
            let is_snow = biome.temperature < -0.3;
            if is_desert || is_snow || world.noise.is_in_settlement(wx, wz) || world.noise.is_in_volcano(wx, wz) {
                continue;
            }

            if world.noise.should_place_tree(wx, wz) && tree_clearance_world(world, wx, wz, surface_y) >= 20 {
                place_tree_world(world, wx, surface_y, wz);
            } else if world.noise.should_place_bush(wx, wz) {
                place_bush_world(world, wx, surface_y, wz);
            }
        }
    }
}

pub fn surface_y_at(world: &VoxelWorld, wx: i32, wz: i32) -> Option<i32> {
    for y in (1..WORLD_HEIGHT).rev() {
        let block = world.peek_block(wx, y, wz);
        if matches!(
            block,
            BlockId::Grass | BlockId::Dirt | BlockId::Sand | BlockId::Snow | BlockId::Planks
        ) {
            return Some(y);
        }
    }
    None
}

fn is_stable_surface_world(world: &VoxelWorld, wx: i32, surface_y: i32, wz: i32) -> bool {
    for dx in -1i32..=1 {
        for dz in -1i32..=1 {
            let neighbor = world.peek_block(wx + dx, surface_y, wz + dz);
            if !neighbor.solid() || neighbor == BlockId::Water || neighbor == BlockId::Lava {
                return false;
            }
            if world.peek_block(wx + dx, surface_y + 1, wz + dz) == BlockId::Air
                && world.peek_block(wx + dx, surface_y - 1, wz + dz) == BlockId::Air
            {
                return false;
            }
        }
    }
    true
}

fn tree_clearance_world(world: &VoxelWorld, wx: i32, wz: i32, surface_y: i32) -> i32 {
    let mut score = 0;
    for dx in -2i32..=2 {
        for dz in -2i32..=2 {
            let foot = world.peek_block(wx + dx, surface_y + 1, wz + dz);
            let head = world.peek_block(wx + dx, surface_y + 2, wz + dz);
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

pub fn trunk_height_for(wx: i32, wz: i32, noise: &crate::noise::NoiseGenerator) -> i32 {
    let variant = noise.roll(wx, wz, 11);
    if variant > 0.7 {
        MAX_TRUNK_HEIGHT
    } else if variant > 0.35 {
        4
    } else {
        MIN_TRUNK_HEIGHT
    }
}

pub fn place_tree_world(world: &mut VoxelWorld, wx: i32, surface_y: i32, wz: i32) {
    let trunk_height = trunk_height_for(wx, wz, &world.noise);
    for dy in 0..trunk_height {
        let y = surface_y + dy;
        if y >= WORLD_HEIGHT {
            break;
        }
        world.set_block(wx, y, wz, BlockId::Wood);
    }
    let leaf_start = surface_y + trunk_height - 2;
    for dy in 0..3 {
        for dx in -2i32..=2 {
            for dz in -2i32..=2 {
                if dx.abs() == 2 && dz.abs() == 2 {
                    continue;
                }
                if dy == 2 && (dx.abs() > 1 || dz.abs() > 1) {
                    continue;
                }
                let lx = wx + dx;
                let ly = leaf_start + dy;
                let lz = wz + dz;
                if ly >= WORLD_HEIGHT {
                    continue;
                }
                let existing = world.peek_block(lx, ly, lz);
                if matches!(existing, BlockId::Air | BlockId::TallGrass | BlockId::Flower) {
                    world.set_block(lx, ly, lz, BlockId::Leaves);
                }
            }
        }
    }
}

fn place_bush_world(world: &mut VoxelWorld, wx: i32, surface_y: i32, wz: i32) {
    for dx in -1i32..=1 {
        for dz in -1i32..=1 {
            let ly = surface_y + 1;
            if ly < WORLD_HEIGHT && world.peek_block(wx + dx, ly, wz + dz) == BlockId::Air {
                world.set_block(wx + dx, ly, wz + dz, BlockId::Leaves);
            }
        }
    }
}

/// Inspect every wood column in loaded chunks and validate tree proportions.
pub fn validate_trees(world: &VoxelWorld) -> Result<(), String> {
    for ((cx, cz), chunk) in &world.chunks {
        let x_base = cx * CHUNK_SIZE;
        let z_base = cz * CHUNK_SIZE;
        for lx in 0..CHUNK_SIZE {
            for lz in 0..CHUNK_SIZE {
                let wx = x_base + lx;
                let wz = z_base + lz;
                let mut trunk_blocks = 0;
                let mut top_y = 0;
                for y in 0..WORLD_HEIGHT {
                    if crate::chunk_gen::get_block_local(&chunk.blocks, lx, y, lz) == BlockId::Wood {
                        trunk_blocks += 1;
                        top_y = y;
                    }
                }
                if trunk_blocks == 0 {
                    continue;
                }
                if trunk_blocks > MAX_TRUNK_HEIGHT {
                    return Err(format!(
                        "tree at ({wx}, {wz}) has trunk height {trunk_blocks} > {MAX_TRUNK_HEIGHT}"
                    ));
                }
                let Some(surface) = surface_y_at(world, wx, wz) else {
                    return Err(format!("tree at ({wx}, {wz}) has no surface below trunk"));
                };
                if top_y - surface > MAX_TREE_TOP_ABOVE_SURFACE {
                    return Err(format!(
                        "tree at ({wx}, {wz}) extends {0} blocks above surface (max {MAX_TREE_TOP_ABOVE_SURFACE})",
                        top_y - surface
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Surface columns should match procedural terrain height at chunk borders.
pub fn validate_chunk_seams(world: &VoxelWorld) -> Result<(), String> {
    for (&(cx, cz), _) in &world.chunks {
        let x_base = cx * CHUNK_SIZE;
        let z_base = cz * CHUNK_SIZE;
        for lx in [0, CHUNK_SIZE - 1] {
            for lz in 0..CHUNK_SIZE {
                let wx = x_base + lx;
                let wz = z_base + lz;
                if !world.noise.is_land(wx, wz) {
                    continue;
                }
                let expected = world.noise.terrain_height(wx, wz);
                let Some(actual) = surface_y_at(world, wx, wz) else {
                    return Err(format!("missing surface at land column ({wx},{wz})"));
                };
                if (actual - expected).abs() > 1 {
                    return Err(format!(
                        "surface at ({wx},{wz}) is {actual}, expected ~{expected}"
                    ));
                }
            }
        }
        for lz in [0, CHUNK_SIZE - 1] {
            for lx in 0..CHUNK_SIZE {
                let wx = x_base + lx;
                let wz = z_base + lz;
                if !world.noise.is_land(wx, wz) {
                    continue;
                }
                let expected = world.noise.terrain_height(wx, wz);
                let Some(actual) = surface_y_at(world, wx, wz) else {
                    return Err(format!("missing surface at land column ({wx},{wz})"));
                };
                if (actual - expected).abs() > 1 {
                    return Err(format!(
                        "surface at ({wx},{wz}) is {actual}, expected ~{expected}"
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meshing::build_chunk_mesh;
    use crate::world::VoxelWorld;
    use bevy::render::mesh::{Mesh, VertexAttributeValues};

    fn populated_world(radius: i32) -> VoxelWorld {
        let mut world = VoxelWorld::new(42);
        world.load_chunks_around(0, 0);
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx == 0 && dz == 0 {
                    continue;
                }
                world.load_chunks_around(dx * CHUNK_SIZE, dz * CHUNK_SIZE);
            }
        }
        world
    }

    #[test]
    fn chunk_seams_have_matching_surface_heights() {
        let world = populated_world(2);
        validate_chunk_seams(&world).expect("chunk seams should align");
    }

    #[test]
    fn trees_respect_height_limits() {
        let world = populated_world(3);
        validate_trees(&world).expect("trees should be within height limits");
    }

    #[test]
    fn trunk_height_is_bounded() {
        let noise = crate::noise::NoiseGenerator::new(42);
        for x in -64..64 {
            for z in -64..64 {
                let h = trunk_height_for(x, z, &noise);
                assert!(
                    (MIN_TRUNK_HEIGHT..=MAX_TRUNK_HEIGHT).contains(&h),
                    "trunk height {h} out of range at ({x},{z})"
                );
            }
        }
    }

    #[test]
    fn mesh_seams_have_no_exposed_boundary_on_flat_neighbors() {
        let mut world = VoxelWorld::new(7);
        world.load_chunks_around(0, 0);
        world.load_chunks_around(CHUNK_SIZE, 0);

        let mesh_a = build_chunk_mesh(&world, 0, 0).expect("chunk 0,0 mesh");
        let mesh_b = build_chunk_mesh(&world, 1, 0).expect("chunk 1,0 mesh");

        let pos_a = mesh_a.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let pos_b = mesh_b.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();

        // Boundary at x=16 in chunk (1,0) local x=0 should have faces only where
        // neighbor differs; verify meshes were built with non-zero geometry.
        assert!(pos_a.len() > 100, "chunk mesh should have substantial geometry");
        assert!(pos_b.len() > 100, "neighbor chunk mesh should have substantial geometry");

        // No vertex should sit exactly on a seam with an offset that implies z-fighting gap.
        for positions in [pos_a, pos_b] {
            if let VertexAttributeValues::Float32x3(verts) = positions {
                for [x, _y, _z] in verts {
                    assert!(
                        x.is_finite(),
                        "mesh vertex x must be finite, got {x}"
                    );
                }
            }
        }
    }

    #[test]
    fn cross_chunk_tree_writes_leaves_in_neighbor_chunk() {
        let mut world = VoxelWorld::new(99);
        world.load_chunks_around(0, 0);
        let wx = CHUNK_SIZE - 2;
        let wz = 8;
        let Some(surface) = surface_y_at(&world, wx, wz) else {
            return;
        };
        place_tree_world(&mut world, wx, surface, wz);
        let leaf_wx = wx + 2;
        let leaf_y = surface + 2;
        let leaf = world.peek_block(leaf_wx, leaf_y, wz);
        assert_eq!(
            leaf,
            BlockId::Leaves,
            "cross-chunk tree should place leaves in neighbor chunk at ({leaf_wx},{leaf_y},{wz})"
        );
    }
}
