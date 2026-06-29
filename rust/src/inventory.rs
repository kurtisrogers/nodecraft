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

    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a >= INVENTORY_SIZE || b >= INVENTORY_SIZE {
            return;
        }
        self.slots.swap(a, b);
    }

    pub fn item_display_name(item: u16) -> &'static str {
        match item {
            0 => "Empty",
            1 => "Grass",
            2 => "Dirt",
            3 => "Stone",
            4 => "Wood",
            5 => "Leaves",
            6 => "Sand",
            7 => "Water",
            8 => "Bedrock",
            9 => "Cobblestone",
            10 => "Planks",
            11 => "Glass",
            12 => "Snow",
            13 => "Crafting Table",
            14 => "Lava",
            15 => "Obsidian",
            16 => "Tall Grass",
            17 => "Flower",
            18 => "Wheat",
            _ => "Unknown",
        }
    }

    pub fn slot_label(&self, index: usize) -> String {
        let slot = &self.slots[index];
        if slot.item == 0 {
            return String::new();
        }
        format!("{} x{}", Self::item_display_name(slot.item), slot.count)
    }

    pub fn hotbar_item(&self) -> Option<u16> {
        let slot = &self.slots[self.hotbar_index];
        if slot.item == 0 {
            None
        } else {
            Some(slot.item)
        }
    }
}
