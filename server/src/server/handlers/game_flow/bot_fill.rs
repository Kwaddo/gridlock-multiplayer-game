use crate::bot::personality::BotPersonalityId;
use crate::player::Team;
use crate::protocols::*;

use crate::server::game::MATCH_MAX_PLAYERS;

/// Match-bot personalities whose difficulty is enabled in `diff_mask`
/// (bit0=Recruit, bit1=Easy, bit2=Normal, bit3=Hard, bit4=Nightmare).
pub(super) fn eligible_bot_personalities(diff_mask: u8) -> Vec<BotPersonalityId> {
    use crate::bot::personality::{get_personality, BotDifficulty};
    const ALL: [BotPersonalityId; 15] = [
        BotPersonalityId::Heater,
        BotPersonalityId::Stinger,
        BotPersonalityId::Rex,
        BotPersonalityId::Merlin,
        BotPersonalityId::Viper,
        BotPersonalityId::Phantom,
        BotPersonalityId::Nexto,
        BotPersonalityId::Bandit,
        BotPersonalityId::Slater,
        BotPersonalityId::Storm,
        BotPersonalityId::Ghost,
        BotPersonalityId::Sniper,
        BotPersonalityId::Bulwark,
        BotPersonalityId::Wraith,
        BotPersonalityId::Overwatch,
    ];
    ALL.into_iter()
        .filter(|pid| {
            let bit = match get_personality(*pid).1 {
                BotDifficulty::Recruit => 0,
                BotDifficulty::Easy => 1,
                BotDifficulty::Normal => 2,
                BotDifficulty::Hard => 3,
                BotDifficulty::Nightmare => 4,
            };
            (diff_mask >> bit) & 1 == 1
        })
        .collect()
}

/// The team of each auto-fill bot to add, in spawn order. Gun Game has no
/// teams: `red` is the total and every bot is teamless. In team modes with
/// two or more humans, the `red + blue` bots are spread so the teams end up
/// even ([`balanced_bot_order`]); a player alone gets `red` and `blue` bots
/// as asked (say, one against many). Either way the order is interleaved,
/// so a match too full for every bot stays as even as it was. Never more
/// bots than free match slots or distinct personalities.
pub(super) fn bot_team_slots(
    match_mode: u8,
    red: u8,
    blue: u8,
    (red_humans, blue_humans): (usize, usize),
    personalities: usize,
) -> Vec<Team> {
    let humans = red_humans + blue_humans;
    let mut slots = Vec::new();
    if match_mode == MODE_GUN_GAME {
        slots.resize(red as usize, Team::Spectator);
    } else if humans >= 2 {
        slots = balanced_bot_order(red_humans, blue_humans, (red + blue) as usize)
            .into_iter()
            .map(|to_red| if to_red { Team::Red } else { Team::Blue })
            .collect();
    } else {
        for i in 0..red.max(blue) {
            if i < red {
                slots.push(Team::Red);
            }
            if i < blue {
                slots.push(Team::Blue);
            }
        }
    }
    slots.truncate(MATCH_MAX_PLAYERS.saturating_sub(humans).min(personalities));
    slots
}

#[cfg(test)]
mod bot_slot_tests {
    use super::*;

    #[test]
    fn team_modes_take_each_count() {
        // alone: as asked
        let slots = bot_team_slots(MODE_TEAM_DEATHMATCH, 1, 3, (1, 0), 15);
        assert_eq!(slots, vec![Team::Red, Team::Blue, Team::Blue, Team::Blue]);
    }

    #[test]
    fn with_two_or_more_humans_bots_even_the_teams() {
        // 1 v 1 asking for 0 red + 4 blue: spread 2 and 2
        let slots = bot_team_slots(MODE_TEAM_DEATHMATCH, 0, 4, (1, 1), 15);
        assert_eq!(slots.iter().filter(|t| **t == Team::Red).count(), 2);
        // 2 v 1 with 1 bot: it goes to Blue
        assert_eq!(bot_team_slots(MODE_TEAM_DEATHMATCH, 1, 0, (2, 1), 15), vec![Team::Blue]);
    }

    #[test]
    fn gun_game_bots_are_teamless() {
        let slots = bot_team_slots(MODE_GUN_GAME, 5, 0, (1, 0), 15);
        assert_eq!(slots, vec![Team::Spectator; 5]);
    }

    #[test]
    fn capped_by_free_slots_and_personalities() {
        assert_eq!(bot_team_slots(MODE_TEAM_DEATHMATCH, 8, 8, (7, 7), 15).len(), 2);
        assert_eq!(bot_team_slots(MODE_GUN_GAME, 15, 0, (0, 0), 4).len(), 4);
        let full = bot_team_slots(MODE_TEAM_DEATHMATCH, 4, 4, (6, 6), 15);
        assert_eq!(full, vec![Team::Red, Team::Blue, Team::Red, Team::Blue]);
    }
}
