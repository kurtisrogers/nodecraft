use crate::blocks::BlockId;
use crate::config::WORLD_HEIGHT;
use crate::world::VoxelWorld;

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

            for y in 0..WORLD_HEIGHT {
                if y < target_y {
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
                    }
                } else if y == target_y {
                    let cur = world.peek_block(wx, y, wz);
                    if cur != BlockId::Water && cur != BlockId::Lava {
                        world.set_block(wx, y, wz, BlockId::Grass);
                    }
                } else {
                    let cur = world.peek_block(wx, y, wz);
                    if cur != BlockId::Air && cur != BlockId::Water && cur != BlockId::Lava {
                        world.set_block(wx, y, wz, BlockId::Air);
                    }
                }
            }
        }
    }
}

pub fn place_settlement(world: &mut VoxelWorld, center_x: i32, center_z: i32, surface_y: i32) {
    flatten_area(world, center_x, center_z, 22, surface_y);
    place_house(world, center_x - 10, surface_y + 1, center_z - 6);
    place_house(world, center_x + 2, surface_y + 1, center_z - 8);
    place_house(world, center_x - 4, surface_y + 1, center_z + 4);
    place_farm_plot(world, center_x + 6, surface_y + 1, center_z + 2, 5);
    place_farm_plot(world, center_x - 14, surface_y + 1, center_z + 4, 4);
}
