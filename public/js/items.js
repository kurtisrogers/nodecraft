import { BLOCKS } from './blocks.js';

export const ItemId = {
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
  STICK: 100,
  RAW_PORK: 101,
  LEATHER: 102,
  ROTTEN_FLESH: 103,
  BEEF: 104,
};

export const ITEMS = {
  [ItemId.STICK]: { id: ItemId.STICK, name: 'Stick', stackable: true, maxStack: 64, placeable: false, color: 0x8b6914 },
  [ItemId.RAW_PORK]: { id: ItemId.RAW_PORK, name: 'Raw Pork', stackable: true, maxStack: 64, placeable: false, color: 0xffb6c1 },
  [ItemId.LEATHER]: { id: ItemId.LEATHER, name: 'Leather', stackable: true, maxStack: 64, placeable: false, color: 0x8b4513 },
  [ItemId.ROTTEN_FLESH]: { id: ItemId.ROTTEN_FLESH, name: 'Rotten Flesh', stackable: true, maxStack: 64, placeable: false, color: 0x5a4a3a },
  [ItemId.BEEF]: { id: ItemId.BEEF, name: 'Raw Beef', stackable: true, maxStack: 64, placeable: false, color: 0xcc4444 },
};

export const BLOCK_DROPS = {
  [ItemId.GRASS]: ItemId.DIRT,
  [ItemId.DIRT]: ItemId.DIRT,
  [ItemId.STONE]: ItemId.COBBLESTONE,
  [ItemId.WOOD]: ItemId.WOOD,
  [ItemId.LEAVES]: null,
  [ItemId.SAND]: ItemId.SAND,
  [ItemId.COBBLESTONE]: ItemId.COBBLESTONE,
  [ItemId.PLANKS]: ItemId.PLANKS,
  [ItemId.GLASS]: ItemId.GLASS,
  [ItemId.SNOW]: ItemId.SNOW,
  [ItemId.CRAFTING_TABLE]: ItemId.CRAFTING_TABLE,
};

export function isBlockItem(itemId) {
  return itemId >= 1 && itemId <= 99;
}

export function getItemName(itemId) {
  if (ITEMS[itemId]) return ITEMS[itemId].name;
  if (isBlockItem(itemId)) return BLOCKS[itemId]?.name ?? 'Unknown';
  return 'Unknown';
}

export function getItemColor(itemId) {
  if (ITEMS[itemId]) return ITEMS[itemId].color;
  if (isBlockItem(itemId)) {
    const block = BLOCKS[itemId];
    if (!block?.color) return 0xffffff;
    return typeof block.color === 'number' ? block.color : block.color.side;
  }
  return 0xffffff;
}
