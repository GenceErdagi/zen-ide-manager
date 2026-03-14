# zjide-manager

`zjide-manager` is a Zellij plugin designed to provide IDE-like capabilities by managing workspace layouts through stateful feature toggles.

Instead of manually cycling through various swap layouts, this plugin allows users to define specific "features" (such as a sidebar, terminal, or debug panel) and maps combinations of these active features to specific layouts. Toggling a feature on or off automatically transitions the workspace to the appropriate layout.

## ⚠️ Demonstration Purpose Only

**This project is published strictly as a demonstration/proof-of-concept.**

1.  **Custom Dependencies:** This plugin relies on a modified, custom version of Zellij and will not compile or function with the standard upstream crates. It specifically requires the features found in the [`/go-to-swap-layout-and-pipe-fix`]('https://github.com/GenceErdagi/zellij/tree/go-to-swap-layout') branch. This fork is necessary until the main Zellij repository incorporates new method for swap layout to a given name.
2.  **Environment Integration:** The original implementation is deeply integrated with a custom Nushell and Yazi setup, utilizing specific environment variables and IPC mechanisms that are not present in standard environments.

## Core Concept

The plugin operates on a bitmask system:

1.  **Features:** You define available features (e.g., `sidebar`, `terminal`).
2.  **Layout Mapping:** You map specific layout names to combinations of enabled features.
    *   Example: Layout `BASE` might represent `sidebar=true, terminal=true`.
    *   Example: Layout `zen` might represent `sidebar=false, terminal=false`.
3.  **State Management:** Sending a command to toggle a feature updates the internal state, and the plugin automatically switches the Zellij swap layout to match the new configuration.

## Configuration

For demonstration purposes, example configuration files have been provided in the `demo-config` directory. These files illustrate how to configure the plugin, define layouts, and set up keybindings.

*   `demo-config/config.kdl`: Contains the main Zellij configuration and plugin definitions.
*   `demo-config/ide.kdl`: Defines the base layout structure.
*   `demo-config/ide.swap.kdl`: Defines the swap layouts corresponding to different states (e.g., `no_sidebar`, `zen`).

### Example Plugin Configuration

```kdl
plugin location="file:/path/to/zjide-manager.wasm" {
    // Define the default starting layout
    default_layout "BASE"
    // (Optional) Automatically switch to this layout on startup
    startup_layout "no_terminal"

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

To build the plugin (assuming the required custom dependencies are present in the parent directory):

```bash
cargo build --target wasm32-wasip1
```
