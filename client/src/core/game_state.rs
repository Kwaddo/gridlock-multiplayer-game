pub enum GameState {
    Login,
    LobbyInventory,
    Loading,
    Playing,
    Paused,
    Settings,
    /// The settings screen opened from the main menu (no game running).
    MenuSettings,
    DevMode,
    Exit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    Username,
    Address,
    ServerName,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MainMenuTab {
    Browse,
    Host,
    Direct,
}
