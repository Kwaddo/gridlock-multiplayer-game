//! Ceiling light placement over corridors and rooms.

use crate::maze::types::{MazeLight, Room};

pub(super) fn generate_ceiling_lights(width: u8, height: u8, cells: &[u8], rooms: &[Room]) -> Vec<MazeLight> {
    let width_usize = width as usize;
    let height_usize = height as usize;
    if width_usize == 0 || height_usize == 0 {
        return Vec::new();
    }

    let mut lights = Vec::new();
    let mut occupied = vec![false; width_usize * height_usize];
    let max_lights = ((width_usize * height_usize) / 24).max(12);

    for room in rooms {
        if room.w < 3 || room.h < 3 {
            continue;
        }

        let start_x = room.x.saturating_add(1);
        let end_x = room.x.saturating_add(room.w).saturating_sub(1);
        let start_y = room.y.saturating_add(1);
        let end_y = room.y.saturating_add(room.h).saturating_sub(1);

        if start_x >= end_x || start_y >= end_y {
            continue;
        }

        let start_x_usize = start_x as usize;
        let end_x_usize = end_x as usize;
        let start_y_usize = start_y as usize;
        let end_y_usize = end_y as usize;

        for x_idx in (start_x_usize..end_x_usize).step_by(4) {
            for y_idx in (start_y_usize..end_y_usize).step_by(4) {
                if lights.len() >= max_lights {
                    break;
                }
                try_add_light(
                    &mut lights,
                    &mut occupied,
                    width,
                    height,
                    cells,
                    x_idx as u8,
                    y_idx as u8,
                    max_lights,
                );
            }
            if lights.len() >= max_lights {
                break;
            }
        }

        let center_x = start_x_usize + ((end_x_usize - start_x_usize) / 2);
        let center_y = start_y_usize + ((end_y_usize - start_y_usize) / 2);
        if lights.len() < max_lights {
            try_add_light(
                &mut lights,
                &mut occupied,
                width,
                height,
                cells,
                center_x as u8,
                center_y as u8,
                max_lights,
            );
        }
    }

    if width_usize > 4 && height_usize > 4 {
        let spacing = 8;
        let max_x = width_usize - 2;
        let max_y = height_usize - 2;
        for gx in (2..max_x).step_by(spacing) {
            for gy in (2..max_y).step_by(spacing) {
                if lights.len() >= max_lights {
                    break;
                }
                try_add_light(
                    &mut lights,
                    &mut occupied,
                    width,
                    height,
                    cells,
                    gx as u8,
                    gy as u8,
                    max_lights,
                );
            }
            if lights.len() >= max_lights {
                break;
            }
        }
    }

    lights
}

fn try_add_light(
    lights: &mut Vec<MazeLight>,
    occupied: &mut [bool],
    width: u8,
    height: u8,
    cells: &[u8],
    x: u8,
    y: u8,
    max_lights: usize,
) {
    if x >= width || y >= height {
        return;
    }

    if lights.len() >= max_lights {
        return;
    }

    let width_usize = width as usize;
    let idx = y as usize * width_usize + x as usize;
    if occupied[idx] || cells[idx] == 0 {
        return;
    }

    if !has_clearance(cells, width, height, x, y) {
        return;
    }

    occupied[idx] = true;
    lights.push(MazeLight {
        x,
        y,
        variant: ((x as u16 + y as u16) % 3) as u8,
    });
}

fn has_clearance(cells: &[u8], width: u8, height: u8, x: u8, y: u8) -> bool {
    if width < 3 || height < 3 {
        return false;
    }

    if x == 0 || y == 0 || x >= width - 1 || y >= height - 1 {
        return false;
    }

    let width_usize = width as usize;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                return false;
            }
            let idx = (ny as usize) * width_usize + (nx as usize);
            if cells[idx] == 0 {
                return false;
            }
        }
    }

    true
}
