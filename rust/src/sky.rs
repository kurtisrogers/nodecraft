use crate::config::EYE_HEIGHT;
use crate::player::PlayerState;
use crate::weather::{celestial_state, DayNight};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const SKY_DISTANCE: f32 = 280.0;
const STAR_DISTANCE: f32 = 95.0;
const SUN_RADIUS: f32 = 11.0;
const MOON_RADIUS: f32 = 9.0;
const SKY_UPDATE_INTERVAL: u32 = 2;

#[cfg(target_arch = "wasm32")]
const STAR_SPRITE_COUNT: usize = 28;
#[cfg(not(target_arch = "wasm32"))]
const STAR_SPRITE_COUNT: usize = 56;

#[derive(Component)]
pub struct SunDisc;

#[derive(Component)]
pub struct SunGlow;

#[derive(Component)]
pub struct MoonDisc;

#[derive(Component)]
pub struct MoonGlow;

#[derive(Component)]
pub(crate) struct StarSprite {
    direction: Vec3,
    twinkle_phase: f32,
    twinkle_speed: f32,
    base_scale: f32,
    material: Handle<StandardMaterial>,
}

#[derive(Resource)]
pub(crate) struct SkyMaterials {
    sun_disc: Handle<StandardMaterial>,
    sun_glow: Handle<StandardMaterial>,
    moon_disc: Handle<StandardMaterial>,
    moon_glow: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub(crate) struct SkyTick(u32);

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 >= edge1 {
        return if x >= edge0 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let a = a.to_srgba();
    let b = b.to_srgba();
    Color::srgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

pub fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let disc_mesh = meshes.add(build_sky_disc_mesh(1.0));
    let glow_mesh = meshes.add(build_sky_disc_mesh(1.0));
    let star_mesh = meshes.add(build_star_quad_mesh());

    let sun_disc = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.94, 0.45),
        emissive: LinearRgba::new(2.4, 1.9, 0.45, 1.0),
        unlit: true,
        ..default()
    });
    let sun_glow = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.82, 0.28, 0.55),
        emissive: LinearRgba::new(1.6, 1.0, 0.2, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let moon_disc = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.92, 0.98),
        emissive: LinearRgba::new(1.1, 1.15, 1.35, 1.0),
        unlit: true,
        ..default()
    });
    let moon_glow = materials.add(StandardMaterial {
        base_color: Color::srgba(0.72, 0.82, 1.0, 0.45),
        emissive: LinearRgba::new(0.55, 0.7, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.insert_resource(SkyMaterials {
        sun_disc: sun_disc.clone(),
        sun_glow: sun_glow.clone(),
        moon_disc: moon_disc.clone(),
        moon_glow: moon_glow.clone(),
    });

    commands.spawn((
        Mesh3d(disc_mesh.clone()),
        MeshMaterial3d(sun_disc),
        Transform::from_scale(Vec3::splat(SUN_RADIUS)),
        SunDisc,
        Visibility::Hidden,
    ));
    commands.spawn((
        Mesh3d(glow_mesh.clone()),
        MeshMaterial3d(sun_glow),
        Transform::from_scale(Vec3::splat(SUN_RADIUS * 2.4)),
        SunGlow,
        Visibility::Hidden,
    ));

    commands.spawn((
        Mesh3d(disc_mesh.clone()),
        MeshMaterial3d(moon_disc),
        Transform::from_scale(Vec3::splat(MOON_RADIUS)),
        MoonDisc,
        Visibility::Hidden,
    ));
    commands.spawn((
        Mesh3d(glow_mesh),
        MeshMaterial3d(moon_glow),
        Transform::from_scale(Vec3::splat(MOON_RADIUS * 2.2)),
        MoonGlow,
        Visibility::Hidden,
    ));

    let mut rng = SmallRng::seed_from_u64(0x5a7e_c001);
    for _ in 0..STAR_SPRITE_COUNT {
        let direction = random_sky_direction(&mut rng);
        let base_scale = rng.gen_range(0.35..0.72);
        let twinkle_phase = rng.gen_range(0.0..std::f32::consts::TAU);
        let twinkle_speed = rng.gen_range(1.6..4.8);
        let tint = rng.gen_range(0.92..1.0);
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(tint, tint, 1.0, 0.98),
            emissive: LinearRgba::new(1.4, 1.45, 1.8, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            StarSprite {
                direction,
                twinkle_phase,
                twinkle_speed,
                base_scale,
                material: material.clone(),
            },
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_scale(Vec3::splat(base_scale)),
            Visibility::Hidden,
        ));
    }

    commands.init_resource::<SkyTick>();
}

fn random_sky_direction(rng: &mut SmallRng) -> Vec3 {
    loop {
        let theta = rng.gen_range(0.0..std::f32::consts::TAU);
        let u = rng.gen_range(-1.0..1.0);
        let radius_xy = (1.0_f32 - u * u).sqrt();
        let dir = Vec3::new(radius_xy * theta.cos(), u, radius_xy * theta.sin());
        if dir.y > 0.12 {
            return dir.normalize();
        }
    }
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

fn build_star_quad_mesh() -> Mesh {
    let half = 0.5;
    let positions = vec![
        [-half, -half, 0.0],
        [half, -half, 0.0],
        [half, half, 0.0],
        [-half, half, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let indices = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn update_sky(
    time: Res<Time>,
    day: Res<DayNight>,
    player: Res<PlayerState>,
    mut tick: ResMut<SkyTick>,
    sky_materials: Res<SkyMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sun: Query<
        (&mut Transform, &mut Visibility),
        (With<SunDisc>, Without<SunGlow>, Without<MoonDisc>, Without<MoonGlow>),
    >,
    mut sun_glow: Query<
        (&mut Transform, &mut Visibility),
        (With<SunGlow>, Without<SunDisc>, Without<MoonDisc>, Without<MoonGlow>),
    >,
    mut moon: Query<
        (&mut Transform, &mut Visibility),
        (With<MoonDisc>, Without<SunDisc>, Without<SunGlow>, Without<MoonGlow>),
    >,
    mut moon_glow: Query<
        (&mut Transform, &mut Visibility),
        (With<MoonGlow>, Without<SunDisc>, Without<SunGlow>, Without<MoonDisc>),
    >,
    mut stars: Query<
        (&StarSprite, &mut Transform, &mut Visibility),
        (
            Without<SunDisc>,
            Without<SunGlow>,
            Without<MoonDisc>,
            Without<MoonGlow>,
        ),
    >,
) {
    tick.0 = tick.0.wrapping_add(1);
    let animate_stars = true;
    let update_positions = tick.0 % SKY_UPDATE_INTERVAL == 0;

    let state = celestial_state(&day);
    let anchor = player.position;
    let camera_pos = anchor + Vec3::Y * EYE_HEIGHT;
    let elapsed = time.elapsed_secs();

    let sun_strength = smoothstep(-0.08, 0.18, state.sun_elevation);
    let moon_strength = smoothstep(-0.08, 0.16, state.moon_elevation);
    let night = smoothstep(0.12, -0.18, state.sun_elevation);

    if update_positions {
        let sun_pos = anchor - state.sun_forward * SKY_DISTANCE;
        if let Ok((mut transform, mut visibility)) = sun.get_single_mut() {
            transform.translation = sun_pos;
            billboard_toward_camera(&mut transform, camera_pos);
            *visibility = if sun_strength > 0.02 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if let Ok((mut transform, mut visibility)) = sun_glow.get_single_mut() {
            transform.translation = sun_pos;
            billboard_toward_camera(&mut transform, camera_pos);
            *visibility = if sun_strength > 0.02 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        let moon_pos = anchor - state.moon_forward * SKY_DISTANCE;
        if let Ok((mut transform, mut visibility)) = moon.get_single_mut() {
            transform.translation = moon_pos;
            billboard_toward_camera(&mut transform, camera_pos);
            *visibility = if moon_strength > 0.02 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if let Ok((mut transform, mut visibility)) = moon_glow.get_single_mut() {
            transform.translation = moon_pos;
            billboard_toward_camera(&mut transform, camera_pos);
            *visibility = if moon_strength > 0.02 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    if update_positions {
        if let Some(mat) = materials.get_mut(&sky_materials.sun_disc) {
            let zenith = state.sun_elevation.clamp(0.0, 1.0);
            let sunset = (1.0 - (state.sun_elevation / 0.34).clamp(0.0, 1.0)) * state.daylight;
            let day_color = Color::srgb(1.0, 0.94, 0.45);
            let sunset_color = Color::srgb(1.0, 0.58, 0.18);
            mat.base_color = lerp_color(day_color, sunset_color, sunset);
            let emissive = 1.8 + zenith * 4.2 + sun_strength * 2.0;
            mat.emissive = LinearRgba::new(
                emissive,
                emissive * (0.82 - sunset * 0.18),
                emissive * (0.28 - sunset * 0.08),
                1.0,
            );
        }
        if let Some(mat) = materials.get_mut(&sky_materials.sun_glow) {
            let sunset = (1.0 - (state.sun_elevation / 0.34).clamp(0.0, 1.0)) * state.daylight;
            let glow = (0.8 + state.sun_elevation.max(0.0) * 2.4 + sunset * 1.6) * sun_strength;
            mat.emissive = LinearRgba::new(glow * 1.1, glow * 0.72, glow * 0.18, 1.0);
            mat.base_color = Color::srgba(1.0, 0.78, 0.28, 0.25 + sun_strength * 0.35);
        }
        if let Some(mat) = materials.get_mut(&sky_materials.moon_disc) {
            let glow = 0.9 + state.moon_elevation.max(0.0) * 2.2 + moon_strength;
            mat.emissive = LinearRgba::new(glow * 0.82, glow * 0.88, glow * 1.15, 1.0);
        }
        if let Some(mat) = materials.get_mut(&sky_materials.moon_glow) {
            let glow = (0.45 + state.moon_elevation.max(0.0) * 1.4) * moon_strength;
            mat.emissive = LinearRgba::new(glow * 0.55, glow * 0.72, glow * 1.05, 1.0);
            mat.base_color = Color::srgba(0.72, 0.82, 1.0, 0.18 + moon_strength * 0.28);
        }
    }

    if animate_stars {
        for (sprite, mut transform, mut visibility) in stars.iter_mut() {
            if night < 0.02 {
                *visibility = Visibility::Hidden;
                continue;
            }
            *visibility = Visibility::Visible;
            transform.translation = anchor + sprite.direction * STAR_DISTANCE;
            billboard_toward_camera(&mut transform, camera_pos);

            let t = elapsed * sprite.twinkle_speed + sprite.twinkle_phase;
            let pulse = 0.5 + 0.5 * t.sin();
            let sparkle = pulse.powf(2.6);
            let flash = (t * 0.41 + sprite.twinkle_phase * 1.7).sin().max(0.0).powi(10);
            let brightness = (0.55 + sparkle * 0.85 + flash * 1.1) * night;

            if let Some(mat) = materials.get_mut(&sprite.material) {
                mat.emissive = LinearRgba::new(
                    brightness,
                    brightness,
                    brightness * 1.08,
                    1.0,
                );
                mat.base_color = Color::srgba(brightness, brightness, brightness * 1.05, 0.85);
            }
            let scale = sprite.base_scale * (0.75 + brightness * 0.65);
            transform.scale = Vec3::new(scale, scale, scale);
        }
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

    #[test]
    fn star_sprites_spawn_on_all_targets() {
        assert!(STAR_SPRITE_COUNT >= 20);
    }
}
