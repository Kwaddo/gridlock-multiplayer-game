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

pub enum LoginField {
    Ip,
    Username,
}

pub enum MainMenuTab {
    Play,
    Servers,
}
