//! The open menu, in the shape the app's inventory tab reads.
//!
//! The Java mod streams `player.currentScreenHandler`, not the inventory: moving
//! an item is a click on a slot, and the server only honours those against the
//! handler it believes is open. With no container open that handler is the
//! player's own inventory, so the same message covers both cases.

use crate::json::{field, string};
use crate::state::Slot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerSlot {
    pub stack: Slot,
    pub may_place: bool,
}

/// One menu's contents, plus where the player's own slots start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerState {
    pub sync_id: i32,
    /// `minecraft:generic_9x3` and the like; `None` for the player's own
    /// inventory, which vanilla gives no menu type.
    pub handler_type: Option<String>,
    pub cursor: Slot,
    pub slots: Vec<ContainerSlot>,
    pub player_start: i32,
    pub hotbar_start: i32,
    pub armor_start: i32,
    pub offhand_index: i32,
}

impl ContainerState {
    pub fn json(&self) -> String {
        let slots: Vec<String> = self
            .slots
            .iter()
            .map(|slot| {
                format!(
                    "{{\"stack\":{},\"mayPlace\":{}}}",
                    slot.stack.json(),
                    slot.may_place
                )
            })
            .collect();

        let mut out = String::from("{");
        let mut first = true;
        field(&mut first, "type", &string("container"), &mut out);
        field(&mut first, "syncId", &self.sync_id.to_string(), &mut out);
        field(
            &mut first,
            "handlerType",
            &self
                .handler_type
                .as_deref()
                .map(string)
                .unwrap_or_else(|| "null".into()),
            &mut out,
        );
        field(&mut first, "cursor", &self.cursor.json(), &mut out);
        field(&mut first, "slots", &format!("[{}]", slots.join(",")), &mut out);
        field(&mut first, "playerStart", &self.player_start.to_string(), &mut out);
        field(&mut first, "hotbarStart", &self.hotbar_start.to_string(), &mut out);
        field(&mut first, "armorStart", &self.armor_start.to_string(), &mut out);
        field(&mut first, "offhandIndex", &self.offhand_index.to_string(), &mut out);
        out.push('}');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_inventory_menu_serialises_its_roles() {
        let state = ContainerState {
            sync_id: 0,
            handler_type: None,
            cursor: Slot::air(),
            slots: Vec::new(),
            player_start: 9,
            hotbar_start: 36,
            armor_start: 5,
            offhand_index: 45,
        };
        let json = state.json();
        assert!(json.contains("\"type\":\"container\""));
        assert!(json.contains("\"handlerType\":null"));
        assert!(json.contains("\"hotbarStart\":36"));
    }
}
