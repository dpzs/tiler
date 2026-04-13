use super::state::{MenuInput, MenuState};

/// Parse a raw key name and modifier string into a [`MenuInput`] for the given [`MenuState`].
///
/// Returns `None` when the key has no meaning in the current state (e.g. any
/// key while the menu is [`MenuState::Closed`], or an unrecognised key name).
///
/// # Key handling rules
///
/// - `"Escape"` always produces [`MenuInput::Escape`] while the menu is open.
/// - In [`MenuState::Overview`]: digit keys `"1"`–`"9"` are accepted (`"0"` is
///   ignored). A `modifiers` string containing `"shift"` produces
///   [`MenuInput::ShiftN`]; otherwise [`MenuInput::PressN`]. The index passed
///   is `digit - 1` (zero-based).
/// - In [`MenuState::ZoomedIn`]: digit keys `"0"`–`"9"` produce
///   [`MenuInput::Digit`].
/// - Numpad key names (e.g. `"KP_1"`) are intentionally rejected and return
///   `None`.
#[must_use]
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
