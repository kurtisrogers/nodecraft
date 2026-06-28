use crate::blocks::{block_drop, BlockId};
use crate::config::{
    EYE_HEIGHT, GRAVITY, JUMP_VELOCITY, MOUSE_SENSITIVITY, PLAYER_HEIGHT, PLAYER_WIDTH, SPRINT_SPEED,
    WALK_SPEED, WORLD_HEIGHT,
};
use crate::inventory::GameInventory;
use crate::meshing::{ChunkMesh, RemeshQueue, VoxelWorldResource};
use crate::mobile::{is_controlling, MobileInput};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_pbr::{DistanceFog, FogFalloff};

#[derive(Resource)]
pub struct PlayerState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub health: i32,
    pub attack_cooldown: f32,
    pub lava_timer: f32,
    pub cursor_locked: bool,
    pub inventory_open: bool,
    pub terrain_ready: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 40.0, 0.0),
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            health: 20,
            attack_cooldown: 0.0,
            lava_timer: 0.0,
            cursor_locked: false,
            inventory_open: false,
            terrain_ready: false,
        }
    }
}

#[derive(Component)]
pub struct PlayerCamera;

pub fn spawn_player(
    mut commands: Commands,
    mut world: ResMut<VoxelWorldResource>,
    mut player: ResMut<PlayerState>,
) {
    let spawn = world.inner.find_safe_spawn(0, 0);
    player.position = Vec3::new(spawn.0, spawn.1, spawn.2);
    player.velocity = Vec3::ZERO;
    ensure_player_clear(&mut world.inner, &mut player.position);

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(player.position + Vec3::Y * EYE_HEIGHT),
        PlayerCamera,
        DistanceFog {
            color: Color::srgb(0.53, 0.81, 0.92),
            falloff: FogFalloff::Linear {
                start: if cfg!(target_arch = "wasm32") {
                    48.0
                } else {
                    crate::config::FOG_START
                },
                end: if cfg!(target_arch = "wasm32") {
                    128.0
                } else {
                    crate::config::FOG_END
                },
            },
            ..default()
        },
    ));
}

pub fn lock_cursor(
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mobile: Res<MobileInput>,
) {
    if player.inventory_open || mobile.is_mobile {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = window.get_single_mut() {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
            player.cursor_locked = false;
        }
        return;
    }
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(mut window) = window.get_single_mut() {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
            player.cursor_locked = true;
        }
    }
}

pub fn mouse_look(
    mut motion: EventReader<MouseMotion>,
    mut player: ResMut<PlayerState>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
    mobile: Res<MobileInput>,
) {
    if !is_controlling(&player, &mobile) {
        motion.clear();
        return;
    }
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if mobile.look_delta != Vec2::ZERO {
        player.yaw -= mobile.look_delta.x;
        player.pitch -= mobile.look_delta.y;
    }
    if delta != Vec2::ZERO {
        player.yaw -= delta.x * MOUSE_SENSITIVITY;
        player.pitch -= delta.y * MOUSE_SENSITIVITY;
    }
    if mobile.look_delta == Vec2::ZERO && delta == Vec2::ZERO {
        if let Ok(mut transform) = camera.get_single_mut() {
            transform.translation = player.position + Vec3::Y * EYE_HEIGHT;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
        }
        return;
    }
    player.pitch = player.pitch.clamp(-1.55, 1.55);

    if let Ok(mut transform) = camera.get_single_mut() {
        transform.translation = player.position + Vec3::Y * EYE_HEIGHT;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    }
}

