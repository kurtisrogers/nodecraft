use crate::config::DAY_LENGTH_SECS;
use crate::player::PlayerState;
use crate::weather::DayNight;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const SKY_DISTANCE: f32 = 320.0;
const SUN_RADIUS: f32 = 14.0;
const STAR_COUNT: usize = 140;

#[derive(Component)]
pub struct SunDisc;

#[derive(Component)]
pub struct StarField;

pub fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sun_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.92, 0.45),
        emissive: LinearRgba::new(2.5, 2.0, 0.6, 1.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(SUN_RADIUS))),
        MeshMaterial3d(sun_material),
        SunDisc,
        Visibility::Hidden,
    ));

    let star_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.95),
        emissive: LinearRgba::new(1.2, 1.2, 1.3, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(build_star_mesh())),
        MeshMaterial3d(star_material),
        StarField,
        Visibility::Hidden,
    ));
}

fn build_star_mesh() -> Mesh {
    let mut rng = SmallRng::seed_from_u64(0x5a7e_c001);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(STAR_COUNT * 4);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(STAR_COUNT * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(STAR_COUNT * 6);

    for _ in 0..STAR_COUNT {
        let theta = rng.gen_range(0.0..std::f32::consts::TAU);
        let u = rng.gen_range(-1.0..1.0);
        let radius_xy = (1.0_f32 - u * u).sqrt();
        let dir = Vec3::new(radius_xy * theta.cos(), u, radius_xy * theta.sin()).normalize();
        let center = dir * SKY_DISTANCE;
        let tangent = if dir.y.abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y.cross(dir).normalize()
        };
        let bitangent = dir.cross(tangent).normalize();
        let size = rng.gen_range(0.35..0.9);
        let base = positions.len() as u32;
        let normal = [dir.x, dir.y, dir.z];
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let offset = tangent * sx * size + bitangent * sy * size;
            let p = center + offset;
            positions.push([p.x, p.y, p.z]);
            normals.push(normal);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn update_sky(
    day: Res<DayNight>,
    player: Res<PlayerState>,
    light: Query<&Transform, With<DirectionalLight>>,
    mut sun: Query<(&mut Transform, &mut Visibility), (With<SunDisc>, Without<StarField>)>,
    mut stars: Query<(&mut Transform, &mut Visibility), (With<StarField>, Without<SunDisc>)>,
) {
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    let sun_height = (cycle * std::f32::consts::TAU).sin();
    let anchor = player.position;

    let light_forward = light
        .get_single()
        .map(|t| t.forward().as_vec3())
        .unwrap_or(Vec3::NEG_Y);

    if let Ok((mut transform, mut visibility)) = sun.get_single_mut() {
        transform.translation = anchor - light_forward * SKY_DISTANCE;
        transform.look_at(anchor, Vec3::Y);
        let alpha = ((sun_height - 0.05) / 0.25).clamp(0.0, 1.0);
        *visibility = if alpha > 0.02 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok((mut transform, mut visibility)) = stars.get_single_mut() {
        transform.translation = anchor;
        let night = ((-sun_height - 0.05) / 0.25).clamp(0.0, 1.0);
        *visibility = if night > 0.02 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
