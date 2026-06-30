use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlockId {
    #[default]
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
    Wood = 4,
    Leaves = 5,
    Sand = 6,
    Water = 7,
    Bedrock = 8,
    Cobblestone = 9,
    Planks = 10,
    Glass = 11,
    Snow = 12,
    CraftingTable = 13,
    Lava = 14,
    Obsidian = 15,
    TallGrass = 16,
    Flower = 17,
    Wheat = 18,
}

impl BlockId {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Grass,
            2 => Self::Dirt,
            3 => Self::Stone,
            4 => Self::Wood,
            5 => Self::Leaves,
            6 => Self::Sand,
            7 => Self::Water,
            8 => Self::Bedrock,
            9 => Self::Cobblestone,
            10 => Self::Planks,
            11 => Self::Glass,
            12 => Self::Snow,
            13 => Self::CraftingTable,
            14 => Self::Lava,
            15 => Self::Obsidian,
            16 => Self::TallGrass,
            17 => Self::Flower,
            18 => Self::Wheat,
            _ => Self::Air,
        }
    }

    pub fn is_cross_decoration(self) -> bool {
        matches!(self, Self::TallGrass | Self::Flower | Self::Wheat)
    }

    /// Blocks that impede player movement (full cubes only).
    pub fn blocks_collision(self) -> bool {
        match self {
            Self::TallGrass | Self::Flower | Self::Wheat | Self::Leaves | Self::Glass => false,
            _ => self.solid(),
        }
    }

    pub fn solid(self) -> bool {
        matches!(
            self,
            Self::Grass
                | Self::Dirt
                | Self::Stone
                | Self::Wood
                | Self::Leaves
                | Self::Sand
                | Self::Bedrock
                | Self::Cobblestone
                | Self::Planks
                | Self::Glass
                | Self::Snow
                | Self::CraftingTable
                | Self::Obsidian
        )
    }

    pub fn transparent(self) -> bool {
        !self.solid() || matches!(self, Self::Leaves | Self::Glass | Self::Water | Self::Lava)
    }

    pub fn color(self, face: Face) -> Color {
        let rgb = match self {
            Self::Air => return Color::NONE,
            Self::Grass => match face {
                Face::Top => [0.35, 0.62, 0.24],
                Face::Bottom => [0.55, 0.41, 0.08],
                Face::Side => [0.55, 0.41, 0.08],
            },
            Self::Dirt => [0.55, 0.41, 0.08],
            Self::Stone => [0.53, 0.53, 0.53],
            Self::Wood => match face {
                Face::Top | Face::Bottom => [0.77, 0.64, 0.35],
                Face::Side => [0.42, 0.27, 0.14],
            },
            Self::Leaves => [0.18, 0.42, 0.12],
            Self::Sand => [0.91, 0.84, 0.64],
            Self::Water => [0.20, 0.40, 0.80],
            Self::Bedrock => [0.20, 0.20, 0.20],
            Self::Cobblestone => [0.42, 0.42, 0.42],
            Self::Planks => [0.77, 0.64, 0.35],
            Self::Glass => [0.78, 0.91, 1.0],
            Self::Snow => [1.0, 1.0, 1.0],
            Self::CraftingTable => match face {
                Face::Top => [0.77, 0.64, 0.35],
                Face::Bottom => [0.42, 0.27, 0.14],
                Face::Side => [0.55, 0.41, 0.08],
            },
            Self::Lava => [1.0, 0.33, 0.0],
            Self::Obsidian => [0.10, 0.04, 0.18],
            Self::TallGrass => [0.29, 0.62, 0.20],
            Self::Flower => match face {
                Face::Top => [1.0, 0.87, 0.33],
                Face::Bottom | Face::Side => [0.24, 0.54, 0.18],
            },
            Self::Wheat => match face {
                Face::Top => [0.83, 0.72, 0.29],
                Face::Bottom | Face::Side => [0.77, 0.63, 0.21],
            },
        };
        let shade = match face {
            Face::Top => 1.0,
            Face::Bottom => 0.6,
            Face::Side => 0.8,
        };
        let emissive = self == Self::Lava;
        let scale = if emissive { 1.5 } else { 1.0 } * shade;
        Color::srgb(rgb[0] * scale, rgb[1] * scale, rgb[2] * scale)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    Side,
}

pub fn block_drop(id: BlockId) -> Option<BlockId> {
    match id {
        BlockId::Grass => Some(BlockId::Dirt),
        BlockId::Stone => Some(BlockId::Cobblestone),
        BlockId::Leaves | BlockId::TallGrass | BlockId::Flower | BlockId::Wheat => None,
        BlockId::Air | BlockId::Water | BlockId::Lava | BlockId::Bedrock => None,
        other => Some(other),
    }
}

pub fn is_block_item(id: u16) -> bool {
    (1..=99).contains(&id)
}
