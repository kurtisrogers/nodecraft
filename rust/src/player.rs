use crate::blocks::{block_drop, BlockId};
use crate::collision::{self, Aabb};
use crate::config::{
    EYE_HEIGHT, FOG_END, FOG_START, GRAVITY, JUMP_VELOCITY, MOUSE_SENSITIVITY, PLAYER_HEIGHT,
    PLAYER_WIDTH, SPRINT_SPEED, WALK_SPEED, WORLD_HEIGHT,
};
use crate::inventory::GameInventory;
use crate::meshing::{ChunkMesh, RemeshQueue, VoxelWorldResource};
use crate::mobile::{is_controlling, MobileInput};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_pbr::{DistanceFog, FogFalloff};

const PLAYER_AABB: Aabb = Aabb {
    half_x: PLAYER_WIDTH * 0.5,
    half_z: PLAYER_WIDTH * 0.5,
    height: PLAYER_HEIGHT,
};

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
    pub walk_bob_phase: f32,
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
            walk_bob_phase: 0.0,
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
    player.pitch = if cfg!(target_arch = "wasm32") {
        // Mobile browsers start with the camera level; nudge down so ground is in view.
        -0.35
    } else {
        0.0
    };
    collision::ensure_clear(&world.inner, &mut player.position, PLAYER_AABB);

    let mut camera = commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            far: 512.0,
            near: 0.05,
            ..default()
        }),
        Transform::from_translation(player.position + Vec3::Y * EYE_HEIGHT)
            .with_rotation(Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0)),
        PlayerCamera,
    ));
    if cfg!(target_arch = "wasm32") {
        // MSAA and distance fog are unreliable on mobile WebGL2; keep the path simple.
        camera.insert(Msaa::Off);
    } else {
        camera.insert((
            DistanceFog {
                color: Color::srgb(0.53, 0.81, 0.92),
                falloff: FogFalloff::Linear {
                    start: FOG_START,
                    end: FOG_END,
                },
                ..default()
            },
        ));
    }
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
        player.yaw += mobile.look_delta.x;
        player.pitch -= mobile.look_delta.y;
    }
    if delta != Vec2::ZERO {
        player.yaw -= delta.x * MOUSE_SENSITIVITY;
        player.pitch -= delta.y * MOUSE_SENSITIVITY;
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

    let mut position = player.position;
    let mut velocity = player.velocity;
    let result = collision::move_aabb(
        &world.inner,
        &mut position,
        &mut velocity,
        PLAYER_AABB,
        dt,
        true,
    );
    player.position = position;
    player.velocity = velocity;
    player.on_ground = result.on_ground;
    if collision::overlaps_solid(&world.inner, player.position, PLAYER_AABB) {
        collision::ensure_clear(&world.inner, &mut player.position, PLAYER_AABB);
    }
    nudge_eye_clear(&world.inner, &mut player.position);

    let horizontal_speed = Vec2::new(player.velocity.x, player.velocity.z).length();
    if player.on_ground && horizontal_speed > 0.5 {
        player.walk_bob_phase += dt * horizontal_speed * 1.15;
    } else {
        player.walk_bob_phase *= 0.85;
    }

    if player.position.y < -10.0 {
        let px = player.position.x.floor() as i32;
        let pz = player.position.z.floor() as i32;
        let spawn = world.inner.find_safe_spawn(px, pz);
        player.position = Vec3::new(spawn.0, spawn.1, spawn.2);
        player.velocity = Vec3::ZERO;
        player.health = 20;
        collision::ensure_clear(&world.inner, &mut player.position, PLAYER_AABB);
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

fn nudge_eye_clear(world: &crate::world::VoxelWorld, pos: &mut Vec3) {
    let eye_y = pos.y + EYE_HEIGHT;
    for _ in 0..6 {
        let bx = pos.x.floor() as i32;
        let by = eye_y.floor() as i32;
        let bz = pos.z.floor() as i32;
        if !world.peek_block(bx, by, bz).solid() {
            break;
        }
        pos.y += 0.25;
        if !collision::overlaps_solid(world, *pos, PLAYER_AABB) {
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
    player: Res<PlayerState>,
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
                enqueue_chunk_and_neighbors(hit.block.x, hit.block.z, &mut queue);
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
            enqueue_chunk_and_neighbors(place.x, place.z, &mut queue);
        }
    }
}

fn enqueue_chunk_and_neighbors(wx: i32, wz: i32, queue: &mut RemeshQueue) {
    let cx = wx.div_euclid(crate::config::CHUNK_SIZE);
    let cz = wz.div_euclid(crate::config::CHUNK_SIZE);
    queue.push((cx, cz));
    let lx = wx.rem_euclid(crate::config::CHUNK_SIZE);
    let lz = wz.rem_euclid(crate::config::CHUNK_SIZE);
    if lx == 0 {
        queue.push((cx - 1, cz));
    }
    if lx == crate::config::CHUNK_SIZE - 1 {
        queue.push((cx + 1, cz));
    }
    if lz == 0 {
        queue.push((cx, cz - 1));
    }
    if lz == crate::config::CHUNK_SIZE - 1 {
        queue.push((cx, cz + 1));
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
    if mobile.is_mobile {
        return;
    }
    if keys.just_pressed(KeyCode::KeyE) || mobile.inventory_pressed {
        player.inventory_open = !player.inventory_open;
    }
    if keys.just_pressed(KeyCode::Escape) {
        player.inventory_open = false;
    }
}

pub fn mobile_hotbar_cycle(
    mut inventory: ResMut<GameInventory>,
    mobile: Res<MobileInput>,
) {
    if !mobile.is_mobile || !mobile.inventory_pressed {
        return;
    }
    inventory.hotbar_index = (inventory.hotbar_index + 1) % crate::config::HOTBAR_SIZE;
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
    if meshed_near < 1 {
        return;
    }
    player.terrain_ready = true;
    player.pitch = if cfg!(target_arch = "wasm32") { -0.35 } else { 0.0 };
    collision::ensure_clear(&world.inner, &mut player.position, PLAYER_AABB);
}

pub fn mobile_session_start(
    mut player: ResMut<PlayerState>,
    mut world: ResMut<VoxelWorldResource>,
    mobile: Res<MobileInput>,
    mut was_active: Local<bool>,
) {
    if mobile.is_mobile {
        player.inventory_open = false;
    }
    if mobile.active && !*was_active {
        player.pitch = if cfg!(target_arch = "wasm32") { -0.35 } else { 0.0 };
        player.yaw = 0.0;
        collision::ensure_clear(&world.inner, &mut player.position, PLAYER_AABB);
    }
    *was_active = mobile.active;
}

pub fn sync_camera(
    player: Res<PlayerState>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
) {
    if let Ok(mut transform) = camera.get_single_mut() {
        let horizontal_speed = Vec2::new(player.velocity.x, player.velocity.z).length();
        let bob = if player.on_ground && horizontal_speed > 0.5 {
            player.walk_bob_phase.sin() * 0.035
        } else {
            0.0
        };
        transform.translation = player.position + Vec3::new(0.0, EYE_HEIGHT + bob, 0.0);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    }
}
