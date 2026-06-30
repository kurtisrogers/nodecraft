use crate::blocks::BlockId;
use crate::chunk_gen::{get_block_local, set_block_local};
use crate::config::{CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};
use crate::world::VoxelWorld;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FoliageKind {
    Grass,
    Flower,
    Wheat,
}

#[derive(Component)]
pub struct FoliageSprite {
    pub kind: FoliageKind,
}

#[derive(Resource)]
pub struct FoliageAssets {
    pub quad: Handle<Mesh>,
    pub grass: Handle<StandardMaterial>,
    pub flower: Handle<StandardMaterial>,
    pub wheat: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct FoliageChunkMap {
    pub entities: HashMap<(i32, i32), Vec<Entity>>,
}

pub fn setup_decorations(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(FoliageAssets {
        quad: meshes.add(build_foliage_quad()),
        grass: materials.add(StandardMaterial {
            base_color: Color::srgba(0.32, 0.68, 0.24, 0.75),
            emissive: LinearRgba::new(0.08, 0.18, 0.05, 1.0),
            alpha_mode: AlphaMode::Mask(0.45),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        flower: materials.add(StandardMaterial {
            base_color: Color::srgba(0.98, 0.82, 0.22, 0.8),
            emissive: LinearRgba::new(0.15, 0.12, 0.03, 1.0),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        wheat: materials.add(StandardMaterial {
            base_color: Color::srgba(0.82, 0.72, 0.22, 0.85),
            emissive: LinearRgba::new(0.1, 0.08, 0.02, 1.0),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    });
    commands.init_resource::<FoliageChunkMap>();
}

fn build_foliage_quad() -> Mesh {
    let w = 0.22;
    let h = 0.38;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for yaw in [0.0_f32, std::f32::consts::FRAC_PI_2] {
        let (sin, cos) = yaw.sin_cos();
        let corners = [
            [-w * cos, 0.0, w * sin],
            [w * cos, 0.0, -w * sin],
            [w * cos, h, -w * sin],
            [-w * cos, h, w * sin],
        ];
        let base = positions.len() as u32;
        for c in corners {
            positions.push(c);
            normals.push([0.0, 1.0, 0.0]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn sync_chunk_decorations(
    commands: &mut Commands,
    world: &mut VoxelWorld,
    assets: &FoliageAssets,
    map: &mut FoliageChunkMap,
    chunk_x: i32,
    chunk_z: i32,
) {
    purge_voxel_foliage_in_chunk(world, chunk_x, chunk_z);
    if let Some(old) = map.entities.remove(&(chunk_x, chunk_z)) {
        for entity in old {
            commands.entity(entity).despawn();
        }
    }

    let mut spawned = Vec::new();
    let base_x = chunk_x * CHUNK_SIZE;
    let base_z = chunk_z * CHUNK_SIZE;
    let chunk = match world.chunks.get(&(chunk_x, chunk_z)) {
        Some(c) => c,
        None => return,
    };

    let mut rng = SmallRng::seed_from_u64(
        (chunk_x as u64)
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add(chunk_z as u64),
    );

    for lx in 0..CHUNK_SIZE {
        for lz in 0..CHUNK_SIZE {
            let wx = base_x + lx;
            let wz = base_z + lz;
            let Some(surface_y) = surface_y_local(chunk, lx, lz) else {
                continue;
            };
            if surface_y <= SEA_LEVEL + 1 || !world.noise.is_land(wx, wz) {
                continue;
            }
            if world.noise.is_in_settlement(wx, wz) || world.noise.is_in_volcano(wx, wz) {
                continue;
            }
            let surface = get_block_local(&chunk.blocks, lx, surface_y, lz);
            if !matches!(surface, BlockId::Grass | BlockId::Dirt | BlockId::Sand | BlockId::Snow) {
                continue;
            }

            let kind = if world.noise.should_place_flower(wx, wz) {
                Some(FoliageKind::Flower)
            } else if world.noise.should_place_tall_grass(wx, wz) {
                Some(FoliageKind::Grass)
            } else {
                None
            };
            let Some(kind) = kind else { continue };

            let (mat, scale, y_offset) = match kind {
                FoliageKind::Grass => (&assets.grass, rng.gen_range(0.55..0.8), 0.02),
                FoliageKind::Flower => (&assets.flower, rng.gen_range(0.5..0.75), 0.02),
                FoliageKind::Wheat => (&assets.wheat, rng.gen_range(0.6..0.85), 0.02),
            };
            let yaw = rng.gen_range(0.0..std::f32::consts::TAU);
            let pos = Vec3::new(
                wx as f32 + 0.5,
                surface_y as f32 + 1.0 + y_offset,
                wz as f32 + 0.5,
            );

            let entity = commands
                .spawn((
                    FoliageSprite { kind },
                    Mesh3d(assets.quad.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_y(yaw))
                        .with_scale(Vec3::splat(scale)),
                ))
                .id();
            spawned.push(entity);
        }
    }

    map.entities.insert((chunk_x, chunk_z), spawned);
    sync_wheat_patches_in_chunk(
        commands,
        assets,
        map,
        world,
        chunk_x,
        chunk_z,
    );
}

fn purge_voxel_foliage_in_chunk(world: &mut VoxelWorld, chunk_x: i32, chunk_z: i32) {
    let Some(chunk) = world.chunks.get_mut(&(chunk_x, chunk_z)) else {
        return;
    };
    let mut changed = false;
    for lx in 0..CHUNK_SIZE {
        for lz in 0..CHUNK_SIZE {
            for ly in 0..WORLD_HEIGHT {
                let block = get_block_local(&chunk.blocks, lx, ly, lz);
                if block.is_cross_decoration() {
                    set_block_local(
                        &mut chunk.blocks,
                        lx,
                        ly,
                        lz,
                        BlockId::Air,
                    );
                    let wx = chunk_x * CHUNK_SIZE + lx;
                    let wz = chunk_z * CHUNK_SIZE + lz;
                    world.modifications.insert((wx, ly, wz), BlockId::Air);
                    changed = true;
                }
            }
        }
    }
    if changed {
        chunk.dirty = true;
    }
}

fn sync_wheat_patches_in_chunk(
    commands: &mut Commands,
    assets: &FoliageAssets,
    map: &mut FoliageChunkMap,
    world: &VoxelWorld,
    chunk_x: i32,
    chunk_z: i32,
) {
    let base_x = chunk_x * CHUNK_SIZE;
    let base_z = chunk_z * CHUNK_SIZE;
    let end_x = base_x + CHUNK_SIZE;
    let end_z = base_z + CHUNK_SIZE;

    for &(origin_x, origin_y, origin_z, size) in &world.wheat_patches {
        let patch_end_x = origin_x + size;
        let patch_end_z = origin_z + size;
        if patch_end_x <= base_x || origin_x >= end_x || patch_end_z <= base_z || origin_z >= end_z {
            continue;
        }

        let mut rng = SmallRng::seed_from_u64(
            (origin_x as u64)
                .wrapping_mul(0x517c_c1b7)
                .wrapping_add(origin_z as u64)
                .wrapping_add(chunk_x as u64)
                .wrapping_add(chunk_z as u64),
        );
        for dx in 1..size - 1 {
            for dz in 1..size - 1 {
                if (dx + dz) % 2 != 0 {
                    continue;
                }
                let wx = origin_x + dx;
                let wz = origin_z + dz;
                if wx < base_x || wx >= end_x || wz < base_z || wz >= end_z {
                    continue;
                }
                let yaw = rng.gen_range(0.0..std::f32::consts::TAU);
                let entity = commands
                    .spawn((
                        FoliageSprite {
                            kind: FoliageKind::Wheat,
                        },
                        Mesh3d(assets.quad.clone()),
                        MeshMaterial3d(assets.wheat.clone()),
                        Transform::from_translation(Vec3::new(
                            wx as f32 + 0.5,
                            origin_y as f32 + 1.0,
                            wz as f32 + 0.5,
                        ))
                        .with_rotation(Quat::from_rotation_y(yaw))
                        .with_scale(Vec3::splat(rng.gen_range(0.85..1.1))),
                    ))
                    .id();
                map.entities.entry((chunk_x, chunk_z)).or_default().push(entity);
            }
        }
    }
}

pub fn billboard_foliage(
    player: Res<crate::player::PlayerState>,
    mut query: Query<&mut Transform, With<FoliageSprite>>,
) {
    let camera_pos = player.position + Vec3::Y * crate::config::EYE_HEIGHT;
    for mut transform in query.iter_mut() {
        let to_camera = camera_pos - transform.translation;
        if to_camera.length_squared() > 1e-6 {
            transform.rotation = Quat::from_rotation_y(to_camera.x.atan2(to_camera.z));
        }
    }
}

pub fn spawn_wheat_sprites(
    commands: &mut Commands,
    assets: &FoliageAssets,
    map: &mut FoliageChunkMap,
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    size: i32,
) {
    let mut rng = SmallRng::seed_from_u64(
        (origin_x as u64)
            .wrapping_mul(0x517c_c1b7)
            .wrapping_add(origin_z as u64),
    );
    for dx in 1..size - 1 {
        for dz in 1..size - 1 {
            if (dx + dz) % 2 != 0 {
                continue;
            }
            let wx = origin_x + dx;
            let wz = origin_z + dz;
            let cx = wx.div_euclid(CHUNK_SIZE);
            let cz = wz.div_euclid(CHUNK_SIZE);
            let yaw = rng.gen_range(0.0..std::f32::consts::TAU);
            let entity = commands
                .spawn((
                    FoliageSprite {
                        kind: FoliageKind::Wheat,
                    },
                    Mesh3d(assets.quad.clone()),
                    MeshMaterial3d(assets.wheat.clone()),
                    Transform::from_translation(Vec3::new(
                        wx as f32 + 0.5,
                        origin_y as f32 + 1.0,
                        wz as f32 + 0.5,
                    ))
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(rng.gen_range(0.85..1.1))),
                ))
                .id();
            map.entities.entry((cx, cz)).or_default().push(entity);
        }
    }
}

fn surface_y_local(chunk: &crate::chunk_gen::ChunkData, lx: i32, lz: i32) -> Option<i32> {
    for y in (1..WORLD_HEIGHT).rev() {
        let block = get_block_local(&chunk.blocks, lx, y, lz);
        if block.solid() && block != BlockId::Bedrock {
            return Some(y);
        }
    }
    None
}

pub fn retain_foliage_chunks(
    commands: &mut Commands,
    map: &mut FoliageChunkMap,
    keep: &std::collections::HashSet<(i32, i32)>,
) {
    map.entities.retain(|key, entities| {
        if keep.contains(key) {
            true
        } else {
            for entity in entities.drain(..) {
                commands.entity(entity).despawn();
            }
            false
        }
    });
}
