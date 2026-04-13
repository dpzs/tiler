# Open Questions

## Iteration 1: Engine Bug Fixes

### Resolved with code changes
1. `move_window_to_monitor` source==target: Added early return guard.
2. `handle_fullscreen_changed` not re-applying layout presets: Fixed to re-apply preset on the window's monitor.
3. `handle_window_closed` double-tiling on stack monitor with preset: Fixed -- preset takes priority over stack tiling.
4. `handle_window_opened` preset vs stack priority: Preset check now happens before stack-monitor check.
5. `handle_window_closed` not clearing `focused_window_id`: Fixed.
6. Orphaned `stack_windows` entries: Added `prune_orphaned_windows` called during tiling.

### Resolved by keeping current behavior
7. `handle_focus_changed` accepting untracked windows: Kept as-is due to D-Bus signal ordering (focus can arrive before WindowOpened). `move_window_to_monitor` already guards against untracked windows.

### Still open
8. `move_window_to_monitor` does not re-apply preset on the source monitor after a window moves away. Only the stack screen and target are retiled.
9. No monitor hot-plug handling: `self.monitors` becomes stale if monitors change after startup.

## Iteration 2: Daemon Event Handling

### 1. Partial failure in handle_window_opened / handle_window_closed

Both `handle_window_opened` and `handle_window_closed` perform multiple steps
(insert into `windows` HashMap, add to desktop stack, retile). If retiling
fails mid-way (e.g., a D-Bus call to `move_resize_window` errors), the engine
state may be inconsistent: the window is tracked but not tiled, or removed from
tracking but the layout not updated.

Currently, errors from `dispatch_event` are logged and execution continues.
This is acceptable because:
- D-Bus errors are typically transient (extension restart, window closed between
  signal and handler).
- The next event (e.g., another window open/close) will trigger a re-tile.
- Making every step transactional (rollback on failure) would add significant
  complexity for marginal benefit.

However, the `prune_desktop` call added in iteration 1 helps mitigate this by
cleaning orphaned window IDs from the desktop stack on every `tile_stack` call.

**Decision**: No change needed now, but worth revisiting if users report
"stuck" layouts that don't recover.

### 2. Single-client IPC limitation

The daemon can only serve one IPC client at a time. While one client is
connected (in the inner `tokio::select!` loop), new connections queue in the
listener backlog. This is fine for the current use case (CLI tool connects,
sends one command, disconnects) but would be a problem if a long-lived client
held the connection open.

**Decision**: Acceptable for now. If multi-client support is needed, spawn each
client handler as a separate tokio task with a shared `Arc<Mutex<TilingEngine>>`.

### 3. tokio::select! fairness between event_rx and IPC

The two `tokio::select!` branches (inner and outer) that read from `event_rx`
use the same receiver, so there is no risk of double-consumption. The inner
branch handles events while an IPC client is connected; the outer branch
handles events between client connections. This is correct but relies on the
assumption that the inner loop `break`s promptly when the client disconnects.
If `read_message` blocked indefinitely on a half-open connection, events would
only be processed in the inner loop's `event_rx` branch and never in the outer
loop. Currently, `read_message` returns `Err` on EOF/disconnect, so this is
not a practical concern.

## Iteration 3: Layout and Stack Tiling Logic

### 1. Excess windows stashed at full monitor size consume resources

Both `apply_layout_to_monitor` and `tile_stack` (after the minimum-tile-size
cap was added) stash excess windows behind the layout at full monitor
dimensions. These windows are invisible (behind raised layout windows) but
remain alive and consume GPU compositing resources. Alternatives considered:

- **Minimize excess windows**: Would hide them from the user entirely, making
  them hard to recover. Also requires un-minimizing when the layout changes.
- **Move excess to the stack screen**: Would clutter the stack. Also creates
  a circular dependency if the stack itself has excess.
- **Show a visual indicator**: Could overlay a badge showing "+N hidden".

