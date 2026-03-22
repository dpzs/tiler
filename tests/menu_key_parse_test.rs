use tiler::menu::key_parse::parse_menu_key;
use tiler::menu::state::{MenuState, MenuInput};

// ===========================================================================
// Escape key
// ===========================================================================

#[test]
fn should_parse_escape_in_overview() {
    let result = parse_menu_key("Escape", "", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::Escape));
}

#[test]
fn should_parse_escape_in_zoomed() {
    let result = parse_menu_key("Escape", "", MenuState::ZoomedIn(0));
    assert_eq!(result, Some(MenuInput::Escape));
}

// ===========================================================================
// Digit keys in Overview -> PressN (0-indexed)
// ===========================================================================

#[test]
fn should_parse_digit_1_overview_returns_press_n_0() {
    let result = parse_menu_key("1", "", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::PressN(0)));
}

#[test]
fn should_parse_digit_3_overview_returns_press_n_2() {
    let result = parse_menu_key("3", "", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::PressN(2)));
}

#[test]
fn should_parse_digit_0_overview_returns_none() {
    let result = parse_menu_key("0", "", MenuState::Overview);
    assert_eq!(result, None);
}

// ===========================================================================
// Shift+Digit in Overview -> ShiftN (0-indexed)
// ===========================================================================

#[test]
fn should_parse_digit_1_shift_overview_returns_shift_n_0() {
    let result = parse_menu_key("1", "shift", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::ShiftN(0)));
}

#[test]
fn should_parse_digit_0_shift_overview_returns_none() {
    let result = parse_menu_key("0", "shift", MenuState::Overview);
    assert_eq!(result, None);
}

// ===========================================================================
// Digit keys in ZoomedIn -> Digit (raw value)
// ===========================================================================

#[test]
fn should_parse_digit_1_zoomed_returns_digit_1() {
    let result = parse_menu_key("1", "", MenuState::ZoomedIn(0));
    assert_eq!(result, Some(MenuInput::Digit(1)));
}

#[test]
fn should_parse_digit_0_zoomed_returns_digit_0() {
    let result = parse_menu_key("0", "", MenuState::ZoomedIn(0));
    assert_eq!(result, Some(MenuInput::Digit(0)));
}

#[test]
fn should_parse_digit_9_zoomed_returns_digit_9() {
    let result = parse_menu_key("9", "", MenuState::ZoomedIn(0));
    assert_eq!(result, Some(MenuInput::Digit(9)));
}

// ===========================================================================
// Unknown / irrelevant keys
// ===========================================================================

#[test]
fn should_parse_unknown_key_returns_none() {
    let result = parse_menu_key("x", "", MenuState::Overview);
    assert_eq!(result, None);

    let result = parse_menu_key("F1", "", MenuState::Overview);
    assert_eq!(result, None);
}

// ===========================================================================
// Closed state ignores all keys
// ===========================================================================

#[test]
fn should_parse_any_key_closed_returns_none() {
    let result = parse_menu_key("1", "", MenuState::Closed);
    assert_eq!(result, None);

    let result = parse_menu_key("Escape", "", MenuState::Closed);
    assert_eq!(result, None);
}
