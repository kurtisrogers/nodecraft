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
};

export const HOTBAR_BLOCKS = [
  BlockId.GRASS,
  BlockId.DIRT,
  BlockId.STONE,
  BlockId.COBBLESTONE,
  BlockId.WOOD,
  BlockId.PLANKS,
  BlockId.LEAVES,
  BlockId.SAND,
  BlockId.GLASS,
];

export function isSolid(blockId) {
  return BLOCKS[blockId]?.solid ?? false;
}

export function isTransparent(blockId) {
  return BLOCKS[blockId]?.transparent ?? true;
}

export function getBlockColor(blockId, face) {
  const block = BLOCKS[blockId];
  if (!block?.color) return 0xffffff;
  if (typeof block.color === 'number') return block.color;
  if (face === 'top') return block.color.top;
  if (face === 'bottom') return block.color.bottom;
  return block.color.side;
}