**Decision**: Current behavior (stash behind) is the least surprising. The
resource cost is negligible for typical window counts. Revisit if users
report performance issues with many excess windows.

### 2. Preset applied to the stack monitor

Nothing prevents a user from applying a preset layout (e.g., SideBySide) to
the same monitor that serves as the stack screen. In that case,
`apply_layout_to_monitor` and `tile_stack` would both try to position
windows on the same monitor, potentially conflicting. Currently the code
does not guard against this because the menu UI only shows non-stack
monitors for preset assignment, but the engine API does not enforce it.

**Decision**: Add a guard in a future iteration, or document that the stack
monitor index must not overlap with preset-assignable monitors.

### 3. The 500ms geometry-change grace period

The `TILING_GRACE` constant (500ms) suppresses geometry-change snap-back
signals after a tiling batch completes. This prevents false snap-backs from
Mutter's async geometry notifications. However:

- 500ms may be too long on fast hardware, causing a visible delay before
  enforcement resumes.
- 500ms may be too short on slow hardware or under heavy load, causing
  spurious snap-backs.

**Decision**: 500ms is a reasonable default based on observed Mutter behavior
on typical GNOME Wayland sessions. Could be made configurable if needed.

## Iteration 4: Menu State Machine and IPC Protocol Robustness

### Resolved with code changes

1. **ZoomedIn ignored ToggleMenu**: The menu hotkey was silently ignored when
   the menu was in ZoomedIn state, meaning the user had to press Escape or a
   valid digit to close the menu. Fixed: ToggleMenu now produces Dismiss from
   ZoomedIn, matching the behavior from Overview.

2. **`Command::ApplyLayout` with invalid layout digit left menu stuck**: When
   the daemon received `ApplyLayout` with a digit like 5-8, it set the menu
   state to `ZoomedIn(monitor)` but the state machine returned no action and
   stayed in ZoomedIn. The menu was then stuck in a zombie ZoomedIn state
   (no overlay shown, but state was not Closed). Fixed: the daemon now
   validates the layout digit before entering the state machine, and restores
   the previous menu state on failure.

3. **Test suite not compiling after `StackScreenPosition` API change**: All
   test files using `TilingEngine::new` and `run_daemon` were updated from
   integer indices to `StackScreenPosition::Left`/`Right`.

### Resolved by analysis (no code change needed)

4. **Menu overlay hiding on error in `handle_menu_input`**: Analyzed the
   execution order in `handle_menu_input`. The `hide_menu` call (line 543-545)
   happens *before* the action execution block (line 547+). So if
   `apply_layout_to_monitor` fails, the overlay is already hidden. The error
   propagates to the caller but the UI is in a clean state. No fix needed.

5. **`parse_menu_key` substring match for "shift" modifier**: The function
   uses `modifiers.contains("shift")` which would match a hypothetical
   modifier string like "supershift". However, the GNOME extension never
   sends such strings -- modifier names are standardized ("shift", "ctrl",
   "alt", "super"). Documented in a test as a known behavior.

6. **IPC decode_frame 16 MiB limit**: The MAX_FRAME_SIZE of 16 MiB is
   reasonable for this use case. Commands and responses are small JSON
   payloads (typically < 1 KB). The limit prevents memory exhaustion from
   malformed length headers. No change needed.

7. **Single-client IPC blocking**: Already documented in iteration 2. The
   inner `tokio::select!` loop processes events even while a client is
   connected. A slow client blocks other IPC *clients* but not event
   processing. Acceptable for the single-extension architecture.

### Still open

8. **No read timeout on IPC connections**: A client that connects but sends
   no data (or sends a partial frame) will hold the inner loop indefinitely.
   The `read_message` call will block on `read_exact` until the client
   disconnects or sends data. A `tokio::time::timeout` wrapper around
   `read_message` would allow the daemon to drop idle connections and return
   to the outer accept loop. This is low-priority since the only client is
   the CLI tool which connects, sends, and disconnects immediately.