pub fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mut world: ResMut<VoxelWorldResource>,
    mobile: Res<MobileInput>,
) {
    if !player.terrain_ready {
        player.velocity = Vec3::ZERO;
        return;
    }
    if player.inventory_open {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    let sprinting = keys.pressed(KeyCode::ShiftLeft) || mobile.sprint;
    let speed = if sprinting { SPRINT_SPEED } else { WALK_SPEED };

    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let right = Vec3::new(player.yaw.cos(), 0.0, -player.yaw.sin());
    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        wish += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        wish -= forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        wish -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        wish += right;
    }
    if mobile.move_vec.length_squared() > 0.0 {
        wish += forward * -mobile.move_vec.y;
        wish += right * mobile.move_vec.x;
    }
    if wish.length_squared() > 0.0 {
        wish = wish.normalize() * speed;
    }

    player.velocity.x = wish.x;
    player.velocity.z = wish.z;
    player.velocity.y += GRAVITY * dt;

    if player.on_ground && (keys.just_pressed(KeyCode::Space) || mobile.jump_pressed) {
        player.velocity.y = JUMP_VELOCITY;
        player.on_ground = false;
    }

    move_with_collision(&mut player, &mut world.inner, dt);

    if player.position.y < -10.0 {
        let px = player.position.x.floor() as i32;
        let pz = player.position.z.floor() as i32;
        let spawn = world.inner.find_safe_spawn(px, pz);
        player.position = Vec3::new(spawn.0, spawn.1, spawn.2);
        player.velocity = Vec3::ZERO;
        player.health = 20;
        ensure_player_clear(&mut world.inner, &mut player.position);
    }

    player.attack_cooldown = (player.attack_cooldown - dt).max(0.0);
    player.lava_timer = (player.lava_timer - dt).max(0.0);

    if feet_in_lava(&mut world.inner, &player) && player.lava_timer <= 0.0 {
        player.health -= 4;
        player.lava_timer = 0.5;
    }
}

fn feet_in_lava(world: &mut crate::world::VoxelWorld, player: &PlayerState) -> bool {
    let bx = player.position.x.floor() as i32;
    let by = player.position.y.floor() as i32;
    let bz = player.position.z.floor() as i32;
    world.get_block(bx, by, bz) == BlockId::Lava
        || world.get_block(bx, by - 1, bz) == BlockId::Lava
}

fn move_with_collision(player: &mut PlayerState, world: &mut crate::world::VoxelWorld, dt: f32) {
    let half = PLAYER_WIDTH * 0.5;
    let was_on_ground = player.on_ground;
    player.on_ground = false;

    if was_on_ground {
        try_step_up(player, world, half, dt);
    }

    let max_step = 0.45_f32;
    let steps = ((player.velocity.length() * dt) / max_step).ceil().max(1.0) as u32;
    let sub_dt = dt / steps as f32;

    for _ in 0..steps {
        let vy_before = player.velocity.y;

        player.position.x += player.velocity.x * sub_dt;
        resolve_axis(
            world,
            &mut player.position,
            half,
            PLAYER_HEIGHT,
            0,
            player.velocity.x.signum(),
        );

        player.position.z += player.velocity.z * sub_dt;
        resolve_axis(
            world,
            &mut player.position,
            half,
            PLAYER_HEIGHT,
            2,
            player.velocity.z.signum(),
        );

        player.position.y += player.velocity.y * sub_dt;
        let landed = resolve_axis(
            world,
            &mut player.position,
            half,
            PLAYER_HEIGHT,
            1,
            vy_before.signum(),
        );
        if landed && vy_before <= 0.0 {
            player.on_ground = true;
            player.velocity.y = 0.0;
        }
    }

    for _ in 0..8 {
        if !depenetrate_player(world, &mut player.position, half, PLAYER_HEIGHT) {
            break;
        }
    }
    nudge_eye_clear(world, &mut player.position, half);
    snap_to_ground(player, world, half);
}

const STEP_HEIGHT: f32 = 0.62;

fn try_step_up(player: &mut PlayerState, world: &mut crate::world::VoxelWorld, half: f32, dt: f32) {
    let wish_x = player.velocity.x * dt;
    let wish_z = player.velocity.z * dt;
    if wish_x.abs() < 1e-4 && wish_z.abs() < 1e-4 {
        return;
    }

    let saved = player.position;
    let raised = saved + Vec3::new(0.0, STEP_HEIGHT, 0.0);
    if !volume_clear(world, raised, half, PLAYER_HEIGHT) {
        return;
    }

    let mut test = raised;
    test.x += wish_x;
    resolve_axis(world, &mut test, half, PLAYER_HEIGHT, 0, wish_x.signum());
    test.z += wish_z;
    resolve_axis(world, &mut test, half, PLAYER_HEIGHT, 2, wish_z.signum());

    if !overlaps_solid(world, test, half, PLAYER_HEIGHT) {
        player.position = test;
    }
}

fn snap_to_ground(
    player: &mut PlayerState,
    world: &mut crate::world::VoxelWorld,
    half: f32,
) {
    if player.velocity.y > 0.5 {
        return;
    }
    let floor_y = ground_height(world, player.position.x, player.position.z);
    let Some(floor_y) = floor_y else {
        return;
    };
    let target = floor_y + 0.001;
    let gap = player.position.y - target;
    if gap > -0.08 && gap < 0.22 {
        player.position.y = target;
        player.velocity.y = 0.0;
        player.on_ground = true;
    }
    let _ = half;
}

