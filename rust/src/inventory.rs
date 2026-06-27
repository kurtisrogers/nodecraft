use crate::blocks::BlockId;
use crate::config::{INVENTORY_SIZE, MAX_STACK};
use bevy::prelude::Resource;

#[derive(Clone, Copy)]
pub struct Slot {
    pub item: u16,
    pub count: u32,
}

impl Default for Slot {
    fn default() -> Self {
        Self { item: 0, count: 0 }
    }
}

#[derive(Resource)]
pub struct GameInventory {
    pub slots: [Slot; INVENTORY_SIZE],
    pub hotbar_index: usize,
}

impl Default for GameInventory {
    fn default() -> Self {
        Self {
            slots: [Slot::default(); INVENTORY_SIZE],
            hotbar_index: 0,
        }
    }
}

impl GameInventory {
    pub fn with_starter_items() -> Self {
        let mut inv = Self::default();
        inv.add_item(BlockId::Dirt as u16, 16);
        inv.add_item(BlockId::Cobblestone as u16, 16);
        inv.add_item(BlockId::Wood as u16, 8);
        inv
    }

    pub fn add_item(&mut self, item: u16, count: u32) -> bool {
        let mut remaining = count;
        for slot in &mut self.slots {
            if slot.item == item && slot.count < MAX_STACK {
                let space = MAX_STACK - slot.count;
                let add = remaining.min(space);
                slot.count += add;
                remaining -= add;
                if remaining == 0 {
                    return true;
                }
            }
        }
        for slot in &mut self.slots {
            if slot.item == 0 {
                let add = remaining.min(MAX_STACK);
                slot.item = item;
                slot.count = add;
                remaining -= add;
                if remaining == 0 {
                    return true;
                }
            }
        }
        remaining == 0
    }

    pub fn remove_item(&mut self, item: u16, count: u32) -> bool {
        if !self.has_item(item, count) {
            return false;
        }
        let mut remaining = count;
        for slot in &mut self.slots {
            if slot.item == item {
                let take = remaining.min(slot.count);
                slot.count -= take;
                remaining -= take;
                if slot.count == 0 {
                    slot.item = 0;
                }
                if remaining == 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_item(&self, item: u16, count: u32) -> bool {
        let total: u32 = self
            .slots
            .iter()
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum();
        total >= count
    }

    pub fn hotbar_item(&self) -> Option<u16> {
        let slot = &self.slots[self.hotbar_index];
        if slot.item == 0 { None } else { Some(slot.item) }
    }
}
