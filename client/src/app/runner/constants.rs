use platform::Duration;

/// Time the loading screen may spend loading assets each frame; it keeps the
/// screen drawing (and a game hosted in this tab answering) at about 60 fps.
pub(super) const LOADING_FRAME_BUDGET: Duration = Duration::from_millis(12);
/// The same on the title screen and while a join is still connecting: less,
/// so the menus and the connection's own work (signalling, the join
/// handshake) keep their frames.
pub(super) const CONNECTING_FRAME_BUDGET: Duration = Duration::from_millis(8);

/// How long without hearing from the server before the connection counts as lost.
pub(super) const CLIENT_CONNECTION_LOST_TIMEOUT: Duration = Duration::from_secs(8);