fn ground_height(world: &mut crate::world::VoxelWorld, x: f32, z: f32) -> Option<f32> {
    let bx = x.floor() as i32;
    let bz = z.floor() as i32;
    for y in (0..WORLD_HEIGHT).rev() {
        if world.get_block(bx, y, bz).solid() {
            return Some(y as f32 + 1.0);
        }
    }
    None
}

fn player_aabb(pos: Vec3, half: f32, height: f32) -> ([f32; 3], [f32; 3]) {
    (
        [pos.x - half, pos.y, pos.z - half],
        [pos.x + half, pos.y + height, pos.z + half],
    )
}

fn volume_clear(
    world: &mut crate::world::VoxelWorld,
    pos: Vec3,
    half: f32,
    height: f32,
) -> bool {
    !overlaps_solid(world, pos, half, height)
}

fn overlaps_solid(
    world: &mut crate::world::VoxelWorld,
    pos: Vec3,
    half: f32,
    height: f32,
) -> bool {
    let (min, max) = player_aabb(pos, half, height);
    let min_b = [
        min[0].floor() as i32,
        min[1].floor() as i32,
        min[2].floor() as i32,
    ];
    let max_b = [
        (max[0] - 0.001).floor() as i32,
        (max[1] - 0.001).floor() as i32,
        (max[2] - 0.001).floor() as i32,
    ];

    for bx in min_b[0]..=max_b[0] {
        for by in min_b[1]..=max_b[1] {
            for bz in min_b[2]..=max_b[2] {
                if world.get_block(bx, by, bz).solid() {
                    return true;
                }
            }
        }
    }
    false
}

fn ensure_player_clear(world: &mut crate::world::VoxelWorld, pos: &mut Vec3) {
    let half = PLAYER_WIDTH * 0.5;
    for _ in 0..12 {
        if !depenetrate_player(world, pos, half, PLAYER_HEIGHT) {
            break;
        }
    }
    nudge_eye_clear(world, pos, half);
    if overlaps_solid(world, *pos, half, PLAYER_HEIGHT) {
        if let Some(floor_y) = ground_height(world, pos.x, pos.z) {
            pos.y = floor_y + 0.5;
            for _ in 0..8 {
                if !depenetrate_player(world, pos, half, PLAYER_HEIGHT) {
                    break;
                }
            }
        }
    }
}

