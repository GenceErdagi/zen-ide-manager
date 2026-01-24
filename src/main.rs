use std::collections::{BTreeMap, BTreeSet};

use zellij_tile::prelude::*;

#[derive(Default)]
struct State {
    config: Option<PluginConfig>,
    tabs: BTreeMap<usize, TabState>,
    active_tab: Option<usize>,
}

#[derive(Default, Debug, Clone)]
struct TabState {
    active_layout: Option<String>,
    last_bits: Option<u64>,
    is_dirty: bool,
}

#[derive(Debug, Clone)]
struct PluginConfig {
    default_layout: Option<String>,
    feature_to_bit: BTreeMap<String, u8>,
    state_bits: BTreeMap<String, u64>,
    bits_to_state: BTreeMap<u64, String>,
    commands: BTreeMap<String, CommandSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandKind {
    Toggle,
    Show,
    Hide,
    SetState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandTarget {
    Feature(String),
    State(String),
}

#[derive(Debug, Clone)]
struct CommandSpec {
    kind: CommandKind,
    target: CommandTarget,
}

impl CommandSpec {
    fn parse(raw: &str) -> Result<Self, String> {
        let normalized = raw.replace([':', '='], " ");
        let tokens: Vec<&str> = normalized
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .collect();
        if tokens.is_empty() {
            return Err("empty trigger definition".into());
        }

        let head = tokens[0].to_ascii_lowercase();
        let tail = tokens.get(1).copied();

        let (kind, target) = match head.as_str() {
            "toggle" => (
                CommandKind::Toggle,
                CommandTarget::Feature(
                    tail.ok_or_else(|| "toggle command missing feature".to_string())?
                        .to_string(),
                ),
            ),
            "show" | "on" => (
                CommandKind::Show,
                CommandTarget::Feature(
                    tail.ok_or_else(|| "show command missing feature".to_string())?
                        .to_string(),
                ),
            ),
            "hide" | "off" => (
                CommandKind::Hide,
                CommandTarget::Feature(
                    tail.ok_or_else(|| "hide command missing feature".to_string())?
                        .to_string(),
                ),
            ),
            "state" | "set_state" | "layout" => (
                CommandKind::SetState,
                CommandTarget::State(
                    tail.ok_or_else(|| "set_state command missing layout name".to_string())?
                        .to_string(),
                ),
            ),
            _ => (
                CommandKind::Toggle,
                CommandTarget::Feature(tokens[0].to_string()),
            ),
        };

        Ok(Self { kind, target })
    }
}

impl PluginConfig {
    fn parse(raw: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut default_layout = None;
        let mut layout_defs: Vec<(String, BTreeMap<String, bool>)> = Vec::new();
        let mut commands = BTreeMap::new();

        for (key, value) in raw {
            if key == "default_layout" {
                default_layout = Some(value.trim().to_string());
                continue;
            }

            if let Some(name) = key.strip_prefix("layout.") {
                let layout_features = parse_layout_line(value)?;
                layout_defs.push((name.to_string(), layout_features));
                continue;
            }

            if let Some(name) = key.strip_prefix("trigger.") {
                let command = CommandSpec::parse(value)?;
                commands.insert(name.to_string(), command);
            }
        }

        if layout_defs.is_empty() {
            return Err("no layouts configured".into());
        }

        let mut feature_set = BTreeSet::new();
        for (_, feature_map) in &layout_defs {
            for feature in feature_map.keys() {
                feature_set.insert(feature.clone());
            }
        }

        if feature_set.is_empty() {
            return Err("no features declared in layouts".into());
        }

        if feature_set.len() > 64 {
            return Err("supports up to 64 features".into());
        }

        let feature_order: Vec<String> = feature_set.into_iter().collect();
        let feature_to_bit = feature_order
            .iter()
            .enumerate()
            .map(|(idx, name)| (name.clone(), idx as u8))
            .collect::<BTreeMap<_, _>>();

        let mut state_bits = BTreeMap::new();
        let mut bits_to_state = BTreeMap::new();
        for (name, layout_features) in layout_defs {
            let mut bits = 0u64;
            for (feature_name, bit_index) in &feature_to_bit {
                if *layout_features.get(feature_name).unwrap_or(&false) {
                    bits |= 1u64 << bit_index;
                }
            }

            if let Some(existing) = bits_to_state.get(&bits) {
                eprintln!(
                    "zjide-manager: duplicate bitmask {bits} for layouts {existing} and {name}",
                );
            } else {
                bits_to_state.insert(bits, name.clone());
            }

            state_bits.insert(name, bits);
        }

        let default_layout = default_layout.or_else(|| state_bits.keys().next().cloned());

        Ok(Self {
            default_layout,
            feature_to_bit,
            state_bits,
            bits_to_state,
            commands,
        })
    }

    fn bit_for_feature(&self, feature: &str) -> Option<u64> {
        self.feature_to_bit.get(feature).map(|idx| 1u64 << idx)
    }

    fn default_bits(&self) -> Option<u64> {
        self.default_layout
            .as_ref()
            .and_then(|name| self.state_bits.get(name))
            .copied()
    }

    fn bits_for_state(&self, state: &str) -> Option<u64> {
        self.state_bits.get(state).copied()
    }

    fn resolve_target_bits(&self, current_bits: u64, command: &CommandSpec) -> Option<u64> {
        match (&command.kind, &command.target) {
            (CommandKind::Toggle, CommandTarget::Feature(feature)) => {
                self.bit_for_feature(feature).map(|bit| current_bits ^ bit)
            }
            (CommandKind::Show, CommandTarget::Feature(feature)) => {
                self.bit_for_feature(feature).map(|bit| current_bits | bit)
            }
            (CommandKind::Hide, CommandTarget::Feature(feature)) => {
                self.bit_for_feature(feature).map(|bit| current_bits & !bit)
            }
            (CommandKind::SetState, CommandTarget::State(state)) => self.bits_for_state(state),
            // Gracefully handle a set-state command that was defined without the explicit keyword
            (CommandKind::SetState, CommandTarget::Feature(state)) => self.bits_for_state(state),
            (_, CommandTarget::State(state)) => self.bits_for_state(state),
        }
    }

    fn closest_state(&self, target_bits: u64) -> Option<(String, u64)> {
        self.bits_to_state
            .iter()
            .map(|(bits, name)| ((bits ^ target_bits).count_ones(), name.clone(), *bits))
            .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)))
            .map(|(_, name, bits)| (name, bits))
    }
}

