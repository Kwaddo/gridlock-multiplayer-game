/*
    This module is cause we made so many weapons lol so we need an efficient way to manage
    them. It also contains the player's loadout, which is the list of weapons they have 
    equipped in their 5 inventory slots. The loadout is sent to the server on spawn and 
    whenever the player changes it in the lobby.
*/

use super::stats::WeaponId;

pub const LOADOUT_SLOTS: usize = 5;

pub struct Loadout {
    pub slots: [WeaponId; LOADOUT_SLOTS],
}

impl Loadout {
    pub fn default_loadout() -> Self {
        Loadout {
            slots: [
                WeaponId::M1911,
                WeaponId::Mp5,
                WeaponId::Spas,
                WeaponId::M16,
                WeaponId::BoltAction,
            ],
        }
    }

    // Assign a weapon to a slot. If it already exists in another slot, swap them.
    pub fn assign(&mut self, slot: usize, weapon: WeaponId) {
        if slot >= LOADOUT_SLOTS {
            return;
        }
        // check if weapon is already in another slot — swap
        if let Some(existing_slot) = self.slots.iter().position(|&w| w == weapon) {
            self.slots[existing_slot] = self.slots[slot];
        }
        self.slots[slot] = weapon;
    }
}
