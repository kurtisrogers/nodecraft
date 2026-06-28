use crate::config::WORLD_HEIGHT;
use crate::world::VoxelWorld;
use bevy::prelude::*;

const EPS: f32 = 0.001;
const SKIN: f32 = 0.001;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub half_x: f32,
    pub half_z: f32,
    pub height: f32,
}

impl Aabb {
    pub fn new(half_x: f32, half_z: f32, height: f32) -> Self {
        Self {
            half_x,
            half_z,
            height,
        }
    }

    pub fn from_uniform(half: f32, height: f32) -> Self {
        Self::new(half, half, height)
    }

    fn bounds(self, pos: Vec3) -> ([f32; 3], [f32; 3]) {
        (
            [pos.x - self.half_x, pos.y, pos.z - self.half_z],
            [
                pos.x + self.half_x,
                pos.y + self.height,
                pos.z + self.half_z,
            ],
        )
    }
}

pub struct MoveResult {
    pub on_ground: bool,
    pub hit_ceiling: bool,
}

pub fn move_aabb(
    world: &mut VoxelWorld,
    pos: &mut Vec3,
    velocity: &mut Vec3,
    aabb: Aabb,
    dt: f32,
    step_up: bool,
) -> MoveResult {
    let was_on_ground = is_on_ground(world, *pos, aabb);
    let mut on_ground = false;
    let mut hit_ceiling = false;

    if step_up && was_on_ground {
        try_step_up(world, pos, aabb, velocity, dt);
    }

    let speed = velocity.length() * dt;
    let steps = (speed / 0.4).ceil().max(1.0) as u32;
    let sub_dt = dt / steps as f32;

    for _ in 0..steps {
        let vy_before = velocity.y;

        pos.x += velocity.x * sub_dt;
        resolve_axis(world, pos, aabb, 0, velocity.x.signum());

        pos.z += velocity.z * sub_dt;
        resolve_axis(world, pos, aabb, 2, velocity.z.signum());

        pos.y += velocity.y * sub_dt;
        if resolve_axis(world, pos, aabb, 1, vy_before.signum()) {
            if vy_before < 0.0 {
                on_ground = true;
                velocity.y = 0.0;
            } else if vy_before > 0.0 {
                hit_ceiling = true;
                velocity.y = 0.0;
            }
        }
    }

    for _ in 0..6 {
        if !depenetrate(world, pos, aabb) {
            break;
        }
    }

    if is_on_ground(world, *pos, aabb) {
        on_ground = true;
        if velocity.y < 0.0 {
            velocity.y = 0.0;
        }
        snap_feet_to_floor(world, pos, aabb);
    }

    if overlaps_solid(world, *pos, aabb) {
        ensure_clear(world, pos, aabb);
    }

    MoveResult {
        on_ground,
        hit_ceiling,
    }
}

pub fn depenetrate(world: &mut VoxelWorld, pos: &mut Vec3, aabb: Aabb) -> bool {
    let (min, max) = aabb.bounds(*pos);
    let min_b = block_min(min);
    let max_b = block_max(max);

    let mut best_push = Vec3::ZERO;
    let mut best_depth = f32::INFINITY;

    for bx in min_b[0]..=max_b[0] {
        for by in min_b[1]..=max_b[1] {
            for bz in min_b[2]..=max_b[2] {
                if !world.get_block(bx, by, bz).solid() {
                    continue;
                }
                let block_min_v = Vec3::new(bx as f32, by as f32, bz as f32);
                let block_max_v = block_min_v + Vec3::ONE;
                if !overlaps(min, max, block_min_v, block_max_v) {
                    continue;
                }

                let block_center = (block_min_v + block_max_v) * 0.5;
                let player_center = Vec3::new(pos.x, pos.y + aabb.height * 0.5, pos.z);

                let overlap_x = (max[0] - block_min_v.x).min(block_max_v.x - min[0]);
                let overlap_y = (max[1] - block_min_v.y).min(block_max_v.y - min[1]);
                let overlap_z = (max[2] - block_min_v.z).min(block_max_v.z - min[2]);

                let push_y = if pos.y + aabb.height * 0.5 < block_center.y {
                    block_max_v.y - pos.y
                } else {
                    -((pos.y + aabb.height) - block_min_v.y)
                };

                let candidates = [
                    (
                        Vec3::new(
                            if player_center.x < block_center.x {
                                -overlap_x
                            } else {
                                overlap_x
                            },
                            0.0,
                            0.0,
                        ),
                        overlap_x,
                    ),
                    (Vec3::new(0.0, push_y, 0.0), overlap_y),
                    (
                        Vec3::new(
                            0.0,
                            0.0,
                            if player_center.z < block_center.z {
                                -overlap_z
                            } else {
                                overlap_z
                            },
                        ),
                        overlap_z,
                    ),
                ];

                for (push, depth) in candidates {
                    if depth > EPS && depth < best_depth {
                        best_depth = depth;
                        best_push = push;
                    }
                }
            }
        }
    }

    if best_depth.is_finite() {
        *pos += best_push + best_push.signum() * Vec3::splat(SKIN);
        return true;
    }
    false
}

