use serde::{ Serialize, Deserialize };
use std::path::PathBuf;
use super::crosshair::{ CrosshairConfig, CrosshairType };
use super::stats::WeaponKind;
use macroquad::prelude::*;

/*
    This module is for storing user created crosshairs,
    they will be saved by weapon kind and username in a json file and loaded/applied the same way
    a user selects a crosshair preset for a weapon, it will override the default config for that weapon kind
*/

#[derive(Clone, Serialize, Deserialize)]
pub struct CrosshairPreset {
    pub name: String,
    pub is_default: bool,
    pub crosshair_type: CrosshairType,
    pub color: [u8; 4], // we need to store it this way because Color doesn't implement Serialize/Deserialize
    pub dot_enabled: bool,
    pub dot_size: f32, // % of screen height
    pub line_length: f32, // % of screen height
    pub line_thickness: f32, // % of screen height
    pub center_gap: f32, // % of screen height — base hip-fire gap, shrinks to 0 when ADS
    pub outline_enabled: bool,
    pub outline_thickness: f32, // % of screen height
}

impl CrosshairPreset {
    // convert from runtime CrosshairConfig to storable CrosshairPreset
    pub fn from_config(name: &str, config: &CrosshairConfig, is_default: bool) -> Self {
        Self {
            name: name.to_string(),
            is_default,
            crosshair_type: config.crosshair_type,
            color: [
                (config.color.r * 255.0) as u8,
                (config.color.g * 255.0) as u8,
                (config.color.b * 255.0) as u8,
                (config.color.a * 255.0) as u8,
            ],
            dot_enabled: config.dot_enabled,
            dot_size: config.dot_size,
            line_length: config.line_length,
            line_thickness: config.line_thickness,
            center_gap: config.center_gap,
            outline_enabled: config.outline_enabled,
            outline_thickness: config.outline_thickness,
        }
    }

    // apply preset to a crosshair config
    pub fn apply_to(&self, config: &mut CrosshairConfig) {
        config.crosshair_type = self.crosshair_type;
        config.color = Color::from_rgba(self.color[0], self.color[1], self.color[2], self.color[3]);
        config.dot_enabled = self.dot_enabled;
        config.dot_size = self.dot_size;
        config.line_length = self.line_length;
        config.line_thickness = self.line_thickness;
        config.center_gap = self.center_gap;
        config.outline_enabled = self.outline_enabled;
        config.outline_thickness = self.outline_thickness;
    }
}

// manager for loading/saving presets, also holds the currently loaded presets in memory
pub struct CrosshairPresetManager {
    pub presets: Vec<CrosshairPreset>,
    pub save_path: PathBuf,
}

impl CrosshairPresetManager {
    pub fn new(username: &str) -> Self {
        let mut manager = Self {
            presets: Vec::new(),
            save_path: PathBuf::from(format!("crosshair_presets_{}.json", username)),
        };
        manager.rebuild_defaults();
        manager.load_user_presets();
        manager
    }
    // rebuild defaults
    fn rebuild_defaults(&mut self) {
        self.presets.clear();
        for kind in WeaponKind::ALL {
            let config = CrosshairConfig::for_weapon(kind);
            let name = format!("{} Default", kind.stats().name);
            self.presets.push(CrosshairPreset::from_config(&name, &config, true));
        }
    }
    // load user presets from JSON and append
    fn load_user_presets(&mut self) {
        if let Ok(data) = std::fs::read_to_string(&self.save_path) {
            if let Ok(user_presets) = serde_json::from_str::<Vec<CrosshairPreset>>(&data) {
                for mut p in user_presets {
                    p.is_default = false; // just in case, user presets should never be default
                    self.presets.push(p);
                }
            }
        }
    }

    // save only user non-default presets to JSON
    pub fn save(&self) {
        let user_only: Vec<&CrosshairPreset> = self.presets.iter()
        .filter(|p| !p.is_default)
        .collect();
        if let Ok(data) = serde_json::to_string_pretty(&user_only) {
            let _ = std::fs::write(&self.save_path, data);
        }
    }

    // create new user preset from config
    pub fn save_as_new(&mut self, name: &str, config: &CrosshairConfig) {
        self.presets.push(CrosshairPreset::from_config(name, config, false));
        self.save();
    }

    // overwrite existing user preset
    pub fn overwrite(&mut self, index: usize, config: &CrosshairConfig) {
        if index < self.presets.len() && !self.presets[index].is_default { // cant overwrite defaults
            self.presets[index] = CrosshairPreset::from_config(&self.presets[index].name, config, false);
            self.save();

        }
    }

    pub fn delete(&mut self, index: usize) {
        if index < self.presets.len() && !self.presets[index].is_default {
            self.presets.remove(index);
            self.save();
        }
    }

    pub fn default_count(&self) -> usize { // helper to count how many default presets there are, used to separate them in the UI
        self.presets.iter().filter(|p| p.is_default).count()
    }

    // find a preset by name, returns (index, is_default) if found
    pub fn find_by_name(&self, name: &str) -> Option<(usize, bool)> {
        self.presets.iter().enumerate()
            .find(|(_, p)| p.name == name)
            .map(|(i, p)| (i, p.is_default))
    }

    // suggested name for custom preset ("Custom_preset_1", "Custom_preset_2", etc)
    pub fn suggest_name(&self) -> String {
        let custom_count = self.presets.iter().filter(|p| !p.is_default).count();
        format!("custom_preset_{}", custom_count + 1)
    }
}
