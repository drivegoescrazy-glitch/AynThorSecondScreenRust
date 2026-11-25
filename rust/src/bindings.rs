//! The binding list the app's input grid is built from.
//!
//! The Java mod enumerates every binding the game has. Horizon's mod API
//! presses a mapping by name rather than exposing a binding registry, so this
//! is a fixed list of the actions worth a second-screen button. `id` is the
//! wire identity: the app sends it back as `BIND:<id>` and the mod presses that
//! mapping. `boundKey` is left empty -- this side cannot read another key's
//! current binding -- and `unbound` is false so the app still shows the button.

use crate::json::{field, string};

pub struct Binding {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
}

/// Controls a player reaches for while their thumb is on the bottom screen.
pub const BINDINGS: &[Binding] = &[
    Binding { id: "key.attack", label: "Attack", category: "Gameplay" },
    Binding { id: "key.use", label: "Use Item", category: "Gameplay" },
    Binding { id: "key.pickItem", label: "Pick Block", category: "Gameplay" },
    Binding { id: "key.drop", label: "Drop Item", category: "Gameplay" },
    Binding { id: "key.swapOffhand", label: "Swap Offhand", category: "Gameplay" },
    Binding { id: "key.sneak", label: "Sneak", category: "Movement" },
    Binding { id: "key.sprint", label: "Sprint", category: "Movement" },
    Binding { id: "key.jump", label: "Jump", category: "Movement" },
    Binding { id: "key.forward", label: "Walk Forward", category: "Movement" },
    Binding { id: "key.back", label: "Walk Backward", category: "Movement" },
    Binding { id: "key.left", label: "Strafe Left", category: "Movement" },
    Binding { id: "key.right", label: "Strafe Right", category: "Movement" },
    Binding { id: "key.inventory", label: "Inventory", category: "Inventory" },
    Binding { id: "key.hotbar.1", label: "Hotbar 1", category: "Inventory" },
    Binding { id: "key.chat", label: "Chat", category: "Multiplayer" },
    Binding { id: "key.playerlist", label: "Player List", category: "Multiplayer" },
];

pub fn json() -> String {
    let entries: Vec<String> = BINDINGS
        .iter()
        .map(|binding| {
            let mut out = String::from("{");
            let mut first = true;
            field(&mut first, "id", &string(binding.id), &mut out);
            field(&mut first, "label", &string(binding.label), &mut out);
            field(&mut first, "category", &string(binding.category), &mut out);
            field(&mut first, "boundKey", &string(""), &mut out);
            field(&mut first, "unbound", "false", &mut out);
            out.push('}');
            out
        })
        .collect();
    format!("{{\"type\":\"bindings\",\"bindings\":[{}]}}", entries.join(","))
}

/// The old single-letter and action codes, kept so an app build that predates
/// `BIND:` still works. They name actions, never keys.
pub fn legacy_mapping(code: &str) -> Option<&'static str> {
    Some(match code {
        "INVENTORY" | "E" => "key.inventory",
        "DROP" | "G" => "key.drop",
        "SWAP" | "R" | "F" => "key.swapOffhand",
        "USE" | "H" => "key.use",
        "ATTACK" | "K" => "key.attack",
        "JUMP" => "key.jump",
        "SNEAK" => "key.sneak",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_well_formed_json_with_the_expected_identity() {
        let json = json();
        assert!(json.starts_with("{\"type\":\"bindings\",\"bindings\":["));
        assert!(json.contains("\"id\":\"key.jump\""));
        assert_eq!(BINDINGS.len(), json.matches("\"id\":").count());
    }

    #[test]
    fn letter_aliases_keep_the_behaviour_they_had() {
        assert_eq!(legacy_mapping("R"), Some("key.swapOffhand"));
        assert_eq!(legacy_mapping("nope"), None);
    }
}
