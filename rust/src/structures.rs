use crate::blocks::BlockId;
use crate::config::{SEA_LEVEL, WORLD_HEIGHT};
use crate::world::VoxelWorld;

pub fn place_lava_column(world: &mut VoxelWorld, wx: i32, wy: i32, wz: i32) {
    if wy < 1 || wy >= WORLD_HEIGHT - 1 {
        return;
    }
    let below = world.peek_block(wx, wy - 1, wz);
    if !below.solid() && below != BlockId::Lava {
        return;
    }
    if world.peek_block(wx, wy, wz) == BlockId::Air {
        world.set_block(wx, wy, wz, BlockId::Lava);
    }
}

pub fn place_volcano(world: &mut VoxelWorld, center_x: i32, center_z: i32) {
    const RADIUS: i32 = 28;
    const CRATER: i32 = 7;
    let peak_y = world.noise.terrain_height(center_x, center_z);
    let flow_angle = world.noise.roll(center_x, center_z, 71) * std::f32::consts::TAU;
    let flow_dx = flow_angle.cos();
    let flow_dz = flow_angle.sin();

    for dx in -RADIUS..=RADIUS {
        for dz in -RADIUS..=RADIUS {
            let wx = center_x + dx;
            let wz = center_z + dz;
            let dist = ((dx * dx + dz * dz) as f32).sqrt();
            if dist > RADIUS as f32 {
                continue;
            }

            let surface_y = world.noise.terrain_height(wx, wz);
            let in_crater = dist < CRATER as f32;

            if in_crater {
                for y in (surface_y - 5).max(1)..=surface_y {
                    world.set_block(wx, y, wz, BlockId::Air);
                }
                let floor = (surface_y - 3).max(world.noise.base_land_height(wx, wz) + 2);
                world.set_block(wx, floor - 1, wz, BlockId::Stone);
                for y in floor..=floor + 1 {
                    world.set_block(wx, y, wz, BlockId::Lava);
                }
                if dist > CRATER as f32 - 2.5 {
                    world.set_block(wx, floor - 1, wz, BlockId::Obsidian);
                }
            } else {
                let base = world.noise.base_land_height(wx, wz);
                for y in base..=surface_y {
                    let cur = world.peek_block(wx, y, wz);
                    if !matches!(cur, BlockId::Grass | BlockId::Dirt | BlockId::Stone | BlockId::Sand) {
                        continue;
                    }
                    let t = if surface_y > base {
                        (y - base) as f32 / (surface_y - base) as f32
                    } else {
                        1.0
                    };
                    let block = if t > 0.72 {
                        BlockId::Cobblestone
                    } else if t > 0.45 {
                        BlockId::Stone
                    } else {
                        continue;
                    };
                    world.set_block(wx, y, wz, block);
                }
            }
        }
    }

    place_lava_flow(world, center_x, center_z, peak_y, flow_dx, flow_dz);
}

fn place_lava_flow(
    world: &mut VoxelWorld,
    center_x: i32,
    center_z: i32,
    start_y: i32,
    dir_x: f32,
    dir_z: f32,
) {
    let mut x = center_x as f32;
    let mut z = center_z as f32;
    let mut last_y = start_y;

    for step in 0..16 {
        x += dir_x * 2.4;
        z += dir_z * 2.4;
        let wx = x.round() as i32;
        let wz = z.round() as i32;
        let y = world.noise.terrain_height(wx, wz);
        if y <= SEA_LEVEL + 2 {
            break;
        }
        if y > last_y + 3 {
            break;
        }
        last_y = y;
        place_lava_column(world, wx, y, wz);
        if step % 3 == 0 {
            place_lava_column(world, wx + dir_z.round() as i32, y, wz + dir_x.round() as i32);
        }
    }
}

