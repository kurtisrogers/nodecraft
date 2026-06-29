use crate::player::PlayerState;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const CLOUD_COUNT: usize = 10;
const WASM_CLOUD_COUNT: usize = 6;
const CLOUD_VARIANTS: usize = 3;
const CLOUD_HEIGHT_MIN: f32 = 68.0;
const CLOUD_HEIGHT_MAX: f32 = 96.0;
const CLOUD_SPREAD: f32 = 110.0;
const CLOUD_UPDATE_INTERVAL: u32 = 2;

#[derive(Component)]
pub struct VoxelCloud {
    pub offset: Vec3,
    pub drift: Vec2,
}

#[derive(Resource, Default)]
pub(crate) struct CloudTick(u32);

struct CloudProfile {
    count: usize,
    width: u32,
    height: u32,
    depth: u32,
    top_faces_only: bool,
    alpha_mode: AlphaMode,
}

fn cloud_profile() -> CloudProfile {
    if cfg!(target_arch = "wasm32") {
        CloudProfile {
            count: WASM_CLOUD_COUNT,
            width: 5,
            height: 3,
            depth: 5,
            top_faces_only: true,
            alpha_mode: AlphaMode::Mask(0.45),
        }
    } else {
        CloudProfile {
            count: CLOUD_COUNT,
            width: 7,
            height: 4,
            depth: 7,
            top_faces_only: true,
            alpha_mode: AlphaMode::Blend,
        }
    }
}

pub fn setup_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let profile = cloud_profile();
    let cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.97, 0.98, 1.0, 0.92),
        emissive: LinearRgba::new(0.15, 0.16, 0.2, 1.0),
        unlit: true,
        alpha_mode: profile.alpha_mode,
        ..default()
    });

    let variant_handles: Vec<Handle<Mesh>> = (0..CLOUD_VARIANTS)
        .map(|seed| {
            meshes.add(build_cloud_mesh(
                seed as u32 + 1,
                profile.width,
                profile.height,
                profile.depth,
                profile.top_faces_only,
            ))
        })
        .collect();

    let mut rng = SmallRng::seed_from_u64(0x_c10d_5eed);
    for i in 0..profile.count {
        let offset = Vec3::new(
            rng.gen_range(-CLOUD_SPREAD..CLOUD_SPREAD),
            rng.gen_range(CLOUD_HEIGHT_MIN..CLOUD_HEIGHT_MAX),
            rng.gen_range(-CLOUD_SPREAD..CLOUD_SPREAD),
        );
        let drift = Vec2::new(rng.gen_range(-1.2..1.2), rng.gen_range(-0.8..0.8));
        commands.spawn((
            Mesh3d(variant_handles[i % CLOUD_VARIANTS].clone()),
            MeshMaterial3d(cloud_material.clone()),
            VoxelCloud { offset, drift },
            Transform::from_translation(offset),
        ));
    }

    commands.init_resource::<CloudTick>();
}

