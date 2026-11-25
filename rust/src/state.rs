//! The snapshot the companion app draws: exactly the Java mod's `HudState`.

use crate::json::{field, number, string};

/// One item stack, in the shape the app renders. An empty slot is
/// `minecraft:air` with count 0, not an omitted entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slot {
    pub item_id: String,
    pub count: u32,
    pub damage: u32,
    pub max_damage: u32,
    pub glint: bool,
}

impl Slot {
    pub fn air() -> Self {
        Self {
            item_id: "minecraft:air".into(),
            ..Self::default()
        }
    }

    pub fn json(&self) -> String {
        format!(
            "{{\"itemId\":{},\"count\":{},\"damage\":{},\"maxDamage\":{},\"hasGlint\":{}}}",
            string(&self.item_id),
            self.count,
            self.damage,
            self.max_damage,
            self.glint
        )
    }
}

/// One tick's HUD, sent only when it changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub health: f32,
    pub max_health: f32,
    /// Golden hearts, on top of health/maxHealth.
    pub absorption: f32,
    pub armor: i32,
    pub food: i32,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub selected_slot: i32,
    /// Raw ticks, so the app applies vanilla's own bubble rounding.
    pub air: i32,
    pub max_air: i32,
    /// `SURVIVAL`, `CREATIVE`, ... or empty before a world reports one.
    pub game_mode: String,
    /// `NORMAL`, `POISONED`, `WITHERED` or `FROZEN`.
    pub heart_type: String,
    pub hardcore: bool,
    pub hotbar: Vec<Slot>,
    pub offhand: Slot,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            health: 0.0,
            max_health: 20.0,
            absorption: 0.0,
            armor: 0,
            food: 20,
            xp_level: 0,
            xp_progress: 0.0,
            selected_slot: 0,
            air: 300,
            max_air: 300,
            game_mode: String::new(),
            heart_type: "NORMAL".into(),
            hardcore: false,
            hotbar: vec![Slot::air(); 9],
            offhand: Slot::air(),
        }
    }
}

impl Snapshot {
    /// The full state line. Field names and nesting are the Java mod's; the app
    /// tells this shape apart from the others by the absence of `type`.
    pub fn json(&self) -> String {
        let hotbar: Vec<String> = self.hotbar.iter().map(Slot::json).collect();
        let mut out = String::from("{");
        let mut first = true;
        field(&mut first, "health", &number(self.health), &mut out);
        field(&mut first, "maxHealth", &number(self.max_health), &mut out);
        field(&mut first, "absorption", &number(self.absorption), &mut out);
        field(&mut first, "armor", &self.armor.to_string(), &mut out);
        field(&mut first, "food", &self.food.to_string(), &mut out);
        field(&mut first, "xpLevel", &self.xp_level.to_string(), &mut out);
        field(&mut first, "xpProgress", &number(self.xp_progress), &mut out);
        field(&mut first, "selectedSlot", &self.selected_slot.to_string(), &mut out);
        field(&mut first, "air", &self.air.to_string(), &mut out);
        field(&mut first, "maxAir", &self.max_air.to_string(), &mut out);
        field(&mut first, "gameMode", &string(&self.game_mode), &mut out);
        field(&mut first, "heartType", &string(&self.heart_type), &mut out);
        field(&mut first, "hardcore", &self.hardcore.to_string(), &mut out);
        field(&mut first, "hotbar", &format!("[{}]", hotbar.join(",")), &mut out);
        field(&mut first, "offhand", &self.offhand.json(), &mut out);
        out.push('}');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_floats_keep_a_decimal_point() {
        let mut snapshot = Snapshot::default();
        snapshot.health = 20.0;
        snapshot.xp_progress = 0.5;
        let json = snapshot.json();
        assert!(json.contains("\"health\":20.0"));
        assert!(json.contains("\"xpProgress\":0.5"));
    }

    #[test]
    fn a_default_snapshot_has_nine_empty_hotbar_slots() {
        let json = Snapshot::default().json();
        assert_eq!(json.matches("minecraft:air").count(), 10); // nine plus offhand
    }
}
