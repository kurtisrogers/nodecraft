use crate::collision::{self, Aabb};
use crate::config::DAY_LENGTH_SECS;
use crate::meshing::VoxelWorldResource;
use crate::mobile::{is_controlling, MobileInput};
use crate::player::PlayerState;
use bevy::prelude::*;
use rand::Rng;

const MOB_CAP: usize = 36;
const SPAWN_INTERVAL: f32 = 3.5;
const WASM_MOB_CAP: usize = 14;
const WASM_SPAWN_INTERVAL: f32 = 5.5;
const ATTACK_DAMAGE: i32 = 5;
const ATTACK_RANGE: f32 = 4.0;

fn mob_cap() -> usize {
    if cfg!(target_arch = "wasm32") {
        WASM_MOB_CAP
    } else {
        MOB_CAP
    }
}

fn spawn_interval() -> f32 {
    if cfg!(target_arch = "wasm32") {
        WASM_SPAWN_INTERVAL
    } else {
        SPAWN_INTERVAL
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobType {
    Pig,
    Cow,
    Sheep,
    Chicken,
    Zombie,
}

impl MobType {
    fn health(self) -> i32 {
        match self {
            Self::Pig | Self::Cow => 10,
            Self::Sheep => 8,
            Self::Chicken => 4,
            Self::Zombie => 20,
        }
    }

    fn speed(self) -> f32 {
        match self {
            Self::Cow => 1.5,
            Self::Sheep => 1.8,
            Self::Pig => 2.0,
            Self::Chicken => 2.2,
            Self::Zombie => 3.5,
        }
    }

    fn hostile(self) -> bool {
        matches!(self, Self::Zombie)
    }

    fn color(self) -> Color {
        match self {
            Self::Pig => Color::srgb(1.0, 0.71, 0.76),
            Self::Cow => Color::srgb(0.55, 0.45, 0.33),
            Self::Sheep => Color::srgb(0.95, 0.95, 0.95),
            Self::Chicken => Color::srgb(1.0, 1.0, 1.0),
            Self::Zombie => Color::srgb(0.29, 0.49, 0.31),
        }
    }

    fn size(self) -> Vec3 {
        match self {
            Self::Pig => Vec3::new(0.9, 0.9, 1.4),
            Self::Cow => Vec3::new(1.0, 1.4, 1.6),
            Self::Sheep => Vec3::new(0.85, 1.0, 1.3),
            Self::Chicken => Vec3::new(0.45, 0.55, 0.55),
            Self::Zombie => Vec3::new(0.6, 1.8, 0.6),
        }
    }

    fn aabb(self) -> Aabb {
        let size = self.size();
        Aabb::new(size.x * 0.5, size.z * 0.5, size.y)
    }
}

#[derive(Component)]
pub struct MobEntity {
    pub kind: MobType,
    pub health: i32,
    pub velocity: Vec3,
    pub wander_timer: f32,
    pub wander_dir: Vec3,
    pub alive: bool,
}

#[derive(Resource, Default)]
pub struct MobManager {
    pub count: usize,
    pub next_id: u32,
    pub spawn_timer: f32,
}

pub fn mob_spawner(
    time: Res<Time>,
    player: Res<PlayerState>,
    day: Res<crate::weather::DayNight>,
    mut manager: ResMut<MobManager>,
    mut world: ResMut<VoxelWorldResource>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing: Query<&MobEntity>,
) {
    manager.count = existing.iter().filter(|m| m.alive).count();
    if manager.count >= mob_cap() {
        return;
    }
    if !player.terrain_ready {
        return;
    }
    manager.spawn_timer -= time.delta_secs();
    if manager.spawn_timer > 0.0 {
        return;
    }
    manager.spawn_timer = spawn_interval();

    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let dist = rng.gen_range(12.0..36.0);
    let x = player.position.x + angle.cos() * dist;
    let z = player.position.z + angle.sin() * dist;
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;

    if !world.inner.noise.is_land(ix, iz) {
        return;
    }
    if world.inner.noise.is_in_volcano(ix, iz) {
        return;
    }

    let spawn = world.inner.find_safe_spawn(ix, iz);
    if spawn.1 < 3.0 {
        return;
    }

    let is_night = is_night(&day);
    let kind = pick_spawn_type(is_night, &mut rng);
    spawn_mob(
        &mut commands,
        &mut manager,
        &mut materials,
        &mut meshes,
        kind,
        Vec3::new(spawn.0, spawn.1, spawn.2),
    );
}

fn spawn_mob(
    commands: &mut Commands,
    manager: &mut MobManager,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    kind: MobType,
    position: Vec3,
) {
    manager.next_id += 1;
    let size = kind.size();
    let mat = materials.add(StandardMaterial {
        base_color: kind.color(),
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(mat),
        Transform::from_translation(position + Vec3::Y * size.y * 0.5),
        MobEntity {
            kind,
            health: kind.health(),
            velocity: Vec3::ZERO,
            wander_timer: 2.0,
            wander_dir: Vec3::Z,
            alive: true,
        },
    ));
}

fn pick_spawn_type(is_night: bool, rng: &mut impl Rng) -> MobType {
    let roll: f32 = rng.gen();
    if is_night {
        if roll < 0.28 {
            MobType::Zombie
        } else if roll < 0.48 {
            MobType::Pig
        } else if roll < 0.68 {
            MobType::Cow
        } else if roll < 0.86 {
            MobType::Sheep
        } else {
            MobType::Chicken
        }
    } else if roll < 0.28 {
        MobType::Pig
    } else if roll < 0.56 {
        MobType::Cow
    } else if roll < 0.8 {
        MobType::Sheep
    } else {
        MobType::Chicken
    }
}

pub fn mob_ai(
    time: Res<Time>,
    player: Res<PlayerState>,
    day: Res<crate::weather::DayNight>,
    mut world: ResMut<VoxelWorldResource>,
    mut query: Query<(&mut MobEntity, &mut Transform)>,
) {
    let dt = time.delta_secs().min(0.05);
    let is_night = is_night(&day);
    let mut rng = rand::thread_rng();

    for (mut mob, mut transform) in query.iter_mut() {
        if !mob.alive {
            continue;
        }

        let kind = mob.kind;
        let aabb = kind.aabb();
        let mut move_dir = Vec3::ZERO;

        if kind.hostile() && is_night {
            let to_player = player.position - transform.translation;
            let flat = Vec3::new(to_player.x, 0.0, to_player.z);
            let dist = flat.length();
            if dist < 24.0 && dist > 0.5 {
                move_dir = flat.normalize();
            }
        } else {
            mob.wander_timer -= dt;
            if mob.wander_timer <= 0.0 {
                mob.wander_timer = rng.gen_range(2.0..5.0);
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                mob.wander_dir = Vec3::new(angle.cos(), 0.0, angle.sin());
            }
            move_dir = mob.wander_dir;
        }

        mob.velocity.x = move_dir.x * kind.speed();
        mob.velocity.z = move_dir.z * kind.speed();
        mob.velocity.y -= 20.0 * dt;

        let mut feet = transform.translation - Vec3::Y * aabb.height * 0.5;
        let result = collision::move_aabb(
            &world.inner,
            &mut feet,
            &mut mob.velocity,
            aabb,
            dt,
            true,
        );
        transform.translation = feet + Vec3::Y * aabb.height * 0.5;

        if result.on_ground && mob.velocity.y < 0.0 {
            mob.velocity.y = 0.0;
        }

        if move_dir.length_squared() > 0.01 {
            transform.rotation = Quat::from_rotation_y(move_dir.x.atan2(move_dir.z));
        }
    }
}

pub fn mob_attack_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    mut player: ResMut<PlayerState>,
    mut mobs: Query<(Entity, &mut MobEntity, &Transform)>,
    mut commands: Commands,
    mobile: Res<MobileInput>,
    ui: Res<crate::menu::GameUiState>,
) {
    if !is_controlling(&player, &mobile, &ui) || player.attack_cooldown > 0.0 {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) && !mobile.break_pressed {
        return;
    }

    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let eye = player.position + Vec3::Y * crate::config::EYE_HEIGHT;
    let mut best: Option<(Entity, f32)> = None;

    for (entity, mob, transform) in mobs.iter() {
        if !mob.alive {
            continue;
        }
        let to_mob = transform.translation - eye;
        let flat = Vec3::new(to_mob.x, 0.0, to_mob.z);
        let dist = flat.length();
        if dist > ATTACK_RANGE || dist < 0.01 {
            continue;
        }
        if forward.dot(flat / dist) < 0.6 {
            continue;
        }
        if best.map(|(_, d)| dist < d).unwrap_or(true) {
            best = Some((entity, dist));
        }
    }

    if let Some((entity, _)) = best {
        player.attack_cooldown = 0.4;
        if let Ok((_, mut mob, _)) = mobs.get_mut(entity) {
            mob.health -= ATTACK_DAMAGE;
            if mob.health <= 0 {
                mob.alive = false;
                commands.entity(entity).despawn();
            }
        }
    }
}

fn is_night(day: &crate::weather::DayNight) -> bool {
    let cycle = (day.time % DAY_LENGTH_SECS) / DAY_LENGTH_SECS;
    cycle > 0.5
}
