/// Longest tunnel dug out of a room to reach the maze; longer ones made long
/// straight lanes across the map.
pub(super) const MAX_ROOM_TUNNEL: usize = 5;

/// Share of the maze's dead ends that get opened into a loop. A perfect maze
/// is all dead ends, which means long walks back out and players hunting for
/// each other; loops give flanks and shorter routes between any two spots.
pub(super) const DEAD_END_OPEN_CHANCE: f64 = 0.65;
