use crate::config::EYE_HEIGHT;
use crate::player::{PlayerCamera, PlayerState};
use crate::weather::{celestial_state, DayNight, MoonLight, SunLight};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const SKY_DISTANCE: f32 = 280.0;
const SUN_RADIUS: f32 = 10.0;
const MOON_RADIUS: f32 = 8.0;
const STAR_COUNT: usize = 120;
const SKY_UPDATE_INTERVAL: u32 = 2;

#[derive(Component)]
pub struct SunDisc;

#[derive(Component)]
pub struct MoonDisc;

#[derive(Component)]
pub struct StarField;

#[derive(Resource, Default)]
pub(crate) struct SkyTick(u32);

pub fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sun_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.92, 0.45),
        emissive: LinearRgba::new(2.0, 1.6, 0.4, 1.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(build_sky_disc_mesh(SUN_RADIUS))),
        MeshMaterial3d(sun_material),
        SunDisc,
        Visibility::Hidden,
    ));

    let moon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.9, 0.96),
        emissive: LinearRgba::new(0.9, 0.95, 1.1, 1.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(build_sky_disc_mesh(MOON_RADIUS))),
        MeshMaterial3d(moon_material),
        MoonDisc,
        Visibility::Hidden,
    ));

    if cfg!(not(target_arch = "wasm32")) {
        let star_material = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.9),
            emissive: LinearRgba::new(0.8, 0.8, 0.9, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(build_star_mesh())),
            MeshMaterial3d(star_material),
            StarField,
            Visibility::Hidden,
        ));
    }

    commands.init_resource::<SkyTick>();
}

/// Two crossed quads — cheap "round" disc that billboards well.
fn build_sky_disc_mesh(radius: f32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(8);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(8);
    let mut indices: Vec<u32> = Vec::with_capacity(12);

    let quads = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ];

    for (normal, up) in quads {
        let right = Vec3::new(normal[0], normal[1], normal[2]).cross(Vec3::new(up[0], up[1], up[2]));
        let verts = [
            right * radius + Vec3::new(up[0], up[1], up[2]) * radius,
            -right * radius + Vec3::new(up[0], up[1], up[2]) * radius,
            -right * radius - Vec3::new(up[0], up[1], up[2]) * radius,
            right * radius - Vec3::new(up[0], up[1], up[2]) * radius,
        ];
        let base = positions.len() as u32;
        for v in verts {
            positions.push([v.x, v.y, v.z]);
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
        let size = rng.gen_range(0.12..0.28);
        let base = positions.len() as u32;
        let normal = [-dir.x, -dir.y, -dir.z];
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
    camera: Query<&Transform, With<PlayerCamera>>,
    sun_light: Query<&Transform, (With<SunLight>, Without<MoonLight>)>,
    moon_light: Query<&Transform, (With<MoonLight>, Without<SunLight>)>,
    mut tick: ResMut<SkyTick>,
    mut sun: Query<
        (&mut Transform, &mut Visibility),
        (With<SunDisc>, Without<MoonDisc>, Without<StarField>),
    >,
    mut moon: Query<
        (&mut Transform, &mut Visibility),
        (With<MoonDisc>, Without<SunDisc>, Without<StarField>),
    >,
    mut stars: Query<
        (&mut Transform, &mut Visibility),
        (With<StarField>, Without<SunDisc>, Without<MoonDisc>),
    >,
) {
    tick.0 = tick.0.wrapping_add(1);
    if tick.0 % SKY_UPDATE_INTERVAL != 0 {
        return;
    }

    let state = celestial_state(&day);
    let anchor = player.position;
    let camera_pos = camera
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(anchor + Vec3::Y * EYE_HEIGHT);

    let sun_forward = sun_light
        .get_single()
        .map(|t| t.forward().as_vec3())
        .unwrap_or(Vec3::NEG_Y);
    let moon_forward = moon_light
        .get_single()
        .map(|t| t.forward().as_vec3())
        .unwrap_or(Vec3::Y);

    if let Ok((mut transform, mut visibility)) = sun.get_single_mut() {
        transform.translation = anchor - sun_forward * SKY_DISTANCE;
        billboard_toward_camera(&mut transform, camera_pos);
        *visibility = if state.sun > 0.08 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok((mut transform, mut visibility)) = moon.get_single_mut() {
        transform.translation = anchor - moon_forward * SKY_DISTANCE;
        billboard_toward_camera(&mut transform, camera_pos);
        *visibility = if state.sun < -0.08 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok((mut transform, mut visibility)) = stars.get_single_mut() {
        transform.translation = anchor;
        *visibility = if state.sun < -0.08 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn billboard_toward_camera(transform: &mut Transform, camera_pos: Vec3) {
    let to_camera = camera_pos - transform.translation;
    if to_camera.length_squared() > 1e-6 {
        transform.look_at(camera_pos, Vec3::Y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_disc_is_lightweight() {
        let mesh = build_sky_disc_mesh(10.0);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("disc positions");
        match positions {
            bevy::render::mesh::VertexAttributeValues::Float32x3(verts) => {
                assert_eq!(verts.len(), 8, "disc should be two crossed quads");
            }
            _ => panic!("unexpected vertex format"),
        }
    }
}
