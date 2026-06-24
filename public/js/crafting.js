import { BlockId } from './blocks.js';
import { ItemId } from './items.js';

export const CRAFT_GRID_SIZE = 9;

export const RECIPES = [
  {
    id: 'planks',
    name: 'Planks',
    grid: [ItemId.WOOD, 0, 0, 0, 0, 0, 0, 0, 0],
    result: { itemId: ItemId.PLANKS, count: 4 },
  },
  {
    id: 'sticks',
    name: 'Sticks',
    grid: [ItemId.PLANKS, 0, 0, ItemId.PLANKS, 0, 0, 0, 0, 0],
    result: { itemId: ItemId.STICK, count: 4 },
  },
  {
    id: 'crafting_table',
    name: 'Crafting Table',
    grid: [ItemId.PLANKS, ItemId.PLANKS, 0, ItemId.PLANKS, ItemId.PLANKS, 0, 0, 0, 0],
    result: { itemId: ItemId.CRAFTING_TABLE, count: 1 },
  },
  {
    id: 'glass',
    name: 'Glass',
    grid: [ItemId.SAND, ItemId.SAND, 0, ItemId.SAND, ItemId.SAND, 0, 0, 0, 0],
    result: { itemId: ItemId.GLASS, count: 4 },
  },
  {
    id: 'cobblestone_wall',
    name: 'Cobblestone',
    grid: [ItemId.STONE, 0, 0, 0, 0, 0, 0, 0, 0],
    result: { itemId: ItemId.COBBLESTONE, count: 1 },
  },
];

function normalizeGrid(grid) {
  const g = [...grid];
  while (g.length < 9) g.push(0);

  let minR = 3, maxR = -1, minC = 3, maxC = -1;
  for (let r = 0; r < 3; r++) {
    for (let c = 0; c < 3; c++) {
      if (g[r * 3 + c]) {
        minR = Math.min(minR, r);
        maxR = Math.max(maxR, r);
        minC = Math.min(minC, c);
        maxC = Math.max(maxC, c);
      }
    }
  }
  if (maxR < 0) return [];

  const result = [];
  for (let r = minR; r <= maxR; r++) {
    for (let c = minC; c <= maxC; c++) {
      result.push(g[r * 3 + c] || 0);
    }
  }
  return result;
}

function gridsMatch(a, b) {
  const na = normalizeGrid(a);
  const nb = normalizeGrid(b);
  if (na.length !== nb.length) return false;
  for (let i = 0; i < na.length; i++) {
    if (na[i] !== nb[i]) return false;
  }
  return na.length > 0;
}

export function matchRecipe(grid) {
  for (const recipe of RECIPES) {
    if (gridsMatch(grid, recipe.grid)) return recipe;
  }
  return null;
}

export function getRecipeIngredients(recipe) {
  const counts = new Map();
  for (const itemId of recipe.grid) {
    if (!itemId) continue;
    counts.set(itemId, (counts.get(itemId) ?? 0) + 1);
  }
  return counts;
}

export function canCraft(recipe, inventory) {
  const ingredients = getRecipeIngredients(recipe);
  for (const [itemId, count] of ingredients) {
    if (!inventory.hasItem(itemId, count)) return false;
  }
  return true;
}

export function craft(recipe, inventory) {
  if (!canCraft(recipe, inventory)) return false;
  const ingredients = getRecipeIngredients(recipe);
  for (const [itemId, count] of ingredients) {
    inventory.removeItem(itemId, count);
  }
  inventory.addItem(recipe.result.itemId, recipe.result.count);
  return true;
}

export function canCraftDirectly(recipeId, inventory) {
  const recipe = RECIPES.find((r) => r.id === recipeId);
  return recipe ? canCraft(recipe, inventory) : false;
}

export function craftDirectly(recipeId, inventory) {
  const recipe = RECIPES.find((r) => r.id === recipeId);
  return recipe ? craft(recipe, inventory) : false;
}
