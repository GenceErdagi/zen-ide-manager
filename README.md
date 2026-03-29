# zjide-manager

`zjide-manager` is a Zellij plugin designed to provide IDE-like capabilities by managing workspace layouts through stateful feature toggles.

Instead of manually cycling through various swap layouts, this plugin allows users to define specific "features" (such as a sidebar, terminal, or debug panel) and maps combinations of these active features to specific layouts. Toggling a feature on or off automatically transitions the workspace to the appropriate layout using the standard Zellij `previous_swap_layout` and `next_swap_layout` APIs.

## Core Concept

The plugin operates on a bitmask system:

1.  **Features:** You define available features (e.g., `sidebar`, `terminal`).
2.  **Layout Mapping:** You map specific layout names to combinations of enabled features.
    *   Example: Layout `BASE` might represent `sidebar=true, terminal=true`.
    *   Example: Layout `zen` might represent `sidebar=false, terminal=false`.
3.  **State Management:** Sending a command to toggle a feature updates the internal state, and the plugin automatically navigates to the appropriate layout using the shortest path via prev/next swap layout commands.

## Configuration

For demonstration purposes, example configuration files have been provided in the `demo-config` directory. These files illustrate how to configure the plugin, define layouts, and set up keybindings.

*   `demo-config/config.kdl`: Plugin configuration.
*   `demo-config/layouts/ide.kdl`: Base layout + swap layouts (all in one file).

### Example Plugin Configuration

```kdl
plugin location="file:/path/to/zjide-manager.wasm" {
    // Define the default starting layout
    default_layout "BASE"
    // (Optional) Automatically switch to this layout on startup
    startup_layout "no_terminal"

    // Focus management
    default_focus_pane "Editor"
    pane_name.editor   "Editor"
    pane_name.terminal "Terminal"
    pane_name.sidebar  "File-Explorer"

    // Map layouts to feature flags
    layout.BASE        "sidebar=true, terminal=true"
    layout.no_sidebar  "sidebar=false, terminal=true"
    layout.no_terminal "sidebar=true, terminal=false"
    layout.zen         "sidebar=false, terminal=false"

    // Define triggers to control features
    trigger.toggle_sidebar  "toggle sidebar"
    trigger.toggle_terminal "toggle terminal"
    trigger.zen             "state zen"
}
```

## Focus Management

The plugin includes an intelligent focus management system that automatically shifts focus when layouts change:

1.  **Priority Focus:** When a feature is newly enabled (e.g., toggling the terminal on), the plugin prioritizes focusing that new pane.
    *   **Priority Order:** `terminal` > `sidebar` > other features.
2.  **Default Focus:** If no new features are enabled (e.g., toggling a feature off or just switching states), the plugin falls back to focusing the `default_focus_pane`.
3.  **Automatic Fallback:** If `default_focus_pane` is not explicitly set, it defaults to the value of `pane_name.editor`.

### Dynamic Focus via Pipe

You can manually trigger focus for any tracked pane using the `focus-pane` pipe message:

```kdl
bind "Alt f" {
    MessagePlugin "zjide-manager" {
        name "focus-pane"
        payload "Editor"
    }
}
```

### Keybindings

Keybindings send messages to the plugin to trigger state changes:

```kdl
bind "Alt e" {
    MessagePlugin "zjide-manager" {
        name "toggle_sidebar"
    }
}
```

## Building

Requirements:
- Rust with `wasm32-wasip1` target: `rustup target add wasm32-wasip1`

```bash
# Debug build
cargo build --target wasm32-wasip1

# Release build (smaller, optimized)
cargo build --release --target wasm32-wasip1
```

The wasm file will be at `target/wasm32-wasip1/debug/zjide-manager.wasm` or `target/wasm32-wasip1/release/zjide-manager.wasm`.

## Testing / Demo

The `demo-config` directory contains a complete example setup:

```bash
# Build the plugin first
cargo build

# Run the demo (use --config to load demo config, -l for layout)
ZELLIJ_CONFIG=demo-config/config.kdl zellij -l demo-config/layouts/ide.kdl
```

**Keybindings:**
- `Alt e` - Toggle sidebar (File-Explorer)
- `Alt r` - Toggle terminal (r for right pane)

## Release

To release manually:

```bash
# Build release version
cargo build --release --target wasm32-wasip1

# The wasm is at:
# target/wasm32-wasip1/release/zjide-manager.wasm

# Share this file with users
```