fn parse_layout_line(raw: &str) -> Result<BTreeMap<String, bool>, String> {
    let mut features = BTreeMap::new();

    if raw.trim().is_empty() {
        return Err("empty layout definition".into());
    }

    for chunk in raw.split(',') {
        let part = chunk.trim();
        if part.is_empty() {
            continue;
        }

        let mut pieces = part.splitn(2, '=');
        let feature = pieces
            .next()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| "missing feature name".to_string())?;

        let value = pieces.next().map(str::trim).unwrap_or("true");
        let enabled = match value {
            "true" | "1" | "on" => true,
            "false" | "0" | "off" => false,
            _ => {
                return Err(format!(
                    "invalid feature value '{value}' (expected true/false)"
                ))
            }
        };

        features.insert(feature.to_string(), enabled);
    }

    Ok(features)
}

register_plugin!(State);

impl State {
    fn on_tab_update(&mut self, tabs: Vec<TabInfo>) {
        let Some(config) = self.config.as_ref() else {
            return;
        };

        if let Some(active) = tabs.iter().find(|tab| tab.active) {
            self.active_tab = Some(active.position);
        }

        for tab in tabs {
            let tab_state = self.tabs.entry(tab.position).or_default();
            tab_state.active_layout = tab.active_swap_layout_name.clone();
            tab_state.is_dirty = tab.is_swap_layout_dirty;

            if let Some(layout_name) = tab_state.active_layout.as_ref() {
                if let Some(bits) = config.state_bits.get(layout_name) {
                    tab_state.last_bits = Some(*bits);
                }
            }
        }
    }

    fn current_bits(&self, config: &PluginConfig) -> Option<u64> {
        if let Some(active_tab) = self.active_tab {
            if let Some(tab_state) = self.tabs.get(&active_tab) {
                if let Some(layout_name) = tab_state.active_layout.as_ref() {
                    if let Some(bits) = config.state_bits.get(layout_name) {
                        return Some(*bits);
                    }
                }

                if let Some(bits) = tab_state.last_bits {
                    return Some(bits);
                }
            }
        }

        config.default_bits()
    }

    fn apply_command(&mut self, command_name: &str) {
        let Some(config) = self.config.as_ref() else {
            eprintln!("zjide-manager: plugin not configured yet");
            return;
        };

        let Some(command) = config.commands.get(command_name) else {
            eprintln!("zjide-manager: unknown trigger '{command_name}'");
            return;
        };

        let Some(current_bits) = self.current_bits(config) else {
            eprintln!("zjide-manager: unable to determine current layout bits");
            return;
        };

        let Some(target_bits) = config.resolve_target_bits(current_bits, command) else {
            eprintln!(
                "zjide-manager: trigger '{command_name}' references an unknown feature/state"
            );
            return;
        };

        let (target_layout, resolved_bits) = if let Some(layout) =
            config.bits_to_state.get(&target_bits)
        {
            (layout.clone(), target_bits)
        } else if let Some((layout, bits)) = config.closest_state(target_bits) {
            eprintln!(
                "zjide-manager: layout for mask {target_bits} missing, falling back to {layout}"
            );
            (layout, bits)
        } else {
            eprintln!("zjide-manager: no layouts available to satisfy trigger '{command_name}'");
            return;
        };

        go_to_swap_layout(&target_layout);

        if let Some(active_tab) = self.active_tab {
            let tab_state = self.tabs.entry(active_tab).or_default();
            tab_state.active_layout = Some(target_layout);
            tab_state.last_bits = Some(resolved_bits);
            tab_state.is_dirty = false;
        }
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[EventType::TabUpdate]);

        match PluginConfig::parse(&configuration) {
            Ok(config) => self.config = Some(config),
            Err(err) => eprintln!("zjide-manager: failed to parse configuration: {err}"),
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => self.on_tab_update(tabs),
            _ => {}
        }

        false
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        self.apply_command(&pipe_message.name);
        false
    }

    fn render(&mut self, _: usize, _: usize) {}
}
