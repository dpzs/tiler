use tiler::menu::state::{MenuState, MenuInput, MenuAction};

// ===========================================================================
// Initial state
// ===========================================================================

#[test]
fn test_menu_starts_closed() {
    let state = MenuState::default();
    assert_eq!(state, MenuState::Closed);
}

// ===========================================================================
// Closed -> Overview
// ===========================================================================

#[test]
fn test_closed_to_overview_on_toggle() {
    let (next, action) = MenuState::Closed.transition(MenuInput::ToggleMenu);
    assert_eq!(next, MenuState::Overview);
    assert_eq!(action, None);
}

// ===========================================================================
// Overview -> ZoomedIn
// ===========================================================================

#[test]
fn test_overview_to_zoomed_on_press_n() {
    let (next, action) = MenuState::Overview.transition(MenuInput::PressN(2));
    assert_eq!(next, MenuState::ZoomedIn(2));
    assert_eq!(action, Some(MenuAction::ZoomIn(2)));
}

// ===========================================================================
// Overview -> Closed (Esc)
// ===========================================================================

#[test]
fn test_overview_to_closed_on_esc() {
    let (next, action) = MenuState::Overview.transition(MenuInput::Escape);
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::Dismiss));
}

// ===========================================================================
// Overview -> Closed (ShiftN = move window)
// ===========================================================================

#[test]
fn test_overview_shift_n_moves_window_and_closes() {
    let (next, action) = MenuState::Overview.transition(MenuInput::ShiftN(3));
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::MoveWindow(3)));
}

// ===========================================================================
// ZoomedIn -> Closed (Esc)
// ===========================================================================

#[test]
fn test_zoomed_to_closed_on_esc() {
    let (next, action) = MenuState::ZoomedIn(1).transition(MenuInput::Escape);
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::Dismiss));
}

// ===========================================================================
// ZoomedIn -> Closed (apply layout 1-4)
// ===========================================================================

#[test]
fn test_zoomed_apply_layout_1() {
    let (next, action) = MenuState::ZoomedIn(0).transition(MenuInput::Digit(1));
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::ApplyLayout(0, 1)));
}

#[test]
fn test_zoomed_apply_layout_4() {
    let (next, action) = MenuState::ZoomedIn(5).transition(MenuInput::Digit(4));
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::ApplyLayout(5, 4)));
}

// ===========================================================================
// ZoomedIn -> Closed (9 = enforce on)
// ===========================================================================

#[test]
fn test_zoomed_enforce_on() {
    let (next, action) = MenuState::ZoomedIn(2).transition(MenuInput::Digit(9));
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::EnforceOn(2)));
}

// ===========================================================================
// ZoomedIn -> Closed (0 = enforce off)
// ===========================================================================

#[test]
fn test_zoomed_enforce_off() {
    let (next, action) = MenuState::ZoomedIn(2).transition(MenuInput::Digit(0));
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::EnforceOff(2)));
}

// ===========================================================================
// Invalid inputs are no-ops
// ===========================================================================

#[test]
fn test_closed_ignores_esc() {
    let (next, action) = MenuState::Closed.transition(MenuInput::Escape);
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, None);
}

#[test]
fn test_closed_ignores_digit() {
    let (next, action) = MenuState::Closed.transition(MenuInput::Digit(1));
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, None);
}

#[test]
fn test_overview_ignores_digit() {
    // Digits only matter in ZoomedIn
    let (next, action) = MenuState::Overview.transition(MenuInput::Digit(3));
    assert_eq!(next, MenuState::Overview);
    assert_eq!(action, None);
}

#[test]
fn test_zoomed_ignores_invalid_digit() {
    // Digit 5 is not a valid action
    let (next, action) = MenuState::ZoomedIn(0).transition(MenuInput::Digit(5));
    assert_eq!(next, MenuState::ZoomedIn(0));
    assert_eq!(action, None);
}

#[test]
fn test_zoomed_ignores_toggle() {
    let (next, action) = MenuState::ZoomedIn(0).transition(MenuInput::ToggleMenu);
    assert_eq!(next, MenuState::ZoomedIn(0));
    assert_eq!(action, None);
}

// ===========================================================================
// Toggle from Overview closes
// ===========================================================================

#[test]
fn test_overview_toggle_closes() {
    let (next, action) = MenuState::Overview.transition(MenuInput::ToggleMenu);
    assert_eq!(next, MenuState::Closed);
    assert_eq!(action, Some(MenuAction::Dismiss));
}
