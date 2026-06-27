export const BlockId = {
  AIR: 0,
  GRASS: 1,
  DIRT: 2,
  STONE: 3,
  WOOD: 4,
  LEAVES: 5,
  SAND: 6,
  WATER: 7,
  BEDROCK: 8,
  COBBLESTONE: 9,
  PLANKS: 10,
  GLASS: 11,
  SNOW: 12,
  CRAFTING_TABLE: 13,
  LAVA: 14,
  OBSIDIAN: 15,
  TALL_GRASS: 16,
  FLOWER: 17,
  WHEAT: 18,
};

export const BLOCKS = {
  [BlockId.AIR]: {
    id: BlockId.AIR,
    name: 'Air',
    solid: false,
    transparent: true,
    color: null,
  },
  [BlockId.GRASS]: {
    id: BlockId.GRASS,
    name: 'Grass',
    solid: true,
    transparent: false,
    color: { top: 0x5a9e3a, side: 0x8b6914, bottom: 0x8b6914 },
  },
  [BlockId.DIRT]: {
    id: BlockId.DIRT,
    name: 'Dirt',
    solid: true,
    transparent: false,
    color: 0x8b6914,
  },
  [BlockId.STONE]: {
    id: BlockId.STONE,
    name: 'Stone',
    solid: true,
    transparent: false,
    color: 0x888888,
  },
  [BlockId.WOOD]: {
    id: BlockId.WOOD,
    name: 'Wood',
    solid: true,
    transparent: false,
    color: { top: 0xc4a35a, side: 0x6b4423, bottom: 0xc4a35a },
  },
  [BlockId.LEAVES]: {
    id: BlockId.LEAVES,
    name: 'Leaves',
    solid: true,
    transparent: true,
    color: 0x2d6b1e,
  },
  [BlockId.SAND]: {
    id: BlockId.SAND,
    name: 'Sand',
    solid: true,
    transparent: false,
    color: 0xe8d5a3,
  },
  [BlockId.WATER]: {
    id: BlockId.WATER,
    name: 'Water',
    solid: false,
    transparent: true,
    color: 0x3366cc,
    opacity: 0.6,
  },
  [BlockId.BEDROCK]: {
    id: BlockId.BEDROCK,
    name: 'Bedrock',
    solid: true,
    transparent: false,
    color: 0x333333,
  },
  [BlockId.COBBLESTONE]: {
    id: BlockId.COBBLESTONE,
    name: 'Cobblestone',
    solid: true,
    transparent: false,
    color: 0x6b6b6b,
  },
  [BlockId.PLANKS]: {
    id: BlockId.PLANKS,
    name: 'Planks',
    solid: true,
    transparent: false,
    color: 0xc4a35a,
  },
  [BlockId.GLASS]: {
    id: BlockId.GLASS,
    name: 'Glass',
    solid: true,
    transparent: true,
    color: 0xc8e8ff,
    opacity: 0.3,
  },
  [BlockId.SNOW]: {
    id: BlockId.SNOW,
    name: 'Snow',
    solid: true,
    transparent: false,
    color: 0xffffff,
  },
  [BlockId.CRAFTING_TABLE]: {
    id: BlockId.CRAFTING_TABLE,
    name: 'Crafting Table',
    solid: true,
    transparent: false,
    color: { top: 0xc4a35a, side: 0x8b6914, bottom: 0x6b4423 },
  },
  [BlockId.LAVA]: {
    id: BlockId.LAVA,
    name: 'Lava',
    solid: false,
    transparent: true,
    color: 0xff5500,
    emissive: true,
    damages: true,
  },
  [BlockId.OBSIDIAN]: {
    id: BlockId.OBSIDIAN,
    name: 'Obsidian',
    solid: true,
    transparent: false,
    color: 0x1a0a2e,
  },
  [BlockId.TALL_GRASS]: {
    id: BlockId.TALL_GRASS,
    name: 'Tall Grass',
    solid: false,
    transparent: true,
    color: 0x4a9e32,
  },
  [BlockId.FLOWER]: {
    id: BlockId.FLOWER,
    name: 'Flower',
    solid: false,
    transparent: true,
    color: { top: 0xffdd55, side: 0x3d8a2f, bottom: 0x3d8a2f },
  },
  [BlockId.WHEAT]: {
    id: BlockId.WHEAT,
    name: 'Wheat',
    solid: false,
    transparent: true,
    color: { top: 0xd4b84a, side: 0xc4a035, bottom: 0x8b6914 },
  },
};

export const HOTBAR_BLOCKS = [
  BlockId.GRASS,
  BlockId.DIRT,
  BlockId.STONE,
  BlockId.COBBLESTONE,
  BlockId.WOOD,
  BlockId.PLANKS,
  BlockId.LEAVES,
  BlockId.OBSIDIAN,
  BlockId.GLASS,
];

export function isSolid(blockId) {
  return BLOCKS[blockId]?.solid ?? false;
}

export function isTransparent(blockId) {
  return BLOCKS[blockId]?.transparent ?? true;
}

export function isLava(blockId) {
  return blockId === BlockId.LAVA;
}

export function getBlockColor(blockId, face) {
  const block = BLOCKS[blockId];
  if (!block?.color) return 0xffffff;
  if (typeof block.color === 'number') return block.color;
  if (face === 'top') return block.color.top;
  if (face === 'bottom') return block.color.bottom;
  return block.color.side;
}
