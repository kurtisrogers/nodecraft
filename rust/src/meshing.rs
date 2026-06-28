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
    pending: std::collections::HashSet<(i32, i32)>,
}

impl RemeshQueue {
    pub fn push(&mut self, key: (i32, i32)) {
        if self.pending.insert(key) {
            self.keys.push(key);
        }
    }

    pub fn pop_nearest(&mut self, player_chunk: (i32, i32)) -> Option<(i32, i32)> {
        if self.keys.is_empty() {
            return None;
        }
        let (pcx, pcz) = player_chunk;
        let best_idx = self
            .keys
            .iter()
            .enumerate()
            .min_by_key(|(_, (cx, cz))| {
                let dx = cx - pcx;
                let dz = cz - pcz;
                dx * dx + dz * dz
            })
            .map(|(idx, _)| idx)?;
        let key = self.keys.swap_remove(best_idx);
        self.pending.remove(&key);
        Some(key)
    }

    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    fn retain_loaded(&mut self, loaded: &std::collections::HashSet<(i32, i32)>) {
        self.keys.retain(|key| loaded.contains(key));
        self.pending.retain(|key| loaded.contains(key));
    }
}

#[derive(Resource, Default)]
pub struct ChunkEntityMap {
    pub entities: std::collections::HashMap<(i32, i32), Entity>,
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

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(4096);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(4096);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(4096);
    let mut indices: Vec<u32> = Vec::with_capacity(6144);

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
                if block.is_cross_decoration() {
                    push_cross_decoration(
                        &mut positions,
                        &mut normals,
                        &mut colors,
                        &mut indices,
                        x,
                        y,
                        z,
                        block,
                    );
                    continue;
                }
                if !block.solid() {
                    continue;
                }
                for (dir, face) in dirs {
                    let neighbor = neighbor_block(world, chunk_x, chunk_z, x + dir[0], y + dir[1], z + dir[2]);
                    if face_visible(block, neighbor) {
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

fn face_visible(block: BlockId, neighbor: BlockId) -> bool {
    if neighbor == BlockId::Air {
        return true;
    }
    if block == neighbor {
        return false;
    }
    neighbor.transparent()
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
    let rgba = if cfg!(target_arch = "wasm32") {
        let srgb = color.to_srgba();
        [srgb.red, srgb.green, srgb.blue, srgb.alpha]
    } else {
        let linear = color.to_linear();
        [linear.red, linear.green, linear.blue, linear.alpha]
    };
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

fn push_cross_decoration(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    x: i32,
    y: i32,
    z: i32,
    block: BlockId,
) {
    let fx = x as f32;
    let fy = y as f32;
    let fz = z as f32;
    let color = block.color(Face::Side);
    let rgba = if cfg!(target_arch = "wasm32") {
        let srgb = color.to_srgba();
        [srgb.red, srgb.green, srgb.blue, srgb.alpha]
    } else {
        let linear = color.to_linear();
        [linear.red, linear.green, linear.blue, linear.alpha]
    };

    let quads: [([[f32; 3]; 4], [f32; 3]); 2] = [
        (
            [
                [fx, fy, fz + 0.5],
                [fx + 1.0, fy, fz + 0.5],
                [fx + 1.0, fy + 1.0, fz + 0.5],
                [fx, fy + 1.0, fz + 0.5],
            ],
            [0.0, 0.0, 1.0],
        ),
        (
            [
                [fx + 0.5, fy, fz],
                [fx + 0.5, fy, fz + 1.0],
                [fx + 0.5, fy + 1.0, fz + 1.0],
                [fx + 0.5, fy + 1.0, fz],
            ],
            [1.0, 0.0, 0.0],
        ),
    ];

    for (verts, normal) in quads {
        let base = positions.len() as u32;
        for v in verts {
            positions.push(v);
            normals.push(normal);
            colors.push(rgba);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

pub fn sync_chunk_meshes(
    mut commands: Commands,
    mut world: ResMut<VoxelWorldResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_mat: Res<ChunkMaterial>,
    mut queue: ResMut<RemeshQueue>,
    mut entity_map: ResMut<ChunkEntityMap>,
) {
    let mut budget = if cfg!(target_arch = "wasm32") { 12 } else { 6 };
    let player_chunk = world.player_chunk;
    let loaded: std::collections::HashSet<_> = world.loaded_chunks.iter().copied().collect();

    entity_map.entities.retain(|key, entity| {
        if loaded.contains(key) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    queue.retain_loaded(&loaded);

    for &(cx, cz) in &world.loaded_chunks {
        if let Some(chunk) = world.inner.chunks.get(&(cx, cz)) {
            if chunk.dirty {
                queue.push((cx, cz));
            }
        }
    }

    while budget > 0 {
        let Some((cx, cz)) = queue.pop_nearest(player_chunk) else { break };
        budget -= 1;

        let Some(mesh) = build_chunk_mesh(&world.inner, cx, cz) else {
            if let Some(entity) = entity_map.entities.remove(&(cx, cz)) {
                commands.entity(entity).despawn();
            }
            continue;
        };

        if let Some(chunk) = world.inner.chunks.get_mut(&(cx, cz)) {
            chunk.dirty = false;
        }

        let handle = meshes.add(mesh);
        if let Some(&entity) = entity_map.entities.get(&(cx, cz)) {
            commands.entity(entity).insert(Mesh3d(handle));
        } else {
            let entity = commands
                .spawn((
                    Mesh3d(handle),
                    MeshMaterial3d(chunk_mat.0.clone()),
                    Transform::from_xyz(
                        (cx * CHUNK_SIZE) as f32,
                        0.0,
                        (cz * CHUNK_SIZE) as f32,
                    ),
                    ChunkMesh { chunk_x: cx, chunk_z: cz },
                ))
                .id();
            entity_map.entities.insert((cx, cz), entity);
        }
    }

    #[cfg(target_arch = "wasm32")]
    if !entity_map.entities.is_empty() {
        crate::wasm_entry::hide_loading_overlay_if_ready(entity_map.entities.len());
    }
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

pub fn update_world_chunks(
    mut world: ResMut<VoxelWorldResource>,
    mut queue: ResMut<RemeshQueue>,
    player: Res<PlayerState>,
    mut wasm_settlements: Local<bool>,
) {
    let px = player.position.x.floor() as i32;
    let pz = player.position.z.floor() as i32;

    #[cfg(target_arch = "wasm32")]
    if !*wasm_settlements {
        *wasm_settlements = true;
        crate::chunk_gen::ensure_settlements_near(&mut world.inner, px, pz, 96);
        for &(cx, cz) in &world.loaded_chunks.clone() {
            if let Some(chunk) = world.inner.chunks.get_mut(&(cx, cz)) {
                chunk.dirty = true;
            }
            queue.push((cx, cz));
        }
    }

    let new_chunk = (px.div_euclid(CHUNK_SIZE), pz.div_euclid(CHUNK_SIZE));
    if new_chunk != world.player_chunk {
        world.player_chunk = new_chunk;
        let radius = world.inner.render_distance * CHUNK_SIZE + 64;
        crate::chunk_gen::ensure_settlements_near(&mut world.inner, px, pz, radius);
        let previous: std::collections::HashSet<_> = world.loaded_chunks.iter().copied().collect();
        world.loaded_chunks = world.inner.load_chunks_around(px, pz);
        let loaded_set: std::collections::HashSet<_> = world.loaded_chunks.iter().copied().collect();
        world.inner.retain_chunks(&loaded_set);
        let new_chunks: Vec<_> = world
            .loaded_chunks
            .iter()
            .copied()
            .filter(|key| !previous.contains(key))
            .collect();
        for (cx, cz) in new_chunks {
            queue.push((cx, cz));
            for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)] {
                let neighbor = (cx + dx, cz + dz);
                if let Some(chunk) = world.inner.chunks.get_mut(&neighbor) {
                    chunk.dirty = true;
                    queue.push(neighbor);
                }
            }
        }
    }
}

/// Immediately mesh chunks around the player so the world is visible on frame 1.
pub fn bootstrap_player_meshes(
    mut commands: Commands,
    mut world: ResMut<VoxelWorldResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_mat: Res<ChunkMaterial>,
    mut entity_map: ResMut<ChunkEntityMap>,
    mut queue: ResMut<RemeshQueue>,
) {
    let (pcx, pcz) = world.player_chunk;
    let radius = if cfg!(target_arch = "wasm32") { 2 } else { 1 };
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            mesh_chunk_entity(
                &mut commands,
                &mut world,
                &mut meshes,
                &chunk_mat,
                &mut entity_map,
                &mut queue,
                pcx + dx,
                pcz + dz,
            );
        }
    }
    if cfg!(target_arch = "wasm32") && !entity_map.entities.is_empty() {
        crate::wasm_entry::hide_loading_overlay_if_ready(entity_map.entities.len());
    }
}

fn mesh_chunk_entity(
    commands: &mut Commands,
    world: &mut VoxelWorldResource,
    meshes: &mut Assets<Mesh>,
    chunk_mat: &ChunkMaterial,
    entity_map: &mut ChunkEntityMap,
    queue: &mut RemeshQueue,
    cx: i32,
    cz: i32,
) {
    if !world.loaded_chunks.contains(&(cx, cz)) {
        return;
    }
    let Some(mesh) = build_chunk_mesh(&world.inner, cx, cz) else {
        return;
    };
    if let Some(chunk) = world.inner.chunks.get_mut(&(cx, cz)) {
        chunk.dirty = false;
    }
    let handle = meshes.add(mesh);
    if let Some(&entity) = entity_map.entities.get(&(cx, cz)) {
        commands.entity(entity).insert(Mesh3d(handle));
    } else {
        let entity = commands
            .spawn((
                Mesh3d(handle),
                MeshMaterial3d(chunk_mat.0.clone()),
                Transform::from_xyz((cx * CHUNK_SIZE) as f32, 0.0, (cz * CHUNK_SIZE) as f32),
                ChunkMesh { chunk_x: cx, chunk_z: cz },
            ))
            .id();
        entity_map.entities.insert((cx, cz), entity);
    }
    queue.push((cx, cz));
}

use crate::player::PlayerState;
