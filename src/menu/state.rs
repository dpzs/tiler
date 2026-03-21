use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuState {
    Closed,
    Overview,
    ZoomedIn(u32),
}

impl Default for MenuState {
    fn default() -> Self {
        MenuState::Closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuInput {
    ToggleMenu,
    Escape,
    PressN(u32),
    ShiftN(u32),
    Digit(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuAction {
    Dismiss,
    ZoomIn(u32),
    MoveWindow(u32),
    ApplyLayout(u32, u8),
    EnforceOn(u32),
    EnforceOff(u32),
}

impl MenuState {
    pub fn transition(self, input: MenuInput) -> (MenuState, Option<MenuAction>) {
        match (self, input) {
            // Closed
            (MenuState::Closed, MenuInput::ToggleMenu) => (MenuState::Overview, None),
            (MenuState::Closed, _) => (MenuState::Closed, None),

            // Overview
            (MenuState::Overview, MenuInput::ToggleMenu) => {
                (MenuState::Closed, Some(MenuAction::Dismiss))
            }
            (MenuState::Overview, MenuInput::Escape) => {
                (MenuState::Closed, Some(MenuAction::Dismiss))
            }
            (MenuState::Overview, MenuInput::PressN(monitor)) => {
                (MenuState::ZoomedIn(monitor), Some(MenuAction::ZoomIn(monitor)))
            }
            (MenuState::Overview, MenuInput::ShiftN(monitor)) => {
                (MenuState::Closed, Some(MenuAction::MoveWindow(monitor)))
            }
            (MenuState::Overview, _) => (MenuState::Overview, None),

            // ZoomedIn
            (MenuState::ZoomedIn(_), MenuInput::Escape) => {
                (MenuState::Closed, Some(MenuAction::Dismiss))
            }
            (MenuState::ZoomedIn(monitor), MenuInput::Digit(d)) => match d {
                1..=4 => (MenuState::Closed, Some(MenuAction::ApplyLayout(monitor, d))),
                9 => (MenuState::Closed, Some(MenuAction::EnforceOn(monitor))),
                0 => (MenuState::Closed, Some(MenuAction::EnforceOff(monitor))),
                _ => (MenuState::ZoomedIn(monitor), None),
            },
            (MenuState::ZoomedIn(monitor), _) => (MenuState::ZoomedIn(monitor), None),
        }
    }
}
