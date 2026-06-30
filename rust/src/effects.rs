use crate::meshing::VoxelWorldResource;
use crate::player::{PlayerCamera, PlayerState};
use crate::proc_mesh::box_mesh;
use crate::weather::DayNight;
use bevy::prelude::*;
use rand::Rng;

const SMOKE_LIFETIME: f32 = 3.2;
const MAX_SMOKE_NEAR_PLAYER: usize = 48;

#[derive(Component)]
pub struct SmokePuff {
    pub life: f32,
    pub rise: f32,
    pub drift: Vec2,
    pub volcano_key: (i32, i32),
}

#[derive(Component)]
pub struct LanternPost;

#[derive(Resource, Default)]
pub struct WorldEffectsState {
    pub lanterns_spawned: std::collections::HashSet<(i32, i32)>,
}

#[derive(Resource)]
pub struct EffectAssets {
    pub smoke_mesh: Handle<Mesh>,
    pub smoke_mat: Handle<StandardMaterial>,
    pub lantern_mesh: Handle<Mesh>,
    pub lantern_mat: Handle<StandardMaterial>,
    pub pole_mat: Handle<StandardMaterial>,
}

pub fn setup_effects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let smoke_mesh = meshes.add(Mesh::from(Rectangle::new(1.4, 1.4)));
    let smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.55, 0.58, 0.45),
        emissive: LinearRgba::new(0.2, 0.2, 0.22, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let lantern_mesh = meshes.add(box_mesh(
        Vec3::new(-0.12, 0.0, -0.12),
        Vec3::new(0.12, 0.35, 0.12),
    ));
    let lantern_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.72, 0.28),
        emissive: LinearRgba::new(2.2, 1.4, 0.35, 1.0),
        perceptual_roughness: 0.6,
        ..default()
    });
    let pole_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.24, 0.14),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.insert_resource(EffectAssets {
        smoke_mesh,
        smoke_mat,
        lantern_mesh,
        lantern_mat,
        pole_mat,
    });
    commands.init_resource::<WorldEffectsState>();
}

pub fn sync_volcano_smoke(
    player: Res<PlayerState>,
    world: Res<VoxelWorldResource>,
    assets: Res<EffectAssets>,
    mut commands: Commands,
    existing: Query<&SmokePuff>,
) {
    let px = player.position.x as i32;
    let pz = player.position.z as i32;
    let active_volcanoes: Vec<(i32, i32)> = world
        .inner
        .volcano_centers
        .iter()
        .copied()
        .filter(|&(cx, cz)| {
            let dx = cx - px;
            let dz = cz - pz;
            (dx * dx + dz * dz) <= 180 * 180
        })
        .collect();

    let smoke_count = existing.iter().count();
    if smoke_count >= MAX_SMOKE_NEAR_PLAYER || active_volcanoes.is_empty() {
        return;
    }

    let mut rng = rand::thread_rng();
    if rng.gen::<f32>() > 0.22 {
        return;
    }

    let (cx, cz) = active_volcanoes[rng.gen_range(0..active_volcanoes.len())];
    let y = world.inner.noise.terrain_height(cx, cz) as f32 + rng.gen_range(2.0..5.0);
    let ox = rng.gen_range(-4.0..4.0);
    let oz = rng.gen_range(-4.0..4.0);

    commands.spawn((
        Mesh3d(assets.smoke_mesh.clone()),
        MeshMaterial3d(assets.smoke_mat.clone()),
        SmokePuff {
            life: SMOKE_LIFETIME,
            rise: rng.gen_range(1.2..2.4),
            drift: Vec2::new(rng.gen_range(-0.4..0.4), rng.gen_range(-0.4..0.4)),
            volcano_key: (cx, cz),
        },
        Transform::from_xyz(cx as f32 + ox, y, cz as f32 + oz).with_scale(Vec3::splat(0.4)),
    ));
}

