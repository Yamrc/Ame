#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Recovering,
    Stopped,
    Error,
}

impl EngineState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use EngineState::*;
        match (self, next) {
            (Idle, Loading | Stopped) => true,
            (Loading, Ready | Playing | Stopped | Error) => true,
            (Ready, Loading | Playing | Stopped | Recovering | Error | Paused) => true,
            (Playing, Loading | Ready | Paused | Stopped | Recovering | Error) => true,
            (Paused, Loading | Playing | Stopped | Recovering | Error | Ready) => true,
            (Recovering, Loading | Playing | Paused | Stopped | Error | Ready) => true,
            (Stopped, Idle | Loading) => true,
            (Error, Idle | Loading | Stopped) => true,
            (from, to) => from == to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EngineState;

    #[test]
    fn state_transition_matrix_basics() {
        assert!(EngineState::Idle.can_transition_to(EngineState::Loading));
        assert!(EngineState::Playing.can_transition_to(EngineState::Paused));
        assert!(EngineState::Paused.can_transition_to(EngineState::Playing));
        assert!(!EngineState::Idle.can_transition_to(EngineState::Playing));
    }
}
