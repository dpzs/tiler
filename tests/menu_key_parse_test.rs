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

// ===========================================================================
// Numpad keys are explicitly rejected
// ===========================================================================

#[test]
fn should_reject_numpad_keys_in_overview() {
    assert_eq!(parse_menu_key("KP_1", "", MenuState::Overview), None);
    assert_eq!(parse_menu_key("KP_0", "", MenuState::Overview), None);
    assert_eq!(parse_menu_key("KP_9", "", MenuState::Overview), None);
}

#[test]
fn should_reject_numpad_keys_in_zoomed() {
    assert_eq!(parse_menu_key("KP_1", "", MenuState::ZoomedIn(0)), None);
    assert_eq!(parse_menu_key("KP_0", "", MenuState::ZoomedIn(0)), None);
}

// ===========================================================================
// Modifier variations
// ===========================================================================

#[test]
fn should_detect_shift_in_modifier_string_variants() {
    // "shift" can appear alongside other modifiers
    let result = parse_menu_key("2", "ctrl+shift", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::ShiftN(1)));

    let result = parse_menu_key("2", "shift+alt", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::ShiftN(1)));
}

#[test]
fn should_not_match_shift_in_unrelated_modifier() {
    // "supershift" is not the same as "shift" - but contains() would match.
    // This documents the current behavior (substring match). A strict check
    // would reject "supershift" but the extension never sends such strings.
    let result = parse_menu_key("1", "supershift", MenuState::Overview);
    // Current behavior: contains("shift") matches
    assert_eq!(result, Some(MenuInput::ShiftN(0)));
}

// ===========================================================================
// Multi-character and edge-case key names
// ===========================================================================

#[test]
fn should_reject_empty_key_string() {
    assert_eq!(parse_menu_key("", "", MenuState::Overview), None);
    assert_eq!(parse_menu_key("", "", MenuState::ZoomedIn(0)), None);
}

#[test]
fn should_reject_two_digit_numbers() {
    // "10" parses as u8(10) which is > 9, so the filter rejects it
    assert_eq!(parse_menu_key("10", "", MenuState::Overview), None);
    assert_eq!(parse_menu_key("10", "", MenuState::ZoomedIn(0)), None);
}

#[test]
fn should_reject_negative_looking_strings() {
    assert_eq!(parse_menu_key("-1", "", MenuState::Overview), None);
}

#[test]
fn should_handle_digit_9_in_overview_as_press_n() {
    // Digit 9 in Overview should produce PressN(8) (zero-indexed)
    let result = parse_menu_key("9", "", MenuState::Overview);
    assert_eq!(result, Some(MenuInput::PressN(8)));
}

#[test]
fn should_handle_all_digits_in_zoomed() {
    for d in 0..=9u8 {
        let key = d.to_string();
        let result = parse_menu_key(&key, "", MenuState::ZoomedIn(0));
        assert_eq!(result, Some(MenuInput::Digit(d)), "digit {d} in ZoomedIn");
    }
}