9. **`Command::ApplyLayout` does not validate `monitor` index**: The engine
   handles out-of-range monitor IDs gracefully (returns Ok with no action
   from `apply_layout_to_monitor` when `monitor_rect` returns None), but the
   layout preset is still stored in the desktop's `layout_presets` map for
   a non-existent monitor. This wastes a small amount of memory and could
   cause confusion if a monitor is later hot-plugged with that ID. A
   validation step in the daemon or engine could reject unknown monitor IDs.

## Iteration 6: Config, Startup, and NixOS Module

### Resolved with code changes

1. **Config was defined but never loaded or used**: `main.rs` hardcoded
   `stack_screen_index: 0` and never loaded `TilerConfig`. Fixed: the daemon
   now loads and validates config at startup, and the `stack_screen_position`
   setting is wired through to the engine.

2. **`stack_screen_position` was an unvalidated string**: Any value was
   accepted at parse time. Fixed: added `StackScreenPosition` enum with
   `parse()` and `validate()` methods. Invalid values produce a clear error
   at daemon startup.

3. **No signal handling**: `main.rs` passed `shutdown: None` to `run_daemon`,
   meaning SIGTERM/SIGINT would kill the daemon without cleanup. Fixed: the
   daemon now registers signal handlers that send to the shutdown oneshot
   channel, enabling graceful socket file cleanup.

4. **`stack_screen_index` could be out of range**: The engine accepted a raw
   `usize` index with no validation against the monitor list. Fixed: the
   engine now accepts `StackScreenPosition` and resolves the index during
   `startup()` using the actual monitor geometry.

5. **Empty monitor list not handled**: `startup()` would proceed with no
   monitors, leading to a confusing state. Fixed: startup now returns an
   error if `get_monitors()` returns an empty list.

6. **NixOS config file not found by daemon**: The NixOS module places the
   config at `/etc/tiler/config.toml` but the daemon only checked the user
   XDG path. Fixed: `TilerConfig::default_path()` now checks `$TILER_CONFIG`
   env var, then `$XDG_CONFIG_HOME/tiler/config.toml`, then
   `/etc/tiler/config.toml`, returning the first that exists.

### Still open

10. **NixOS systemd service missing `Environment` for D-Bus**: The
    `tiler.service` does not set `DBUS_SESSION_BUS_ADDRESS` or
    `DISPLAY` in its `serviceConfig`. These are typically inherited from the
    graphical session via `graphical-session.target`, but on some
    non-standard setups they might not be. If users report connection
    failures, adding `Environment` entries may be needed.

11. **Double-start pattern observed in logs**: The daemon starts, fails to
    connect to the GNOME Shell extension (because the extension hasn't
    started yet), exits with code 1, and systemd restarts it 2 seconds later
    (per `RestartSec = 2`). The second attempt succeeds. This is the
    expected behavior of `Restart = on-failure` and `RestartSec = 2`, not a
    bug. However, a more robust approach would be to retry the D-Bus
    connection within the daemon itself (with backoff) rather than relying on
    systemd restart. This would avoid unnecessary process churn and log noise.

12. **Socket file race condition**: The daemon removes the socket file before
    binding (`let _ = std::fs::remove_file(socket_path)`). If two daemon
    instances start simultaneously, one could remove the other's socket.
    This is mitigated by systemd's service model (only one instance runs at
    a time) but is not protected at the Rust level. A file lock or PID file
    could prevent this.

## Iteration 8: Error Handling Audit

### Resolved with code changes

1. **Silent D-Bus signal parse errors in `zbus_proxy.rs`**: Six of seven
   signal handlers in `spawn_signal_listener` used `if let Ok(args)` which
   silently swallowed parse errors. Only the `WindowOpened` handler logged
   the error. Fixed: all seven handlers now use `match` and log errors at
   ERROR level with the signal name for diagnostics.