fn resolve_axis(
    world: &mut crate::world::VoxelWorld,
    pos: &mut Vec3,
    half: f32,
    height: f32,
    axis: usize,
    velocity_sign: f32,
) -> bool {
    const EPS: f32 = 0.001;
    let min = [pos.x - half, pos.y, pos.z - half];
    let max = [pos.x + half, pos.y + height, pos.z + half];
    let min_b = [
        min[0].floor() as i32,
        min[1].floor() as i32,
        min[2].floor() as i32,
    ];
    let max_b = [
        (max[0] - EPS).floor() as i32,
        (max[1] - EPS).floor() as i32,
        (max[2] - EPS).floor() as i32,
    ];

    let mut hit = false;
    match axis {
        0 => {
            let mut new_x = pos.x;
            let mut push_left = 0.0_f32;
            let mut push_right = 0.0_f32;
            for bx in min_b[0]..=max_b[0] {
                for by in min_b[1]..=max_b[1] {
                    for bz in min_b[2]..=max_b[2] {
                        if !world.get_block(bx, by, bz).solid() {
                            continue;
                        }
                        let block_min = bx as f32;
                        let block_max = block_min + 1.0;
                        if max[0] <= block_min
                            || min[0] >= block_max
                            || max[1] <= by as f32
                            || min[1] >= (by + 1) as f32
                            || max[2] <= bz as f32
                            || min[2] >= (bz + 1) as f32
                        {
                            continue;
                        }
                        hit = true;
                        if velocity_sign > 0.0 {
                            new_x = new_x.min(block_min - half - EPS);
                        } else if velocity_sign < 0.0 {
                            new_x = new_x.max(block_max + half + EPS);
                        } else {
                            push_left = push_left.max((pos.x + half) - block_min);
                            push_right = push_right.max(block_max - (pos.x - half));
                        }
                    }
                }
            }
            if velocity_sign == 0.0 && (push_left > 0.0 || push_right > 0.0) {
                new_x = if push_left < push_right {
                    pos.x - push_left - EPS
                } else {
                    pos.x + push_right + EPS
                };
            }
            pos.x = new_x;
        }
        1 => {
            let mut new_y = pos.y;
            let mut push_down = 0.0_f32;
            let mut push_up = 0.0_f32;
            for bx in min_b[0]..=max_b[0] {
                for by in min_b[1]..=max_b[1] {
                    for bz in min_b[2]..=max_b[2] {
                        if !world.get_block(bx, by, bz).solid() {
                            continue;
                        }
                        let block_min = by as f32;
                        let block_max = block_min + 1.0;
                        if max[0] <= bx as f32
                            || min[0] >= (bx + 1) as f32
                            || max[1] <= block_min
                            || min[1] >= block_max
                            || max[2] <= bz as f32
                            || min[2] >= (bz + 1) as f32
                        {
                            continue;
                        }
                        hit = true;
                        if velocity_sign > 0.0 {
                            new_y = new_y.min(block_min - height - EPS);
                        } else if velocity_sign < 0.0 {
                            new_y = new_y.max(block_max + EPS);
                        } else {
                            push_down = push_down.max((pos.y + height) - block_min);
                            push_up = push_up.max(block_max - pos.y);
                        }
                    }
                }
            }
            if velocity_sign == 0.0 && (push_down > 0.0 || push_up > 0.0) {
                new_y = if push_down < push_up {
                    pos.y - push_down - EPS
                } else {
                    pos.y + push_up + EPS
                };
            }
            pos.y = new_y;
        }
        _ => {
            let mut new_z = pos.z;
            let mut push_near = 0.0_f32;
            let mut push_far = 0.0_f32;
            for bx in min_b[0]..=max_b[0] {
                for by in min_b[1]..=max_b[1] {
                    for bz in min_b[2]..=max_b[2] {
                        if !world.get_block(bx, by, bz).solid() {
                            continue;
                        }
                        let block_min = bz as f32;
                        let block_max = block_min + 1.0;
                        if max[0] <= bx as f32
                            || min[0] >= (bx + 1) as f32
                            || max[1] <= by as f32
                            || min[1] >= (by + 1) as f32
                            || max[2] <= block_min
                            || min[2] >= block_max
                        {
                            continue;
                        }
                        hit = true;
                        if velocity_sign > 0.0 {
                            new_z = new_z.min(block_min - half - EPS);
                        } else if velocity_sign < 0.0 {
                            new_z = new_z.max(block_max + half + EPS);
                        } else {
                            push_near = push_near.max((pos.z + half) - block_min);
                            push_far = push_far.max(block_max - (pos.z - half));
                        }
                    }
                }
            }
            if velocity_sign == 0.0 && (push_near > 0.0 || push_far > 0.0) {
                new_z = if push_near < push_far {
                    pos.z - push_near - EPS
                } else {
                    pos.z + push_far + EPS
                };
            }
            pos.z = new_z;
        }
    }
    hit
}

fn depenetrate_player(
    world: &mut crate::world::VoxelWorld,
    pos: &mut Vec3,
    half: f32,
    height: f32,
) -> bool {
    let min = [pos.x - half, pos.y, pos.z - half];
    let max = [pos.x + half, pos.y + height, pos.z + half];
    let min_b = [
        min[0].floor() as i32,
        min[1].floor() as i32,
        min[2].floor() as i32,
    ];
    let max_b = [
        (max[0] - 0.001).floor() as i32,
        (max[1] - 0.001).floor() as i32,
        (max[2] - 0.001).floor() as i32,
    ];

    let mut best_push = Vec3::ZERO;
    let mut best_depth = f32::INFINITY;

    for bx in min_b[0]..=max_b[0] {
        for by in min_b[1]..=max_b[1] {
            for bz in min_b[2]..=max_b[2] {
                if !world.get_block(bx, by, bz).solid() {
                    continue;
                }
                let block_min = Vec3::new(bx as f32, by as f32, bz as f32);
                let block_max = block_min + Vec3::ONE;
                if max[0] <= block_min.x
                    || min[0] >= block_max.x
                    || max[1] <= block_min.y
                    || min[1] >= block_max.y
                    || max[2] <= block_min.z
                    || min[2] >= block_max.z
                {
                    continue;
                }

                let push_x_neg = (pos.x + half) - block_min.x;
                let push_x_pos = block_max.x - (pos.x - half);
                let push_y_neg = (pos.y + height) - block_min.y;
                let push_y_pos = block_max.y - pos.y;
                let push_z_neg = (pos.z + half) - block_min.z;
                let push_z_pos = block_max.z - (pos.z - half);

                let candidates = [
                    (Vec3::new(-push_x_neg, 0.0, 0.0), push_x_neg),
                    (Vec3::new(push_x_pos, 0.0, 0.0), push_x_pos),
                    (Vec3::new(0.0, -push_y_neg, 0.0), push_y_neg),
                    (Vec3::new(0.0, push_y_pos, 0.0), push_y_pos),
                    (Vec3::new(0.0, 0.0, -push_z_neg), push_z_neg),
                    (Vec3::new(0.0, 0.0, push_z_pos), push_z_pos),
                ];

                for (push, depth) in candidates {
                    if depth > 0.0 && depth < best_depth {
                        best_depth = depth;
                        best_push = push;
                    }
                }
            }
        }
    }

    if best_depth.is_finite() {
        *pos += best_push + best_push.signum() * Vec3::splat(0.001);
        return true;
    }
    false
}

