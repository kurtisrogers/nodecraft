use crate::mobs::{MobEntity, MobType};
use bevy::prelude::*;

#[derive(Resource)]
pub struct MobModelAssets {
    pub meshes: std::collections::HashMap<MobType, Handle<Mesh>>,
}

pub fn setup_mob_models(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut map = std::collections::HashMap::new();
    for kind in [
        MobType::Pig,
        MobType::Cow,
        MobType::Sheep,
        MobType::Chicken,
        MobType::Zombie,
    ] {
        map.insert(kind, meshes.add(combined_mob_mesh(kind)));
    }
    commands.insert_resource(MobModelAssets { meshes: map });
}

fn combined_mob_mesh(kind: MobType) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    match kind {
        MobType::Pig => {
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.45, 0.35, -0.7), Vec3::new(0.45, 0.95, 0.7));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.35, 0.55, -1.05), Vec3::new(0.35, 0.85, -0.65));
            for &(x, z) in &[(-0.28, -0.45), (0.28, -0.45), (-0.28, 0.45), (0.28, 0.45)] {
                stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(x - 0.1, 0.0, z - 0.1), Vec3::new(x + 0.1, 0.35, z + 0.1));
            }
        }
        MobType::Cow => {
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.55, 0.45, -0.85), Vec3::new(0.55, 1.25, 0.85));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.35, 0.75, -1.2), Vec3::new(0.35, 1.15, -0.75));
            for &(x, z) in &[(-0.35, -0.55), (0.35, -0.55), (-0.35, 0.55), (0.35, 0.55)] {
                stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(x - 0.12, 0.0, z - 0.12), Vec3::new(x + 0.12, 0.45, z + 0.12));
            }
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.42, 1.15, -1.05), Vec3::new(-0.22, 1.45, -0.85));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(0.22, 1.15, -1.05), Vec3::new(0.42, 1.45, -0.85));
        }
        MobType::Sheep => {
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.55, 0.35, -0.75), Vec3::new(0.55, 1.05, 0.75));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.3, 0.55, -1.0), Vec3::new(0.3, 0.85, -0.65));
            for &(x, z) in &[(-0.28, -0.42), (0.28, -0.42), (-0.28, 0.42), (0.28, 0.42)] {
                stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(x - 0.08, 0.0, z - 0.08), Vec3::new(x + 0.08, 0.38, z + 0.08));
            }
        }
        MobType::Chicken => {
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.18, 0.2, -0.22), Vec3::new(0.18, 0.45, 0.22));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.12, 0.38, -0.42), Vec3::new(0.12, 0.55, -0.18));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.04, 0.42, -0.55), Vec3::new(0.04, 0.48, -0.42));
            for &(x, z) in &[(-0.1, 0.05), (0.1, 0.05)] {
                stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(x - 0.03, 0.0, z - 0.03), Vec3::new(x + 0.03, 0.18, z + 0.03));
            }
        }
        MobType::Zombie => {
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.22, 0.75, -0.15), Vec3::new(0.22, 1.35, 0.15));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.2, 1.35, -0.2), Vec3::new(0.2, 1.75, 0.2));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(-0.55, 0.95, -0.12), Vec3::new(-0.25, 1.55, 0.12));
            stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(0.25, 0.95, -0.12), Vec3::new(0.55, 1.55, 0.12));
            for &(x, z) in &[(-0.12, -0.08), (0.12, -0.08)] {
                stamp_part(&mut positions, &mut normals, &mut indices, Vec3::new(x - 0.1, 0.0, z - 0.1), Vec3::new(x + 0.1, 0.75, z + 0.1));
            }
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn stamp_part(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    min: Vec3,
    max: Vec3,
) {
    crate::proc_mesh::add_box(positions, normals, indices, min, max);
}

pub fn mob_material(kind: MobType, materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    let (base, accent) = match kind {
        MobType::Pig => (Color::srgb(1.0, 0.71, 0.76), Color::srgb(0.92, 0.45, 0.52)),
        MobType::Cow => (Color::srgb(0.42, 0.32, 0.24), Color::srgb(0.92, 0.92, 0.9)),
        MobType::Sheep => (Color::srgb(0.95, 0.95, 0.96), Color::srgb(0.82, 0.82, 0.84)),
        MobType::Chicken => (Color::srgb(0.98, 0.98, 0.98), Color::srgb(0.95, 0.35, 0.15)),
        MobType::Zombie => (Color::srgb(0.29, 0.49, 0.31), Color::srgb(0.45, 0.32, 0.28)),
    };
    materials.add(StandardMaterial {
        base_color: base.mix(&accent, 0.15),
        perceptual_roughness: 1.0,
        ..default()
    })
}

pub fn spawn_mob_model(
    commands: &mut Commands,
    assets: &MobModelAssets,
    materials: &mut Assets<StandardMaterial>,
    kind: MobType,
    position: Vec3,
    health: i32,
) -> Entity {
    let mesh = assets.meshes.get(&kind).cloned().expect("mob mesh");
    let mat = mob_material(kind, materials);
    let aabb = kind.aabb();
    let y = position.y + aabb.height * 0.5;

    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(position.x, y, position.z)),
            MobEntity {
                kind,
                health,
                velocity: Vec3::ZERO,
                wander_timer: 2.0,
                wander_dir: Vec3::Z,
                alive: true,
            },
        ))
        .id()
}
