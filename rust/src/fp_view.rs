use crate::menu::{is_playing, GameUiState};
use crate::player::{PlayerCamera, PlayerState};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

const PICKAXE_REST: Vec3 = Vec3::new(0.42, -0.28, -0.55);
const PICKAXE_REST_ROT: Vec3 = Vec3::new(-0.5, 0.85, 0.15);
const PICKAXE_SCALE: Vec3 = Vec3::splat(0.14);

#[derive(Component)]
pub struct FirstPersonPickaxe;

#[derive(Component)]
pub struct FirstPersonArm;

pub fn setup_fp_view(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<Entity, With<PlayerCamera>>,
) {
    let Ok(camera) = camera.get_single() else {
        return;
    };
    let pickaxe = meshes.add(build_pickaxe_mesh());
    let arm_mesh = meshes.add(build_arm_mesh());
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.72, 0.58),
        perceptual_roughness: 1.0,
        ..default()
    });
    let sleeve = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.42, 0.62),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.38, 0.18),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.entity(camera).with_children(|parent| {
        parent.spawn((
            Mesh3d(arm_mesh.clone()),
            MeshMaterial3d(sleeve.clone()),
            Transform::from_xyz(-0.34, -0.38, -0.42)
                .with_rotation(Quat::from_euler(EulerRot::ZYX, 0.15, 0.35, 0.08))
                .with_scale(Vec3::new(0.11, 0.11, 0.11)),
            FirstPersonArm,
        ));
        parent.spawn((
            Mesh3d(arm_mesh),
            MeshMaterial3d(skin),
            Transform::from_xyz(-0.28, -0.34, -0.38)
                .with_rotation(Quat::from_euler(EulerRot::ZYX, 0.1, 0.2, 0.05))
                .with_scale(Vec3::new(0.09, 0.09, 0.09)),
            FirstPersonArm,
        ));
        parent.spawn((
            Mesh3d(pickaxe),
            MeshMaterial3d(wood),
            Transform::from_translation(PICKAXE_REST)
                .with_rotation(Quat::from_euler(
                    EulerRot::ZYX,
                    PICKAXE_REST_ROT.x,
                    PICKAXE_REST_ROT.y,
                    PICKAXE_REST_ROT.z,
                ))
                .with_scale(PICKAXE_SCALE),
            FirstPersonPickaxe,
        ));
    });
}

fn build_arm_mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    add_box(
        &mut positions,
        &mut normals,
        &mut indices,
        Vec3::new(-0.35, -1.2, -0.35),
        Vec3::new(0.35, 1.0, 0.35),
    );
    add_box(
        &mut positions,
        &mut normals,
        &mut indices,
        Vec3::new(-0.5, 0.8, -0.5),
        Vec3::new(0.5, 1.4, 0.5),
    );

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn build_pickaxe_mesh() -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    add_box(
        &mut positions,
        &mut normals,
        &mut indices,
        Vec3::new(-0.3, -2.5, -0.3),
        Vec3::new(0.3, 2.0, 0.3),
    );
    add_box(
        &mut positions,
        &mut normals,
        &mut indices,
        Vec3::new(-1.4, 1.6, -0.35),
        Vec3::new(1.4, 2.2, 0.35),
    );
    add_box(
        &mut positions,
        &mut normals,
        &mut indices,
        Vec3::new(-0.25, 1.0, -0.25),
        Vec3::new(0.25, 1.8, 0.25),
    );

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn add_box(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    min: Vec3,
    max: Vec3,
) {
    let base = positions.len() as u32;
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    for c in corners {
        positions.push([c.x, c.y, c.z]);
        normals.push([0.0, 1.0, 0.0]);
    }
    let faces = [
        [0, 1, 2, 3],
        [5, 4, 7, 6],
        [4, 0, 3, 7],
        [1, 5, 6, 2],
        [3, 2, 6, 7],
        [4, 5, 1, 0],
    ];
    for f in faces {
        indices.extend_from_slice(&[
            base + f[0],
            base + f[1],
            base + f[2],
            base + f[0],
            base + f[2],
            base + f[3],
        ]);
    }
}

pub fn update_fp_view(
    time: Res<Time>,
    ui: Res<GameUiState>,
    mut player: ResMut<PlayerState>,
    mut parts: Query<&mut Visibility, Or<(With<FirstPersonPickaxe>, With<FirstPersonArm>)>>,
    mut pickaxe: Query<&mut Transform, With<FirstPersonPickaxe>>,
) {
    let show = is_playing(&ui);
    for mut vis in parts.iter_mut() {
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        return;
    }

    let dt = time.delta_secs();
    player.pickaxe_swing = (player.pickaxe_swing - dt * 3.2).max(0.0);
    let swing = player.pickaxe_swing;
    let arc = if swing > 0.0 {
        (swing * std::f32::consts::PI).sin()
    } else {
        0.0
    };

    if let Ok(mut transform) = pickaxe.get_single_mut() {
        let swing_rot = Quat::from_euler(EulerRot::ZYX, -arc * 1.35, arc * 0.25, arc * 0.4);
        let rest_rot = Quat::from_euler(
            EulerRot::ZYX,
            PICKAXE_REST_ROT.x,
            PICKAXE_REST_ROT.y,
            PICKAXE_REST_ROT.z,
        );
        let lunge = Vec3::new(arc * 0.06, -arc * 0.04, -arc * 0.14);
        *transform = Transform::from_translation(PICKAXE_REST + lunge)
            .with_rotation(rest_rot * swing_rot)
            .with_scale(PICKAXE_SCALE);
    }
}
