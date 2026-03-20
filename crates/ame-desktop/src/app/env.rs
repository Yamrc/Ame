use nekowg::{Entity, Global};

use crate::app::state::AppEntity;
use crate::domain::player::PlayerEntity;
use crate::domain::session::SessionState;
use crate::domain::shell::ShellState;

#[derive(Clone)]
pub struct AppEnv {
    pub app: Entity<AppEntity>,
    pub player: Entity<PlayerEntity>,
    pub shell: Entity<ShellState>,
    pub session: Entity<SessionState>,
}

impl Global for AppEnv {}

impl AppEnv {
    pub fn app(&self) -> Entity<AppEntity> {
        self.app.clone()
    }

    pub fn player(&self) -> Entity<PlayerEntity> {
        self.player.clone()
    }

    pub fn shell(&self) -> Entity<ShellState> {
        self.shell.clone()
    }

    pub fn session(&self) -> Entity<SessionState> {
        self.session.clone()
    }
}