2. **`expect()` panic in `engine.rs` startup**: `resolve_index` result was
   unwrapped with `expect()`. While the invariant (monitors is non-empty)
   was checked above, a panic in a daemon is undesirable. Fixed: replaced
   with `.ok_or(...)? ` to propagate as a proper error.

3. **`init_logging` panicked on log file open failure**: The function used
   `expect("failed to open log file")` which gave no path context and
   crashed the daemon. Fixed: returns `Result<WorkerGuard, std::io::Error>`
   with the log file path included in the error message. Caller in `main.rs`
   now handles the error with a user-friendly message and `exit(1)`.

4. **`ipc/client.rs` destroyed error chain**: `send_message` and
   `read_message` results were re-wrapped via `.map_err(|e| e.to_string().into())`
   which destroyed the `.source()` chain. Both functions already return
   `Box<dyn Error + Send + Sync>`, so the re-wrapping was unnecessary.
   Fixed: errors now propagate directly with `?`.

5. **`ipc/client.rs` connection error lacked context**: A failed
   `UnixStream::connect` gave a raw OS error with no indication of what
   socket was being connected to or that the daemon might not be running.
   Fixed: wrapped with a message including the socket path and a hint.

6. **IPC stream corruption tests**: Added two tests proving that a malformed
   JSON frame and a zero-length frame do not corrupt subsequent valid
   messages on the same channel, thanks to the length-prefix framing.

### Analyzed and kept as-is

7. **`let _ = std::fs::remove_file(socket_path)` in daemon.rs**: Three
   occurrences. These are intentional: the file may not exist (startup
   cleanup) or the daemon is shutting down and failure to remove the socket
   is not actionable. Keeping as-is.

8. **`let _ = shutdown_tx.send(())` in main.rs**: The receiver may have been
   dropped if the daemon already exited. The signal handler's job is done
   regardless. Keeping as-is.

9. **`let _ = send_message(...)` during Shutdown handling**: The daemon is
   about to exit; a failed send to the disconnecting client is harmless.
   Keeping as-is.

10. **`let _ = rx.await` for shutdown channel**: If the sender is dropped,
    the channel returns `Err(Canceled)`, which is a valid shutdown signal.
    The `_` is correct here. Keeping as-is.

11. **`dispatch_event` logs errors but continues**: All event handler errors
    are logged at ERROR and execution continues. This is the correct policy
    for a daemon: D-Bus errors are typically transient (window closed between
    signal and handler, extension restarted), and the next event will trigger
    recovery. Crashing/restarting the daemon on a transient D-Bus error would
    cause worse user experience than continuing with a logged error.

12. **`expect()` for signal handlers in main.rs**: The `SIGTERM` and `SIGINT`
    signal registration uses `expect()`. These are called once at startup in
    a spawned task. Signal registration failure is a fundamental OS-level
    problem (not a recoverable runtime condition), so panicking is
    appropriate. The messages are descriptive.

13. **`expect()` for config validation in main.rs line 82**: Uses
    `expect("config was validated above")` after `config.validate()` already
    passed. The invariant is provably upheld by the preceding validation
    check. Keeping as-is.

### Still open

14. **No IPC read timeout**: A client that connects but never sends data
    (or sends a partial frame) blocks the inner select loop indefinitely.
    See iteration 4, item 8 for full analysis. Low priority since the only
    client is the CLI tool.

15. **`dispatch_event` return type**: Currently returns `()`. Making it
    return `Result` would let the daemon distinguish between transient
    errors (log and continue) and fatal errors (e.g., D-Bus connection
    permanently lost). However, distinguishing "permanent D-Bus disconnect"
    from "transient call failure" requires inspecting the zbus error type,
    which is not trivially available through the `Box<dyn Error>` type
    alias. This would require a proper error enum with variants for
    transient vs fatal errors -- a larger refactor best done when the
    need arises.