pub fn update_smoke(
    time: Res<Time>,
    player: Res<PlayerState>,
    camera: Query<&GlobalTransform, With<PlayerCamera>>,
    mut commands: Commands,
    mut puffs: Query<(Entity, &mut SmokePuff, &mut Transform)>,
) {
    let cam_pos = camera
        .get_single()
        .map(|t| t.translation())
        .unwrap_or(Vec3::ZERO);
    let px = player.position.x as i32;
    let pz = player.position.z as i32;
    let dt = time.delta_secs();

    for (entity, mut puff, mut transform) in puffs.iter_mut() {
        let dx = puff.volcano_key.0 - px;
        let dz = puff.volcano_key.1 - pz;
        if (dx * dx + dz * dz) > 200 * 200 {
            commands.entity(entity).despawn();
            continue;
        }

        puff.life -= dt;
        if puff.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let t = 1.0 - puff.life / SMOKE_LIFETIME;
        transform.translation.y += puff.rise * dt;
        transform.translation.x += puff.drift.x * dt;
        transform.translation.z += puff.drift.y * dt;
        transform.scale = Vec3::splat(0.35 + t * 1.6);
        transform.look_at(cam_pos, Vec3::Y);
    }
}

pub fn sync_lanterns(
    world: Res<VoxelWorldResource>,
    day: Res<DayNight>,
    assets: Res<EffectAssets>,
    mut state: ResMut<WorldEffectsState>,
    mut commands: Commands,
) {
    const MAX_LANTERNS: usize = 96;
    if state.lanterns_spawned.len() >= MAX_LANTERNS {
        return;
    }

    let night_boost = {
        let cycle = (day.time % crate::config::DAY_LENGTH_SECS) / crate::config::DAY_LENGTH_SECS;
        let sun = (cycle * std::f32::consts::TAU).sin();
        (1.0 - sun.max(0.0)).clamp(0.0, 1.0)
    };
    let intensity = 600.0 + night_boost * 1400.0;

    for &(cx, cz) in &world.inner.settlement_centers {
        if state.lanterns_spawned.len() >= MAX_LANTERNS {
            break;
        }
        let key = (cx.div_euclid(64), cz.div_euclid(64));
        if state.lanterns_spawned.contains(&key) {
            continue;
        }
        let y = world.inner.noise.terrain_height(cx, cz) as f32 + 2.0;
        let offsets = [
            (cx - 9, cz - 8),
            (cx + 7, cz - 10),
            (cx - 2, cz + 7),
            (cx + 12, cz + 3),
        ];
        for (lx, lz) in offsets {
            spawn_lantern(&mut commands, &assets, lx as f32 + 0.5, y, lz as f32 + 0.5, intensity);
        }
        state.lanterns_spawned.insert(key);
    }
}

pub fn scatter_wild_lanterns(
    player: Res<PlayerState>,
    world: Res<VoxelWorldResource>,
    assets: Res<EffectAssets>,
    mut state: ResMut<WorldEffectsState>,
    mut commands: Commands,
) {
    const MAX_LANTERNS: usize = 96;
    if state.lanterns_spawned.len() >= MAX_LANTERNS {
        return;
    }
    let px = player.position.x as i32;
    let pz = player.position.z as i32;
    let mut rng = rand::thread_rng();
    for _ in 0..2 {
        let wx = px + rng.gen_range(-96..96);
        let wz = pz + rng.gen_range(-96..96);
        let key = (wx.div_euclid(48), wz.div_euclid(48));
        if state.lanterns_spawned.contains(&key) {
            continue;
        }
        if !world.inner.noise.is_land(wx, wz) || world.inner.noise.is_in_volcano(wx, wz) {
            continue;
        }
        let y = world.inner.noise.terrain_height(wx, wz) as f32 + 1.2;
        if y < crate::config::SEA_LEVEL as f32 + 1.0 {
            continue;
        }
        spawn_lantern(&mut commands, &assets, wx as f32 + 0.5, y, wz as f32 + 0.5, 900.0);
        state.lanterns_spawned.insert(key);
    }
}

fn spawn_lantern(
    commands: &mut Commands,
    assets: &EffectAssets,
    x: f32,
    y: f32,
    z: f32,
    intensity: f32,
) {
    commands.spawn((
        Mesh3d(assets.lantern_mesh.clone()),
        MeshMaterial3d(assets.pole_mat.clone()),
        Transform::from_xyz(x, y - 0.55, z).with_scale(Vec3::new(0.14, 1.1, 0.14)),
        LanternPost,
    ));
    commands.spawn((
        Mesh3d(assets.lantern_mesh.clone()),
        MeshMaterial3d(assets.lantern_mat.clone()),
        PointLight {
            color: Color::srgb(1.0, 0.78, 0.42),
            intensity,
            range: 14.0,
            shadows_enabled: !cfg!(target_arch = "wasm32"),
            ..default()
        },
        Transform::from_xyz(x, y, z),
        LanternPost,
    ));
}
