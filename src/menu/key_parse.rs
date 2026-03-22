use super::state::{MenuInput, MenuState};

/// Parse a raw key name and modifier string into a [`MenuInput`],
/// given the current [`MenuState`]. Returns `None` if the key is
/// irrelevant in the current state.
pub fn parse_menu_key(key: &str, modifiers: &str, state: MenuState) -> Option<MenuInput> {
    match state {
        MenuState::Closed => None,
        _ if key == "Escape" => Some(MenuInput::Escape),
        _ => {
            // Only single-char digit keys ("0"-"9") are handled; numpad keys
            // like "KP_1" intentionally parse as Err and return None.
            let digit: u8 = key.parse().ok().filter(|&d| d <= 9)?;
            match state {
                MenuState::Overview => {
                    if digit == 0 {
                        return None;
                    }
                    let index = u32::from(digit - 1);
                    if modifiers.contains("shift") {
                        Some(MenuInput::ShiftN(index))
                    } else {
                        Some(MenuInput::PressN(index))
                    }
                }
                MenuState::ZoomedIn(_) => Some(MenuInput::Digit(digit)),
                // Closed is already handled in the outer match.
                MenuState::Closed => None,
            }
        }
    }
}
