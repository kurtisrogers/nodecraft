use crate::blocks::{BlockId, Face};
use crate::chunk_gen::get_block_local;
use crate::config::{CHUNK_SIZE, WORLD_HEIGHT};
use crate::world::VoxelWorld;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

#[derive(Component)]
pub struct ChunkMesh {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<StandardMaterial>);

#[derive(Resource, Default)]
pub struct RemeshQueue {
    pub keys: Vec<(i32, i32)>,
}

pub fn build_chunk_mesh(world: &VoxelWorld, chunk_x: i32, chunk_z: i32) -> Option<Mesh> {
    let chunk = world.chunks.get(&(chunk_x, chunk_z))?;
    let mut min_y = WORLD_HEIGHT;
    let mut max_y = 0;
    let mut has_blocks = false;

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for y in 0..WORLD_HEIGHT {
                if get_block_local(&chunk.blocks, x, y, z) != BlockId::Air {
                    has_blocks = true;
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
    }
    if !has_blocks {
        return None;
    }

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let dirs: [([i32; 3], Face); 6] = [
        ([0, 1, 0], Face::Top),
        ([0, -1, 0], Face::Bottom),
        ([1, 0, 0], Face::Side),
        ([-1, 0, 0], Face::Side),
        ([0, 0, 1], Face::Side),
        ([0, 0, -1], Face::Side),
    ];

    for x in 0..CHUNK_SIZE {
        for y in min_y..=max_y {
            for z in 0..CHUNK_SIZE {
                let block = get_block_local(&chunk.blocks, x, y, z);
                if block == BlockId::Air {
                    continue;
                }
                for (dir, face) in dirs {
                    let neighbor = neighbor_block(world, chunk_x, chunk_z, x + dir[0], y + dir[1], z + dir[2]);
                    if neighbor.transparent() {
                        push_face(
                            &mut positions,
                            &mut normals,
                            &mut colors,
                            &mut indices,
                            x,
                            y,
                            z,
                            dir,
                            face,
                            block,
                        );
                    }
                }
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

fn neighbor_block(world: &VoxelWorld, chunk_x: i32, chunk_z: i32, lx: i32, ly: i32, lz: i32) -> BlockId {
    if ly < 0 || ly >= WORLD_HEIGHT {
        return BlockId::Air;
    }
    let mut cx = chunk_x;
    let mut cz = chunk_z;
    let mut nx = lx;
    let mut nz = lz;
    if nx < 0 {
        cx -= 1;
        nx += CHUNK_SIZE;
    } else if nx >= CHUNK_SIZE {
        cx += 1;
        nx -= CHUNK_SIZE;
    }
    if nz < 0 {
        cz -= 1;
        nz += CHUNK_SIZE;
    } else if nz >= CHUNK_SIZE {
        cz += 1;
        nz -= CHUNK_SIZE;
    }
    world.peek_block(cx * CHUNK_SIZE + nx, ly, cz * CHUNK_SIZE + nz)
}

fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    x: i32,
    y: i32,
    z: i32,
    dir: [i32; 3],
    face: Face,
    block: BlockId,
) {
    let base = positions.len() as u32;
    let color = block.color(face);
    let rgba = color.to_srgba();
    let rgba = [rgba.red, rgba.green, rgba.blue, rgba.alpha];
    let normal = [dir[0] as f32, dir[1] as f32, dir[2] as f32];

    let verts: [[f32; 3]; 4] = match (dir[0], dir[1], dir[2]) {
        (0, 1, 0) => [
            [x as f32, y as f32 + 1.0, z as f32],
            [x as f32 + 1.0, y as f32 + 1.0, z as f32],
            [x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0],
            [x as f32, y as f32 + 1.0, z as f32 + 1.0],
        ],
        (0, -1, 0) => [
            [x as f32, y as f32, z as f32 + 1.0],
            [x as f32 + 1.0, y as f32, z as f32 + 1.0],
            [x as f32 + 1.0, y as f32, z as f32],
            [x as f32, y as f32, z as f32],
        ],
        (1, 0, 0) => [
            [x as f32 + 1.0, y as f32, z as f32],
            [x as f32 + 1.0, y as f32, z as f32 + 1.0],
            [x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0],
            [x as f32 + 1.0, y as f32 + 1.0, z as f32],
        ],
        (-1, 0, 0) => [
            [x as f32, y as f32, z as f32 + 1.0],
            [x as f32, y as f32, z as f32],
            [x as f32, y as f32 + 1.0, z as f32],
            [x as f32, y as f32 + 1.0, z as f32 + 1.0],
        ],
        (0, 0, 1) => [
            [x as f32 + 1.0, y as f32, z as f32 + 1.0],
            [x as f32, y as f32, z as f32 + 1.0],
            [x as f32, y as f32 + 1.0, z as f32 + 1.0],
            [x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0],
        ],
        _ => [
            [x as f32, y as f32, z as f32],
            [x as f32 + 1.0, y as f32, z as f32],
            [x as f32 + 1.0, y as f32 + 1.0, z as f32],
            [x as f32, y as f32 + 1.0, z as f32],
        ],
    };

    for v in verts {
        positions.push(v);
        normals.push(normal);
        colors.push(rgba);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

pub fn sync_chunk_meshes(
    mut commands: Commands,
    mut world: ResMut<VoxelWorldResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_mat: Res<ChunkMaterial>,
    mut queue: ResMut<RemeshQueue>,
    existing: Query<(Entity, &ChunkMesh)>,
) {
    let mut budget = if cfg!(target_arch = "wasm32") { 2 } else { 4 };
    let player_chunk = world.player_chunk;

    if queue.keys.is_empty() {
        for &(cx, cz) in &world.loaded_chunks {
            if let Some(chunk) = world.inner.chunks.get(&(cx, cz)) {
                if chunk.dirty {
                    queue.keys.push((cx, cz));
                }
            }
        }
    }

    while budget > 0 {
        let Some((cx, cz)) = queue.keys.pop() else { break };
        budget -= 1;

        for (entity, mesh) in existing.iter() {
            if mesh.chunk_x == cx && mesh.chunk_z == cz {
                commands.entity(entity).despawn();
            }
        }

        if let Some(mesh) = build_chunk_mesh(&world.inner, cx, cz) {
            let handle = meshes.add(mesh);
            commands.spawn((
                Mesh3d(handle),
                MeshMaterial3d(chunk_mat.0.clone()),
                Transform::from_xyz(
                    (cx * CHUNK_SIZE) as f32,
                    0.0,
                    (cz * CHUNK_SIZE) as f32,
                ),
                ChunkMesh { chunk_x: cx, chunk_z: cz },
            ));
        }
        if let Some(chunk) = world.inner.chunks.get_mut(&(cx, cz)) {
            chunk.dirty = false;
        }
    }

    let _ = player_chunk;
}

#[derive(Resource)]
pub struct VoxelWorldResource {
    pub inner: VoxelWorld,
    pub loaded_chunks: Vec<(i32, i32)>,
    pub player_chunk: (i32, i32),
}

impl VoxelWorldResource {
    pub fn new(seed: u32) -> Self {
        Self {
            inner: VoxelWorld::new(seed),
            loaded_chunks: Vec::new(),
            player_chunk: (0, 0),
        }
    }
}

pub fn update_world_chunks(mut world: ResMut<VoxelWorldResource>, player: Res<PlayerState>) {
    let px = player.position.x.floor() as i32;
    let pz = player.position.z.floor() as i32;
    let new_chunk = (px.div_euclid(CHUNK_SIZE), pz.div_euclid(CHUNK_SIZE));
    if new_chunk != world.player_chunk {
        world.player_chunk = new_chunk;
        let radius = world.inner.render_distance * CHUNK_SIZE + 64;
        crate::chunk_gen::ensure_settlements_near(&mut world.inner, px, pz, radius);
        world.loaded_chunks = world.inner.load_chunks_around(px, pz);
        world.inner.unload_distant_chunks(px, pz);
    }
}

use crate::player::PlayerState;
