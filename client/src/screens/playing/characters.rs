use crate::types::animated_model::{AnimatedModel, Clip};

use super::PlayingState;

/*
    characters + clips -> the anim_ models everything else already uses
    a character (mesh + skeleton) and the clips (just keyframes) load separately, in any order,
    and every clip gets put on the active character as soon as both are in
    switching character redoes every clip on the new body, so a skin is one file, not a full set of animations
    before this each anim_ file was a whole character with one animation baked in
*/
impl PlayingState {
    // the soldier everyone uses until skins are a real thing
    pub const DEFAULT_CHARACTER: &'static str = "character_ch15";

    // a character finished loading, if its the active one put every clip we already have on it
    pub fn add_character(&mut self, key: String, model: AnimatedModel) {
        model.prepare_gpu(); // upload the mesh once, every clip on this body shares it
        let active = key == self.active_character;
        self.characters.insert(key, model);
        if active {
            self.rebind_clips();
        }
    }

    // a clip finished loading, put it on the active character if that's in already
    pub fn add_clip(&mut self, key: String, clip: Clip) {
        if let Some(character) = self.characters.get(&self.active_character) {
            self.animated_models.insert(key.clone(), character.with_clip(&clip));
        }
        self.clips.insert(key, clip);
    }

    // wear a different character, every clip gets redone on that body, false if it isnt loaded
    pub fn set_character(&mut self, key: &str) -> bool {
        if !self.characters.contains_key(key) {
            return false;
        }
        self.active_character = key.to_string();
        self.rebind_clips();
        true
    }

    // every character in skin test order, default first then the proportion tests (ahmed's priority)
    pub const CHARACTER_ORDER: [&'static str; 6] =
        ["character_ch15", "character_pete", "character_vecna", "character_amy", "character_maw", "character_ty"];

    // the loaded characters in that order, anything else loaded after them
    pub fn character_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> =
            Self::CHARACTER_ORDER.iter().filter(|k| self.characters.contains_key(**k)).map(|k| k.to_string()).collect();
        let mut extra: Vec<String> = self.characters.keys().filter(|k| !keys.contains(k)).cloned().collect();
        extra.sort();
        keys.extend(extra);
        keys
    }

    // dev menu: wear the next character in order, after the last one it goes back to the first
    // before this it went alphabetically and only through whatever happened to be loaded
    pub fn next_character(&mut self) {
        let keys = self.character_keys();
        let now = keys.iter().position(|k| *k == self.active_character).unwrap_or(0);
        if let Some(next) = keys.get((now + 1) % keys.len().max(1)).cloned() {
            self.set_character(&next);
        }
    }

    // what the dev menu shows, like "2/6 pete", plus how many are still loading
    pub fn character_label(&self) -> String {
        let keys = self.character_keys();
        let at = keys.iter().position(|k| *k == self.active_character).map_or(0, |i| i + 1);
        let name = self.active_character.trim_start_matches("character_");
        let loading = Self::CHARACTER_ORDER.len().saturating_sub(keys.len());
        if loading > 0 {
            format!("{at}/{} {name} ({loading} still loading)", keys.len())
        } else {
            format!("{at}/{} {name}", keys.len())
        }
    }

    fn rebind_clips(&mut self) {
        let Some(character) = self.characters.get(&self.active_character) else {
            return;
        };
        for (key, clip) in &self.clips {
            self.animated_models.insert(key.clone(), character.with_clip(clip));
        }
    }
}
