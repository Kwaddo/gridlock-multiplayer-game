/*
    This module is cause we made so many weapons lol so we need an efficient way to manage
    them. It also contains the player's loadout, which is the list of weapons they have
    equipped in their 2 inventory slots:
      - Slot 0: Primary weapon (Shotgun, Assault Rifle, or Sniper)
      - Slot 1: Secondary weapon (Pistol)
    The loadout is sent to the server on spawn and whenever the player changes it in the lobby.
*/

use super::stats::{WeaponCategory, WeaponId};

pub const LOADOUT_SLOTS: usize = 2;

pub struct Loadout {
    pub slots: [WeaponId; LOADOUT_SLOTS],
}

impl Loadout {
    pub fn default_loadout() -> Self {
        Loadout {
            slots: [
                WeaponId::M16,   // slot 0: primary (assault rifle)
                WeaponId::M1911, // slot 1: secondary (pistol)
            ],
        }
    }

    /// Returns whether a weapon is valid for a given slot.
    /// Slot 0 (primary): Shotgun, AssaultRifle, or Sniper.
    /// Slot 1 (secondary): Pistol only.
    pub fn slot_allows(slot: usize, weapon: WeaponId) -> bool {
        let cat = weapon.category();
        match slot {
            0 => matches!(cat, WeaponCategory::Shotgun | WeaponCategory::AssaultRifle | WeaponCategory::Sniper | WeaponCategory::Smg),
            1 => matches!(cat, WeaponCategory::Pistol),
            _ => false,
        }
    }

    // Assign a weapon to a slot. Ignores invalid weapon/slot combinations.
    // If the weapon is already in another slot, swaps them.
    pub fn assign(&mut self, slot: usize, weapon: WeaponId) {
        if slot >= LOADOUT_SLOTS {
            return;
        }
        if !Self::slot_allows(slot, weapon) {
            return;
        }
        // check if weapon is already in another slot — swap
        if let Some(existing_slot) = self.slots.iter().position(|&w| w == weapon) {
            self.slots[existing_slot] = self.slots[slot];
        }
        self.slots[slot] = weapon;
    }
}
