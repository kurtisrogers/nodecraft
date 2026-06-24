import { getItemColor, getItemName } from './items.js';
import { RECIPES, canCraftDirectly, craftDirectly } from './crafting.js';
import { HOTBAR_SIZE, TOTAL_SLOTS } from './inventory.js';

export class GameUI {
  constructor(game) {
    this.game = game;
    this.hotbarEl = document.getElementById('hotbar');
    this.inventoryPanel = document.getElementById('inventory-panel');
    this.craftingPanel = document.getElementById('crafting-panel');
    this.recipeList = document.getElementById('recipe-list');
    this.playerCountEl = document.getElementById('player-count');
    this.timeEl = document.getElementById('time-display');
    this.open = false;
    this.buildRecipeList();
  }

  buildRecipeList() {
    if (!this.recipeList) return;
    this.recipeList.innerHTML = '';
    for (const recipe of RECIPES) {
      const btn = document.createElement('button');
      btn.className = 'recipe-btn';
      btn.textContent = `${recipe.name} → ${recipe.result.count}x`;
      btn.dataset.recipeId = recipe.id;
      btn.addEventListener('click', () => this.craftRecipe(recipe.id));
      this.recipeList.appendChild(btn);
    }
  }

  craftRecipe(recipeId) {
    const { inventory } = this.game.player;
    if (craftDirectly(recipeId, inventory)) {
      this.refreshHotbar();
      this.refreshInventory();
      this.updateRecipeButtons();
    }
  }

  updateRecipeButtons() {
    if (!this.recipeList) return;
    const { inventory } = this.game.player;
    this.recipeList.querySelectorAll('.recipe-btn').forEach((btn) => {
      const can = canCraftDirectly(btn.dataset.recipeId, inventory);
      btn.disabled = !can;
      btn.classList.toggle('available', can);
    });
  }

  toggleInventory() {
    this.open = !this.open;
    this.inventoryPanel?.classList.toggle('open', this.open);
    this.craftingPanel?.classList.toggle('open', this.open);
    if (this.open) {
      if (!document.body.classList.contains('mobile')) {
        document.exitPointerLock();
      }
      this.refreshInventory();
      this.updateRecipeButtons();
    }
  }

  closeInventory() {
    this.open = false;
    this.inventoryPanel?.classList.remove('open');
    this.craftingPanel?.classList.remove('open');
  }

  isOpen() {
    return this.open;
  }

  getItemStyle(itemId) {
    const color = getItemColor(itemId);
    return `#${color.toString(16).padStart(6, '0')}`;
  }

  renderSlot(slot, index, isHotbar = false) {
    const el = document.createElement('div');
    el.className = `inv-slot${isHotbar ? ' hotbar-slot' : ''}${index === this.game.player.hotbarIndex && isHotbar ? ' selected' : ''}`;
    el.dataset.index = index;

    if (slot) {
      el.innerHTML = `
        <div class="block-preview" style="background:${this.getItemStyle(slot.itemId)}" title="${getItemName(slot.itemId)}"></div>
        <span class="slot-count">${slot.count > 1 ? slot.count : ''}</span>
      `;
    }

  if (isHotbar) {
      el.addEventListener('click', () => {
        this.game.player.hotbarIndex = index;
        this.game.player.updateSelectedBlock();
        this.refreshHotbar();
      });
    }

    return el;
  }

  refreshHotbar() {
    const { inventory, hotbarIndex } = this.game.player;
    this.hotbarEl.innerHTML = '';
    for (let i = 0; i < HOTBAR_SIZE; i++) {
      const slot = inventory.getSlot(i);
      const el = this.renderSlot(slot, i, true);
      if (i === hotbarIndex) el.classList.add('selected');
      const num = document.createElement('span');
      num.className = 'slot-number';
      num.textContent = i + 1;
      el.appendChild(num);
      this.hotbarEl.appendChild(el);
    }
  }

  refreshInventory() {
    const grid = document.getElementById('inventory-grid');
    if (!grid) return;
    grid.innerHTML = '';
    const { inventory } = this.game.player;
    for (let i = HOTBAR_SIZE; i < TOTAL_SLOTS; i++) {
      grid.appendChild(this.renderSlot(inventory.getSlot(i), i));
    }
  }

  setPlayerCount(count, label) {
    if (this.playerCountEl) {
      if (label) {
        this.playerCountEl.textContent = label;
      } else {
        this.playerCountEl.textContent = `${count} player${count !== 1 ? 's' : ''} online`;
      }
    }
  }

  setTimeOfDay(dayTime, isNight) {
    if (this.timeEl) {
      this.timeEl.textContent = isNight ? 'Night' : 'Day';
      this.timeEl.classList.toggle('night', isNight);
    }
    if (this.game.scene) {
      const cycle = (dayTime % 120) / 120;
      const skyColor = isNight ? 0x0a0a20 : 0x87ceeb;
      this.game.renderer.setClearColor(skyColor);
      this.game.scene.fog.color.setHex(skyColor);
    }
  }
}