pub fn place_house(world: &mut VoxelWorld, origin_x: i32, origin_y: i32, origin_z: i32) {
    let w = 7;
    let d = 7;
    let h = 4;

    for dy in 0..h {
        for dx in 0..w {
            for dz in 0..d {
                let wx = origin_x + dx;
                let wy = origin_y + dy;
                let wz = origin_z + dz;
                let edge_x = dx == 0 || dx == w - 1;
                let edge_z = dz == 0 || dz == d - 1;
                let is_wall = edge_x || edge_z;
                let is_roof = dy == h - 1;

                if !is_wall && !is_roof {
                    continue;
                }

                if is_roof {
                    world.set_block(wx, wy, wz, BlockId::Wood);
                    continue;
                }

                let door_x = dx == w / 2;
                let door_z = dz == 0;
                if door_z && door_x && dy < 2 {
                    continue;
                }

                let window_y = dy == 2;
                let window_x = edge_x && (dx == 1 || dx == w - 2);
                let window_z = edge_z && (dz == 1 || dz == d - 2);
                if window_y && (window_x || window_z) {
                    world.set_block(wx, wy, wz, BlockId::Glass);
                    continue;
                }

                world.set_block(wx, wy, wz, BlockId::Planks);
            }
        }
    }
}

pub fn place_farm_plot(world: &mut VoxelWorld, origin_x: i32, origin_y: i32, origin_z: i32, size: i32) {
    for dx in 0..size {
        for dz in 0..size {
            let wx = origin_x + dx;
            let wz = origin_z + dz;
            let edge = dx == 0 || dz == 0 || dx == size - 1 || dz == size - 1;
            if edge {
                world.set_block(wx, origin_y, wz, BlockId::Dirt);
            } else if (dx + dz) % 2 == 0 {
                world.set_block(wx, origin_y, wz, BlockId::Wheat);
            }
        }
    }
}

pub fn flatten_area(world: &mut VoxelWorld, center_x: i32, center_z: i32, radius: i32, target_y: i32) {
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let wx = center_x + dx;
            let wz = center_z + dz;
            let dist = ((dx * dx + dz * dz) as f32).sqrt();
            if dist > radius as f32 {
                continue;
            }

            let blend = 1.0 - (dist / radius as f32).powf(1.5);
            let local_target = (target_y as f32 * blend + world.noise.terrain_height(wx, wz) as f32 * (1.0 - blend))
                .round() as i32;

            for y in 0..WORLD_HEIGHT {
                if y < local_target {
                    let cur = world.peek_block(wx, y, wz);
                    if matches!(
                        cur,
                        BlockId::Air
                            | BlockId::Water
                            | BlockId::Leaves
                            | BlockId::TallGrass
                            | BlockId::Flower
                            | BlockId::Wheat
                    ) {
                        world.set_block(wx, y, wz, BlockId::Dirt);
                    } else if cur == BlockId::Stone {
                        world.set_block(wx, y, wz, BlockId::Dirt);
                    }
                } else if y == local_target {
                    let cur = world.peek_block(wx, y, wz);
                    if cur != BlockId::Water && cur != BlockId::Lava {
                        world.set_block(wx, y, wz, BlockId::Grass);
                    }
                } else if y > local_target {
                    let cur = world.peek_block(wx, y, wz);
                    if cur.solid() && cur != BlockId::Water && cur != BlockId::Lava {
                        world.set_block(wx, y, wz, BlockId::Air);
                    }
                }
            }
        }
    }
}

pub fn place_settlement(world: &mut VoxelWorld, center_x: i32, center_z: i32, surface_y: i32) {
    let pad_y = surface_y;
    flatten_area(world, center_x, center_z, 24, pad_y);
    place_house(world, center_x - 10, pad_y + 1, center_z - 6);
    place_house(world, center_x + 2, pad_y + 1, center_z - 8);
    place_house(world, center_x - 4, pad_y + 1, center_z + 4);
    place_farm_plot(world, center_x + 6, pad_y + 1, center_z + 2, 5);
    place_farm_plot(world, center_x - 14, pad_y + 1, center_z + 4, 4);
}
