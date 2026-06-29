//! Voxel structure blueprints — procedural stamps defined in code (no external model files).

use crate::blocks::BlockId;
use crate::world::VoxelWorld;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureKind {
    Cottage,
    Barn,
    FarmPatch,
}

#[derive(Clone, Copy, Debug)]
pub struct StructureBlueprint {
    pub kind: StructureKind,
    pub name: &'static str,
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

pub const COTTAGE: StructureBlueprint = StructureBlueprint {
    kind: StructureKind::Cottage,
    name: "cottage",
    width: 7,
    depth: 7,
    height: 4,
};

pub const BARN: StructureBlueprint = StructureBlueprint {
    kind: StructureKind::Barn,
    name: "barn",
    width: 9,
    depth: 11,
    height: 5,
};

pub const FARM_PATCH: StructureBlueprint = StructureBlueprint {
    kind: StructureKind::FarmPatch,
    name: "farm_patch",
    width: 6,
    depth: 6,
    height: 1,
};

pub fn stamp_cottage(world: &mut VoxelWorld, origin_x: i32, origin_y: i32, origin_z: i32) {
    stamp_box_building(
        world,
        origin_x,
        origin_y,
        origin_z,
        COTTAGE.width,
        COTTAGE.depth,
        COTTAGE.height,
        BlockId::Planks,
        BlockId::Wood,
        true,
    );
}

pub fn stamp_barn(world: &mut VoxelWorld, origin_x: i32, origin_y: i32, origin_z: i32) {
    stamp_box_building(
        world,
        origin_x,
        origin_y,
        origin_z,
        BARN.width,
        BARN.depth,
        BARN.height,
        BlockId::Cobblestone,
        BlockId::Wood,
        false,
    );
    // Hayloft opening
    let mid_x = origin_x + BARN.width / 2;
    let mid_z = origin_z;
    for dy in 1..3 {
        world.set_block(mid_x, origin_y + dy, mid_z, BlockId::Air);
    }
}

pub fn stamp_farm_patch(world: &mut VoxelWorld, origin_x: i32, origin_y: i32, origin_z: i32, size: i32) {
    for dx in 0..size {
        for dz in 0..size {
            let wx = origin_x + dx;
            let wz = origin_z + dz;
            let edge = dx == 0 || dz == 0 || dx == size - 1 || dz == size - 1;
            if edge {
                world.set_block(wx, origin_y, wz, BlockId::Dirt);
            } else {
                world.set_block(wx, origin_y, wz, BlockId::Dirt);
            }
        }
    }
}

fn stamp_box_building(
    world: &mut VoxelWorld,
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    w: i32,
    d: i32,
    h: i32,
    wall: BlockId,
    roof: BlockId,
    cottage_windows: bool,
) {
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
                    world.set_block(wx, wy, wz, roof);
                    continue;
                }

                let door_x = dx == w / 2;
                let door_z = dz == 0;
                if door_z && door_x && dy < 2 {
                    continue;
                }

                if cottage_windows {
                    let window_y = dy == 2;
                    let window_x = edge_x && (dx == 1 || dx == w - 2);
                    let window_z = edge_z && (dz == 1 || dz == d - 2);
                    if window_y && (window_x || window_z) {
                        world.set_block(wx, wy, wz, BlockId::Glass);
                        continue;
                    }
                }

                world.set_block(wx, wy, wz, wall);
            }
        }
    }
}