pub fn ensure_clear(world: &mut VoxelWorld, pos: &mut Vec3, aabb: Aabb) {
    for _ in 0..10 {
        if !depenetrate(world, pos, aabb) {
            break;
        }
    }
    if overlaps_solid(world, *pos, aabb) {
        if let Some(floor) = floor_height(world, pos.x, pos.z, aabb) {
            pos.y = floor + SKIN;
            for _ in 0..8 {
                if !depenetrate(world, pos, aabb) {
                    break;
                }
            }
        }
    }
}

pub fn overlaps_solid(world: &mut VoxelWorld, pos: Vec3, aabb: Aabb) -> bool {
    let (min, max) = aabb.bounds(pos);
    let min_b = block_min(min);
    let max_b = block_max(max);

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

pub fn is_on_ground(world: &mut VoxelWorld, pos: Vec3, aabb: Aabb) -> bool {
    let probe = pos.y - 0.08;
    let samples = [
        (0.0, 0.0),
        (aabb.half_x * 0.7, 0.0),
        (-aabb.half_x * 0.7, 0.0),
        (0.0, aabb.half_z * 0.7),
        (0.0, -aabb.half_z * 0.7),
    ];
    for (dx, dz) in samples {
        let bx = (pos.x + dx).floor() as i32;
        let by = probe.floor() as i32;
        let bz = (pos.z + dz).floor() as i32;
        if world.get_block(bx, by, bz).solid() {
            return true;
        }
    }
    false
}

fn try_step_up(
    world: &mut VoxelWorld,
    pos: &mut Vec3,
    aabb: Aabb,
    velocity: &Vec3,
    dt: f32,
) {
    const STEP: f32 = 0.62;
    let wish_x = velocity.x * dt;
    let wish_z = velocity.z * dt;
    if wish_x.abs() < 1e-4 && wish_z.abs() < 1e-4 {
        return;
    }

    let saved = *pos;
    let mut flat = saved;
    flat.x += wish_x;
    resolve_axis(world, &mut flat, aabb, 0, wish_x.signum());
    flat.z += wish_z;
    resolve_axis(world, &mut flat, aabb, 2, wish_z.signum());

    // Flat path is clear — let the main movement loop handle it.
    if !overlaps_solid(world, flat, aabb) {
        return;
    }

    let raised = saved + Vec3::new(0.0, STEP, 0.0);
    if overlaps_solid(world, raised, aabb) {
        return;
    }

    let mut test = raised;
    test.x += wish_x;
    resolve_axis(world, &mut test, aabb, 0, wish_x.signum());
    test.z += wish_z;
    resolve_axis(world, &mut test, aabb, 2, wish_z.signum());

    if !overlaps_solid(world, test, aabb) {
        *pos = test;
    }
}

fn resolve_axis(
    world: &mut VoxelWorld,
    pos: &mut Vec3,
    aabb: Aabb,
    axis: usize,
    velocity_sign: f32,
) -> bool {
    let (min, max) = aabb.bounds(*pos);
    let min_b = block_min(min);
    let max_b = block_max(max);
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
                        let block_min_x = bx as f32;
                        let block_max_x = block_min_x + 1.0;
                        if !overlap_1d(min[0], max[0], block_min_x, block_max_x)
                            || !overlap_1d(min[1], max[1], by as f32, (by + 1) as f32)
                            || !overlap_1d(min[2], max[2], bz as f32, (bz + 1) as f32)
                        {
                            continue;
                        }
                        hit = true;
                        if velocity_sign > 0.0 {
                            new_x = new_x.min(block_min_x - aabb.half_x - EPS);
                        } else if velocity_sign < 0.0 {
                            new_x = new_x.max(block_max_x + aabb.half_x + EPS);
                        } else {
                            push_left = push_left.max((pos.x + aabb.half_x) - block_min_x);
                            push_right = push_right.max(block_max_x - (pos.x - aabb.half_x));
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
                        let block_min_y = by as f32;
                        let block_max_y = block_min_y + 1.0;
                        if !overlap_1d(min[0], max[0], bx as f32, (bx + 1) as f32)
                            || !overlap_1d(min[1], max[1], block_min_y, block_max_y)
                            || !overlap_1d(min[2], max[2], bz as f32, (bz + 1) as f32)
                        {
                            continue;
                        }
                        hit = true;
                        if velocity_sign > 0.0 {
                            new_y = new_y.min(block_min_y - aabb.height - EPS);
                        } else if velocity_sign < 0.0 {
                            new_y = new_y.max(block_max_y + EPS);
                        } else {
                            push_down = push_down.max((pos.y + aabb.height) - block_min_y);
                            push_up = push_up.max(block_max_y - pos.y);
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
                        let block_min_z = bz as f32;
                        let block_max_z = block_min_z + 1.0;
                        if !overlap_1d(min[0], max[0], bx as f32, (bx + 1) as f32)
                            || !overlap_1d(min[1], max[1], by as f32, (by + 1) as f32)
                            || !overlap_1d(min[2], max[2], block_min_z, block_max_z)
                        {
                            continue;
                        }
                        hit = true;
                        if velocity_sign > 0.0 {
                            new_z = new_z.min(block_min_z - aabb.half_z - EPS);
                        } else if velocity_sign < 0.0 {
                            new_z = new_z.max(block_max_z + aabb.half_z + EPS);
                        } else {
                            push_near = push_near.max((pos.z + aabb.half_z) - block_min_z);
                            push_far = push_far.max(block_max_z - (pos.z - aabb.half_z));
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

pub fn snap_feet_to_floor(world: &mut VoxelWorld, pos: &mut Vec3, aabb: Aabb) {
    let Some(floor) = floor_height(world, pos.x, pos.z, aabb) else {
        return;
    };
    let target = floor + SKIN;
    if pos.y < target && (target - pos.y) < 0.08 {
        pos.y = target;
    }
}

fn floor_height(world: &mut VoxelWorld, x: f32, z: f32, aabb: Aabb) -> Option<f32> {
    let samples = [
        (0.0, 0.0),
        (aabb.half_x * 0.7, 0.0),
        (-aabb.half_x * 0.7, 0.0),
        (0.0, aabb.half_z * 0.7),
        (0.0, -aabb.half_z * 0.7),
    ];
    let mut best = None;
    for (dx, dz) in samples {
        let bx = (x + dx).floor() as i32;
        let bz = (z + dz).floor() as i32;
        for y in (0..WORLD_HEIGHT).rev() {
            if world.get_block(bx, y, bz).solid() {
                let top = y as f32 + 1.0;
                best = Some(best.map_or(top, |v: f32| v.max(top)));
                break;
            }
        }
    }
    best
}

fn block_min(min: [f32; 3]) -> [i32; 3] {
    [
        min[0].floor() as i32,
        min[1].floor() as i32,
        min[2].floor() as i32,
    ]
}

fn block_max(max: [f32; 3]) -> [i32; 3] {
    [
        (max[0] - EPS).floor() as i32,
        (max[1] - EPS).floor() as i32,
        (max[2] - EPS).floor() as i32,
    ]
}

fn overlap_1d(min_a: f32, max_a: f32, min_b: f32, max_b: f32) -> bool {
    max_a > min_b && min_a < max_b
}

fn overlaps(min: [f32; 3], max: [f32; 3], block_min: Vec3, block_max: Vec3) -> bool {
    max[0] > block_min.x
        && min[0] < block_max.x
        && max[1] > block_min.y
        && min[1] < block_max.y
        && max[2] > block_min.z
        && min[2] < block_max.z
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::BlockId;

    fn test_platform() -> VoxelWorld {
        let mut world = VoxelWorld::new(42);
        for x in -1..=2 {
            for z in -1..=2 {
                for y in 1..WORLD_HEIGHT {
                    world.set_block(x, y, z, BlockId::Air);
                }
                world.set_block(x, 0, z, BlockId::Stone);
            }
        }
        world
    }

    #[test]
    fn player_lands_on_floor() {
        let mut world = test_platform();
        let aabb = Aabb::from_uniform(0.3, 1.7);
        let mut pos = Vec3::new(0.5, 8.0, 0.5);
        let mut vel = Vec3::new(0.0, -10.0, 0.0);
        let mut landed = false;
        for _ in 0..40 {
            let result = move_aabb(&mut world, &mut pos, &mut vel, aabb, 0.05, false);
            vel.y = -10.0;
            if result.on_ground {
                landed = true;
                break;
            }
        }
        assert!(landed, "player should reach the ground");
        assert!((pos.y - 1.0).abs() < 0.1, "expected feet near y=1, got {}", pos.y);
        assert!(!overlaps_solid(&mut world, pos, aabb));
    }

    #[test]
    fn player_does_not_fall_through_floor() {
        let mut world = test_platform();
        let aabb = Aabb::from_uniform(0.3, 1.7);
        let mut pos = Vec3::new(0.5, 1.0, 0.5);
        let mut vel = Vec3::new(0.0, -30.0, 0.0);
        for _ in 0..20 {
            move_aabb(&mut world, &mut pos, &mut vel, aabb, 0.05, false);
            vel.y = -30.0;
        }
        assert!(pos.y >= 1.0 - 0.05);
        assert!(!overlaps_solid(&mut world, pos, aabb));
    }

    #[test]
    fn depenetration_pushes_player_out_of_block() {
        let mut world = test_platform();
        world.set_block(0, 1, 0, BlockId::Stone);
        let aabb = Aabb::from_uniform(0.3, 1.7);
        let mut pos = Vec3::new(0.5, 1.2, 0.5);
        ensure_clear(&mut world, &mut pos, aabb);
        assert!(!overlaps_solid(&mut world, pos, aabb));
        assert!(pos.y >= 2.0 - 0.1);
    }

    #[test]
    fn flat_walk_does_not_bounce() {
        let mut world = test_platform();
        let aabb = Aabb::from_uniform(0.3, 1.7);
        let mut pos = Vec3::new(0.5, 1.0, 0.5);
        let mut vel = Vec3::new(4.0, 0.0, 0.0);
        let start_y = pos.y;
        for _ in 0..60 {
            move_aabb(&mut world, &mut pos, &mut vel, aabb, 0.05, true);
            vel.x = 4.0;
            vel.y = 0.0;
            vel.z = 0.0;
        }
        assert!(
            (pos.y - start_y).abs() < 0.05,
            "walking on flat ground should not change height, y={}",
            pos.y
        );
    }

    #[test]
    fn horizontal_move_blocked_by_wall() {
        let mut world = test_platform();
        for y in 0..=2 {
            world.set_block(2, y, 0, BlockId::Stone);
        }
        let aabb = Aabb::from_uniform(0.3, 1.7);
        let mut pos = Vec3::new(0.5, 1.0, 0.5);
        let mut vel = Vec3::new(8.0, 0.0, 0.0);
        move_aabb(&mut world, &mut pos, &mut vel, aabb, 0.1, true);
        assert!(pos.x < 1.75, "player should be blocked by wall, x={}", pos.x);
    }
}