fn nudge_eye_clear(world: &mut crate::world::VoxelWorld, pos: &mut Vec3, half: f32) {
    let eye_y = pos.y + EYE_HEIGHT;
    let ex = pos.x;
    let ez = pos.z;
    for _ in 0..6 {
        let bx = ex.floor() as i32;
        let by = eye_y.floor() as i32;
        let bz = ez.floor() as i32;
        if !world.get_block(bx, by, bz).solid() {
            break;
        }
        pos.y += 0.25;
        if world.is_volume_clear(pos.x, pos.y, pos.z, half, PLAYER_HEIGHT) {
            break;
        }
    }
}

pub struct RayHit {
    pub block: IVec3,
    pub face: IVec3,
}

pub fn raycast(world: &mut crate::world::VoxelWorld, origin: Vec3, direction: Vec3, max_dist: f32) -> Option<RayHit> {
    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;
    let step_x = if direction.x > 0.0 { 1 } else { -1 };
    let step_y = if direction.y > 0.0 { 1 } else { -1 };
    let step_z = if direction.z > 0.0 { 1 } else { -1 };
    let t_delta_x = if direction.x.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / direction.x).abs()
    };
    let t_delta_y = if direction.y.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / direction.y).abs()
    };
    let t_delta_z = if direction.z.abs() < 1e-8 {
        f32::INFINITY
    } else {
        (1.0 / direction.z).abs()
    };
    let mut t_max_x = if step_x > 0 {
        (x as f32 + 1.0 - origin.x) * t_delta_x
    } else {
        (origin.x - x as f32) * t_delta_x
    };
    let mut t_max_y = if step_y > 0 {
        (y as f32 + 1.0 - origin.y) * t_delta_y
    } else {
        (origin.y - y as f32) * t_delta_y
    };
    let mut t_max_z = if step_z > 0 {
        (z as f32 + 1.0 - origin.z) * t_delta_z
    } else {
        (origin.z - z as f32) * t_delta_z
    };
    let mut traveled = 0.0f32;
    let mut last_normal = IVec3::ZERO;

    while traveled < max_dist {
        let block = world.get_block(x, y, z);
        if block.solid() {
            return Some(RayHit {
                block: IVec3::new(x, y, z),
                face: IVec3::new(x + last_normal.x, y + last_normal.y, z + last_normal.z),
            });
        }
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                traveled = t_max_x;
                t_max_x += t_delta_x;
                x += step_x;
                last_normal = IVec3::new(-step_x, 0, 0);
            } else {
                traveled = t_max_z;
                t_max_z += t_delta_z;
                z += step_z;
                last_normal = IVec3::new(0, 0, -step_z);
            }
        } else if t_max_y < t_max_z {
            traveled = t_max_y;
            t_max_y += t_delta_y;
            y += step_y;
            last_normal = IVec3::new(0, -step_y, 0);
        } else {
            traveled = t_max_z;
            t_max_z += t_delta_z;
            z += step_z;
            last_normal = IVec3::new(0, 0, -step_z);
        }
        if y < 0 || y >= WORLD_HEIGHT {
            break;
        }
    }
    None
}