fn build_cloud_mesh(
    seed: u32,
    width: u32,
    height: u32,
    depth: u32,
    top_faces_only: bool,
) -> Mesh {
    let mut rng = SmallRng::seed_from_u64(seed as u64 * 0x9E37_79B9);
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let rgba = [0.97_f32, 0.98, 1.0, 0.92];

    for lx in 0..width {
        for ly in 0..height {
            for lz in 0..depth {
                let nx = (lx as f32 / (width.saturating_sub(1).max(1) as f32)) * 2.0 - 1.0;
                let ny = (ly as f32 / (height.saturating_sub(1).max(1) as f32)) * 2.0 - 1.0;
                let nz = (lz as f32 / (depth.saturating_sub(1).max(1) as f32)) * 2.0 - 1.0;
                let dist = nx * nx + ny * ny * 1.8 + nz * nz;
                if dist > 1.0 {
                    continue;
                }
                if rng.gen::<f32>() > 0.82 {
                    continue;
                }
                push_cloud_cube(
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                    lx as f32,
                    ly as f32,
                    lz as f32,
                    rgba,
                    top_faces_only,
                );
            }
        }
    }

    let center = Vec3::new(width as f32 * 0.5, height as f32 * 0.5, depth as f32 * 0.5);
    for pos in &mut positions {
        pos[0] -= center.x;
        pos[1] -= center.y;
        pos[2] -= center.z;
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn push_cloud_cube(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    x: f32,
    y: f32,
    z: f32,
    rgba: [f32; 4],
    top_faces_only: bool,
) {
    let all_faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [0.0, 1.0, 0.0],
            [
                [x, y + 1.0, z],
                [x + 1.0, y + 1.0, z],
                [x + 1.0, y + 1.0, z + 1.0],
                [x, y + 1.0, z + 1.0],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [x, y, z + 1.0],
                [x + 1.0, y, z + 1.0],
                [x + 1.0, y, z],
                [x, y, z],
            ],
        ),
        (
            [1.0, 0.0, 0.0],
            [
                [x + 1.0, y, z],
                [x + 1.0, y, z + 1.0],
                [x + 1.0, y + 1.0, z + 1.0],
                [x + 1.0, y + 1.0, z],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [x, y, z + 1.0],
                [x, y, z],
                [x, y + 1.0, z],
                [x, y + 1.0, z + 1.0],
            ],
        ),
        (
            [0.0, 0.0, 1.0],
            [
                [x + 1.0, y, z + 1.0],
                [x, y, z + 1.0],
                [x, y + 1.0, z + 1.0],
                [x + 1.0, y + 1.0, z + 1.0],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [x, y, z],
                [x + 1.0, y, z],
                [x + 1.0, y + 1.0, z],
                [x, y + 1.0, z],
            ],
        ),
    ];

    let face_count = if top_faces_only { 1 } else { 6 };
    for (normal, verts) in all_faces.iter().take(face_count) {
        let base = positions.len() as u32;
        for v in *verts {
            positions.push(v);
            normals.push(*normal);
            colors.push(rgba);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

pub fn update_clouds(
    time: Res<Time>,
    player: Res<PlayerState>,
    mut tick: ResMut<CloudTick>,
    mut clouds: Query<(&mut VoxelCloud, &mut Transform)>,
) {
    tick.0 = tick.0.wrapping_add(1);
    if tick.0 % CLOUD_UPDATE_INTERVAL != 0 {
        return;
    }

    let dt = time.delta_secs() * CLOUD_UPDATE_INTERVAL as f32;
    let anchor = player.position;

    for (mut cloud, mut transform) in clouds.iter_mut() {
        cloud.offset.x += cloud.drift.x * dt;
        cloud.offset.z += cloud.drift.y * dt;

        if cloud.offset.x.abs() > CLOUD_SPREAD {
            cloud.offset.x = -cloud.offset.x.signum() * CLOUD_SPREAD;
        }
        if cloud.offset.z.abs() > CLOUD_SPREAD {
            cloud.offset.z = -cloud.offset.z.signum() * CLOUD_SPREAD;
        }

        transform.translation = Vec3::new(
            anchor.x + cloud.offset.x,
            cloud.offset.y,
            anchor.z + cloud.offset.z,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_mesh_has_geometry() {
        let mesh = build_cloud_mesh(3, 7, 4, 7, true);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("cloud positions");
        match positions {
            bevy::render::mesh::VertexAttributeValues::Float32x3(verts) => {
                assert!(verts.len() > 24, "cloud should have multiple voxel faces");
            }
            _ => panic!("unexpected vertex format"),
        }
    }

    #[test]
    fn wasm_cloud_profile_is_lighter() {
        let mesh = build_cloud_mesh(1, 5, 3, 5, true);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("cloud positions");
        match positions {
            bevy::render::mesh::VertexAttributeValues::Float32x3(verts) => {
                let full_mesh = build_cloud_mesh(1, 7, 4, 7, false);
                let full_verts = match full_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
                    bevy::render::mesh::VertexAttributeValues::Float32x3(v) => v.len(),
                    _ => 0,
                };
                assert!(verts.len() < full_verts);
            }
            _ => panic!("unexpected vertex format"),
        }
    }
}
