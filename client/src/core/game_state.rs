pub enum GameState {
    Login,
    LobbyInventory,
    Loading,
    Playing,
    Paused,
    Settings,
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