pub fn block_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    mut player: ResMut<PlayerState>,
    mut world: ResMut<VoxelWorldResource>,
    mut inventory: ResMut<GameInventory>,
    mut queue: ResMut<RemeshQueue>,
    camera: Query<&Transform, With<PlayerCamera>>,
    mobile: Res<MobileInput>,
) {
    if !is_controlling(&player, &mobile) {
        return;
    }
    let Ok(cam) = camera.get_single() else { return };
    let direction = cam.rotation * -Vec3::Z;
    let origin = player.position + Vec3::Y * EYE_HEIGHT;

    if mouse.just_pressed(MouseButton::Left) || mobile.break_pressed {
        if player.attack_cooldown >= 0.35 {
            return;
        }
        if let Some(hit) = raycast(&mut world.inner, origin, direction, 6.0) {
            let block = world.inner.get_block(hit.block.x, hit.block.y, hit.block.z);
            if block != BlockId::Bedrock && block != BlockId::Lava {
                if let Some(drop) = block_drop(block) {
                    inventory.add_item(drop as u16, 1);
                }
                world.inner.set_block(hit.block.x, hit.block.y, hit.block.z, BlockId::Air);
                enqueue_chunk_and_neighbors(&mut world.inner, hit.block.x, hit.block.z, &mut queue);
            }
        }
    }

    if mouse.just_pressed(MouseButton::Right) || mobile.place_pressed {
        let Some(item) = inventory.hotbar_item() else { return };
        if !inventory.has_item(item, 1) {
            return;
        }
        if let Some(hit) = raycast(&mut world.inner, origin, direction, 6.0) {
            let place = hit.face;
            let px = player.position.x.floor() as i32;
            let py = player.position.y.floor() as i32;
            let pz = player.position.z.floor() as i32;
            if place.x == px && place.z == pz && place.y >= py && place.y <= py + 1 {
                return;
            }
            inventory.remove_item(item, 1);
            world
                .inner
                .set_block(place.x, place.y, place.z, BlockId::from_u8(item as u8));
            enqueue_chunk_and_neighbors(&mut world.inner, place.x, place.z, &mut queue);
        }
    }
}

fn enqueue_chunk_and_neighbors(world: &crate::world::VoxelWorld, wx: i32, wz: i32, queue: &mut RemeshQueue) {
    let cx = wx.div_euclid(crate::config::CHUNK_SIZE);
    let cz = wz.div_euclid(crate::config::CHUNK_SIZE);
    queue.keys.push((cx, cz));
    let lx = wx.rem_euclid(crate::config::CHUNK_SIZE);
    let lz = wz.rem_euclid(crate::config::CHUNK_SIZE);
    if lx == 0 {
        queue.keys.push((cx - 1, cz));
    }
    if lx == crate::config::CHUNK_SIZE - 1 {
        queue.keys.push((cx + 1, cz));
    }
    if lz == 0 {
        queue.keys.push((cx, cz - 1));
    }
    if lz == crate::config::CHUNK_SIZE - 1 {
        queue.keys.push((cx, cz + 1));
    }
}

pub fn hotbar_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<GameInventory>,
    mobile: Res<MobileInput>,
) {
    if let Some(index) = mobile.hotbar_select {
        if index < crate::config::HOTBAR_SIZE {
            inventory.hotbar_index = index;
        }
    }
    let digit_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    for (i, key) in digit_keys.iter().enumerate() {
        if keys.just_pressed(*key) {
            inventory.hotbar_index = i;
        }
    }
}

pub fn toggle_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<PlayerState>,
    mobile: Res<MobileInput>,
) {
    if keys.just_pressed(KeyCode::KeyE) || mobile.inventory_pressed {
        player.inventory_open = !player.inventory_open;
    }
    if keys.just_pressed(KeyCode::Escape) {
        player.inventory_open = false;
    }
}

pub fn update_terrain_ready(
    mut player: ResMut<PlayerState>,
    mut world: ResMut<VoxelWorldResource>,
    meshes: Query<&ChunkMesh>,
) {
    if player.terrain_ready {
        return;
    }
    let (pcx, pcz) = world.player_chunk;
    let meshed_near = meshes
        .iter()
        .filter(|mesh| (mesh.chunk_x - pcx).abs() <= 1 && (mesh.chunk_z - pcz).abs() <= 1)
        .count();
    if meshed_near < 5 {
        return;
    }
    player.terrain_ready = true;
    ensure_player_clear(&mut world.inner, &mut player.position);
}

pub fn sync_camera(
    player: Res<PlayerState>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
) {
    if let Ok(mut transform) = camera.get_single_mut() {
        transform.translation = player.position + Vec3::Y * EYE_HEIGHT;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    }
}
