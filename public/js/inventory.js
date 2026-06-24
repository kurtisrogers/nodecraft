import { ItemId } from './items.js';

export const HOTBAR_SIZE = 9;
export const INVENTORY_SIZE = 27;
export const TOTAL_SLOTS = HOTBAR_SIZE + INVENTORY_SIZE;

export class Inventory {
  constructor() {
    this.slots = Array(TOTAL_SLOTS).fill(null);
  }

  getSlot(index) {
    return this.slots[index] ?? null;
  }

  setSlot(index, stack) {
    this.slots[index] = stack;
  }

  findSlotWithItem(itemId) {
    for (let i = 0; i < TOTAL_SLOTS; i++) {
      const slot = this.slots[i];
      if (slot && slot.itemId === itemId && slot.count < 64) return i;
    }
    return -1;
  }

  findEmptySlot() {
    for (let i = 0; i < TOTAL_SLOTS; i++) {
      if (!this.slots[i]) return i;
    }
    return -1;
  }

  addItem(itemId, count = 1) {
    let remaining = count;
    while (remaining > 0) {
      const existing = this.findSlotWithItem(itemId);
      if (existing >= 0) {
        const slot = this.slots[existing];
        const space = 64 - slot.count;
        const add = Math.min(space, remaining);
        slot.count += add;
        remaining -= add;
      } else {
        const empty = this.findEmptySlot();
        if (empty < 0) return count - remaining;
        const add = Math.min(64, remaining);
        this.slots[empty] = { itemId, count: add };
        remaining -= add;
      }
    }
    return count;
  }

  removeItem(itemId, count = 1) {
    let remaining = count;
    for (let i = TOTAL_SLOTS - 1; i >= 0 && remaining > 0; i--) {
      const slot = this.slots[i];
      if (!slot || slot.itemId !== itemId) continue;
      const remove = Math.min(slot.count, remaining);
      slot.count -= remove;
      remaining -= remove;
      if (slot.count <= 0) this.slots[i] = null;
    }
    return remaining === 0;
  }

  countItem(itemId) {
    return this.slots.reduce((sum, slot) => {
      if (slot?.itemId === itemId) return sum + slot.count;
      return sum;
    }, 0);
  }

  hasItem(itemId, count = 1) {
    return this.countItem(itemId) >= count;
  }

  getHotbarItem(index) {
    return this.slots[index]?.itemId ?? null;
  }

  getHotbarCount(index) {
    return this.slots[index]?.count ?? 0;
  }

  toJSON() {
    return this.slots;
  }

  fromJSON(data) {
    this.slots = data.map((s) => (s ? { ...s } : null));
    while (this.slots.length < TOTAL_SLOTS) this.slots.push(null);
  }
}
