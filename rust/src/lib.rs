//! AYN Thor Second Screen, as a Horizon native mod.
//!
//! The companion Android app draws the HUD on the handheld's bottom screen.
//! This half hides the game's own HUD and streams what it would have drawn over
//! a loopback socket, and takes short commands back: hotbar taps, key presses,
//! chat, and clicks into the open menu.
//!
//! This is the same wire protocol as the Fabric mod (`127.0.0.1:48291`,
//! newline-delimited JSON), deliberately: the app is the other half of a
//! release, and neither half should need to know which mod loader the game runs.

mod bindings;
mod chat;
mod container;
mod json;
mod protocol;
mod segment;
mod server;
mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use container::{ContainerSlot, ContainerState};
use mod_api::*;
use state::{Slot as HudSlot, Snapshot};

/// Vanilla's chat history cap, so a reconnecting app sees what the top screen's
/// log would show.
const CHAT_HISTORY_LIMIT: usize = 100;

/// `HUD:` payload from the app: element keys the *game* should draw, because
/// the player switched them off on the second screen. Full state every time.
static SHOWN_IN_GAME: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_HUD: Mutex<Option<String>> = Mutex::new(None);
static LAST_CONTAINER: Mutex<Option<String>> = Mutex::new(None);
static CHAT_HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());
static HAD_PLAYER: AtomicBool = AtomicBool::new(false);
static WARNED_BIND: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct AynThorSecondScreen;

#[mod_api::horizon_mod]
impl Hooks for AynThorSecondScreen {
    /// Once per client tick, paused or not: the snapshot is built here, commands
    /// are drained here, exactly where the Fabric mod's END_CLIENT_TICK ran.
    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        tick(host);
    }

    /// Per layered element: hide the ones the second screen draws.
    fn gui_layer_pre(&self, _host: &dyn ClientHost, event: &mut GuiLayerPreEvent<'_>) -> Flow {
        if game_draws(event.layer) {
            Flow::Continue
        } else {
            Flow::Cancel
        }
    }

    /// Every line the game shows in chat, player messages and server messages
    /// alike. `overlay` is the action bar, which is not chat.
    fn client_chat_received(
        &self,
        _host: &mut dyn ClientHost,
        event: &mut ClientChatReceivedEvent<'_>,
    ) -> Flow {
        if !event.overlay {
            relay_chat(&event.message);
        }
        Flow::Continue
    }
}

fn tick(host: &mut dyn ClientHost) {
    let server = server::server();

    // Commands first, so a tap acts on this tick rather than the next one.
    while let Some(command) = server.poll_command() {
        handle_command(host, &command);
    }

    // The port being taken is the one error whose audience cannot be reached
    // the usual way: the second screen is the thing that is not working.
    warn_about_bind_failure(host, server);

    let Some(player) = host.local_player() else {
        leave_world(server);
        return;
    };
    HAD_PLAYER.store(true, Ordering::Relaxed);

    let snapshot = read_snapshot(host, player);
    broadcast_if_changed(server, &snapshot);

    push_to_new_clients(server);

    if server.has_clients() {
        broadcast_container(host, player, server);
    }
}

/// Sends the snapshot only when it differs from the last one sent. Almost every
/// tick's snapshot is byte-identical while a player walks around, because
/// position is not in it; the Java mod measured 2 lines in 20 seconds idle
/// against ~400 before this.
fn broadcast_if_changed(server: &server::Server, snapshot: &Snapshot) {
    let json = snapshot.json();
    let mut last = LAST_HUD.lock().unwrap();
    if last.as_deref() == Some(json.as_str()) {
        return;
    }
    *last = Some(json.clone());
    drop(last);
    server.broadcast(&json);
}

/// Everything a freshly-connected app needs up front. The last snapshot and
/// container are replayed because both are only broadcast on change.
fn push_to_new_clients(server: &server::Server) {
    while let Some(client) = server.poll_new_client() {
        server.send_to(&client, &bindings::json());

        for message in CHAT_HISTORY.lock().unwrap().iter() {
            server.send_to(&client, message);
        }
        if let Some(container) = LAST_CONTAINER.lock().unwrap().as_ref() {
            server.send_to(&client, container);
        }
        if let Some(hud) = LAST_HUD.lock().unwrap().as_ref() {
            server.send_to(&client, hud);
        }
    }
}

/// Leaving a world is reported once, on the transition. The app would otherwise
/// keep showing the last snapshot forever while the game sits at the title
/// screen, which reads as a frozen app rather than as "not in a world".
fn leave_world(server: &server::Server) {
    if !HAD_PLAYER.swap(false, Ordering::Relaxed) {
        return;
    }
    CHAT_HISTORY.lock().unwrap().clear();
    *LAST_HUD.lock().unwrap() = None;
    *LAST_CONTAINER.lock().unwrap() = None;
    server.broadcast(&protocol::no_player());
}

fn read_snapshot(host: &mut dyn ClientHost, player: PlayerRef) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let entity = player.entity();
    let living = host.living(entity);

    if let Some(living) = living {
        if let Some(value) = living.health(host) {
            snapshot.health = value;
        }
        if let Some(value) = living.max_health(host) {
            snapshot.max_health = value;
        }
        if let Some(value) = living.absorption(host) {
            snapshot.absorption = value;
        }
        // Armor is an attribute in Horizon, not a number on the player.
        snapshot.armor = living
            .attribute_value(host, "minecraft:armor")
            .unwrap_or(0.0)
            .round()
            .clamp(0.0, 20.0) as i32;
    }
    if let Some(food) = player.food(host) {
        snapshot.food = food.level;
    }
    if let Some(xp) = player.experience(host) {
        snapshot.xp_level = xp.level;
        snapshot.xp_progress = xp.progress;
    }
    if let Some(slot) = player.selected_slot(host) {
        snapshot.selected_slot = slot as i32;
    }
    if let Some(air) = entity.air_supply(host) {
        snapshot.air = air;
    }
    if let Some(air) = entity.max_air_supply(host) {
        snapshot.max_air = air;
    }
    if let Some(mode) = player.game_mode(host) {
        snapshot.game_mode = mode.name().to_uppercase();
    }
    snapshot.heart_type = heart_type(host, living, entity);

    let (hotbar, offhand) = read_inventory(host, player);
    snapshot.hotbar = hotbar;
    snapshot.offhand = offhand;
    snapshot
}

/// Vanilla's precedence: poison, then wither, then freezing.
fn heart_type(
    host: &mut dyn ClientHost,
    living: Option<LivingRef>,
    entity: EntityRef,
) -> String {
    let effects = living
        .and_then(|living| living.effects(host))
        .unwrap_or_default();
    let mut poisoned = false;
    let mut withered = false;
    for effect in &effects {
        let id = effect.id.id(host);
        if id.ends_with(":poison") || id.ends_with(":infested") {
            poisoned = true;
        }
        if id.ends_with(":wither") {
            withered = true;
        }
    }
    if poisoned {
        return "POISONED".into();
    }
    if withered {
        return "WITHERED".into();
    }
    if entity.ticks_frozen(host).unwrap_or(0) > 0 {
        return "FROZEN".into();
    }
    "NORMAL".into()
}

/// The nine hotbar stacks and the off-hand, read from the open menu. The menu is
/// the right object rather than the inventory: with no container open it *is*
/// the player's inventory, and its slot roles say where each stack belongs.
fn read_inventory(host: &mut dyn ClientHost, player: PlayerRef) -> (Vec<HudSlot>, HudSlot) {
    let mut hotbar = vec![HudSlot::air(); 9];
    let mut offhand = HudSlot::air();

    let Some(menu) = player.container_menu(host) else {
        return (hotbar, offhand);
    };

    match menu.slot_kinds(host) {
        Some(kinds) => {
            for (index, kind) in kinds.iter().enumerate() {
                let MenuSlotKind::Player(slot) = kind else {
                    continue;
                };
                let stack = menu.slot(host, index).flatten();
                if *slot < 9 {
                    hotbar[*slot as usize] = slot_from(host, stack.as_ref());
                } else if *slot == 40 {
                    offhand = slot_from(host, stack.as_ref());
                }
            }
        }
        None if menu.slot_count(host) == Some(46) => {
            // The player's own inventory menu, whose indices vanilla fixes.
            for index in 0..9 {
                let stack = menu.slot(host, 36 + index).flatten();
                hotbar[index] = slot_from(host, stack.as_ref());
            }
            let stack = menu.slot(host, 45).flatten();
            offhand = slot_from(host, stack.as_ref());
        }
        None => {}
    }
    (hotbar, offhand)
}

fn slot_from(host: &dyn Host, stack: Option<&ItemStack>) -> HudSlot {
    let Some(stack) = stack else {
        return HudSlot::air();
    };
    let item = stack.item();
    let max_damage = host.max_damage(item).unwrap_or(0);
    HudSlot {
        item_id: item.id(host),
        count: stack.count(),
        damage: if max_damage > 0 { stack.damage() } else { 0 },
        max_damage: if max_damage > 0 { max_damage } else { 0 },
        // Horizon's API has no public foil read, so the app's animated glint is
        // not triggered yet. Damage and counts are exact.
        glint: false,
    }
}

fn broadcast_container(
    host: &mut dyn ClientHost,
    player: PlayerRef,
    server: &server::Server,
) {
    let Some(menu) = player.container_menu(host) else {
        return;
    };
    let Some(count) = menu.slot_count(host) else {
        return;
    };
    let kinds = menu.slot_kinds(host).unwrap_or_default();
    let cursor = menu.carried(host).flatten();

    let mut slots = Vec::with_capacity(count);
    let (mut player_start, mut hotbar_start, mut armor_start, mut offhand_index) = (-1, -1, -1, -1);
    for index in 0..count {
        let kind = kinds.get(index);
        match kind {
            Some(MenuSlotKind::Player(9)) => player_start = index as i32,
            Some(MenuSlotKind::Player(0)) => hotbar_start = index as i32,
            Some(MenuSlotKind::Player(36)) => armor_start = index as i32,
            Some(MenuSlotKind::Player(40)) => offhand_index = index as i32,
            _ => {}
        }
        let stack = menu.slot(host, index).flatten();
        slots.push(ContainerSlot {
            stack: slot_from(host, stack.as_ref()),
            // The client knows which slots are result slots; everything else is
            // a best-effort "you may put something here".
            may_place: !matches!(kind, Some(MenuSlotKind::Result)),
        });
    }

    // Every vanilla container keeps the player's 36 slots last; the player's own
    // menu is the exception with named indices, which the roles above found.
    if player_start < 0 {
        player_start = count.saturating_sub(36) as i32;
    }
    if hotbar_start < 0 {
        hotbar_start = count.saturating_sub(9) as i32;
    }

    let state = ContainerState {
        sync_id: menu.container_id(host).unwrap_or(0),
        handler_type: menu.kind(host).flatten(),
        cursor: slot_from(host, cursor.as_ref()),
        slots,
        player_start,
        hotbar_start,
        armor_start,
        offhand_index,
    };

    let json = state.json();
    let mut last = LAST_CONTAINER.lock().unwrap();
    if last.as_deref() == Some(json.as_str()) {
        return;
    }
    *last = Some(json.clone());
    drop(last);
    server.broadcast(&json);
}

fn handle_command(host: &mut dyn ClientHost, command: &str) {
    let server = server::server();

    if let Some(item) = command.strip_prefix("ICON:") {
        // No offscreen item rendering on this side yet; an explicit failure is
        // what stops the app waiting for an answer that is not coming.
        server.broadcast(&protocol::icon_failure(item.trim()));
        return;
    }
    if command == "ASSETS" {
        // The app's bundled textures stand in for the game's; nothing to send.
        return;
    }
    if command == "BINDINGS" {
        server.broadcast(&bindings::json());
        return;
    }
    if let Some(payload) = command.strip_prefix("HUD:") {
        set_shown_in_game(payload);
        return;
    }
    if let Some(line) = command.strip_prefix("CHAT:") {
        send_chat(host, line);
        return;
    }
    if let Some(args) = command.strip_prefix("SLOT:") {
        // A slot click is only meaningful while a screen is open, so unlike the
        // key presses below it is not guarded by `playing()`.
        click_slot(host, args);
        return;
    }

    if let Ok(digit) = command.parse::<u8>() {
        if (1..=9).contains(&digit) {
            let _ = host.select_hotbar_slot(digit - 1);
        }
        return;
    }

    // Vanilla only acts on key bindings while nothing is open; dropping the
    // command matches what a real keypress would have done at that moment.
    if !host.playing() {
        return;
    }
    if let Some(id) = command.strip_prefix("BIND:") {
        let _ = host.click_mapping(id.trim());
        return;
    }
    if let Some(mapping) = bindings::legacy_mapping(command) {
        let _ = host.click_mapping(mapping);
    }
}

fn send_chat(host: &mut dyn ClientHost, line: &str) {
    // The game normalises and truncates as its own chat box does; collapsing
    // whitespace here is the part that has to happen before the leading `/`.
    let message = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if message.is_empty() {
        return;
    }
    let _ = host.send_chat(&message);
}

fn click_slot(host: &mut dyn ClientHost, args: &str) {
    let parts: Vec<&str> = args.split(',').collect();
    if parts.len() != 4 {
        return;
    }
    let (Ok(sync_id), Ok(slot_id), Ok(button)) = (
        parts[0].trim().parse::<i32>(),
        parts[1].trim().parse::<i32>(),
        parts[2].trim().parse::<i32>(),
    ) else {
        return;
    };
    let action = parts[3].trim();

    let Some(player) = host.local_player() else {
        return;
    };
    let Some(menu) = player.container_menu(host) else {
        return;
    };

    // Staleness check first: a chest can close between the app drawing a slot
    // and a finger landing, and "slot 3" then means something else entirely.
    if menu.container_id(host).unwrap_or(0) != sync_id {
        return;
    }
    let slot = match slot_id {
        -999 => None, // clicked outside the window: drops the carried stack
        slot if slot >= 0 => Some(slot as usize),
        _ => return,
    };

    let mouse = if button == 1 {
        MouseButton::Right
    } else {
        MouseButton::Left
    };
    let click = match action {
        "PICKUP" => Some(MenuClick::Pickup { slot, button: mouse }),
        "QUICK_MOVE" => slot.map(|slot| MenuClick::QuickMove { slot }),
        // SWAP's button is the hotbar slot to exchange with (40 is the off-hand),
        // not a mouse button.
        "SWAP" => slot.map(|slot| MenuClick::Swap {
            slot,
            with: if button == 40 {
                SwapWith::Offhand
            } else {
                SwapWith::Hotbar(button.clamp(0, 8) as u8)
            },
        }),
        "CLONE" => slot.map(|slot| MenuClick::Clone { slot }),
        "THROW" => slot.map(|slot| MenuClick::Throw {
            slot,
            whole_stack: button == 1,
        }),
        "PICKUP_ALL" => slot.map(|slot| MenuClick::PickupAll { slot }),
        _ => None,
    };

    if let Some(click) = click {
        if menu.click(host, &[click]) {
            // The click mutates the menu; re-send on the next tick.
            *LAST_CONTAINER.lock().unwrap() = None;
        }
    }
}

fn set_shown_in_game(payload: &str) {
    let next: Vec<String> = payload
        .split(',')
        .map(|key| key.trim().to_lowercase())
        .filter(|key| !key.is_empty())
        .collect();
    *SHOWN_IN_GAME.lock().unwrap() = next;
}

/// Whether the game keeps drawing this HUD layer.
///
/// Horizon draws the health, armor, food and air rows as one layer, so the
/// app's four separate switches collapse to one: the layer is hidden unless at
/// least one of those elements was handed back. The XP bar shares its layer with
/// the mount jump bar and locator bar, which is why `xp_bar` names it.
fn game_draws(layer: &str) -> bool {
    let shown = SHOWN_IN_GAME.lock().unwrap();
    match layer {
        "minecraft:player_health" => shown
            .iter()
            .any(|key| matches!(key.as_str(), "armor" | "hearts" | "hunger" | "bubbles")),
        "minecraft:hotbar" => shown.iter().any(|key| key == "hotbar"),
        "minecraft:contextual_info_bar" => shown.iter().any(|key| key == "xp_bar"),
        _ => true,
    }
}

fn relay_chat(message: &Text) {
    let segments = chat::segments(message);
    if segments.is_empty() {
        return;
    }
    let line = protocol::chat(&segments);
    {
        let mut history = CHAT_HISTORY.lock().unwrap();
        history.push(line.clone());
        while history.len() > CHAT_HISTORY_LIMIT {
            history.remove(0);
        }
    }
    server::server().broadcast(&line);
}

fn warn_about_bind_failure(host: &mut dyn ClientHost, server: &server::Server) {
    let Some(failure) = server.bind_failure() else {
        WARNED_BIND.store(false, Ordering::Relaxed);
        return;
    };
    if WARNED_BIND.swap(true, Ordering::Relaxed) {
        return;
    }
    let line = format!(
        "[Thor HUD] Port {} is in use, so the second screen cannot connect ({failure}). \
         A leftover 'adb reverse tcp:{} tcp:{}' is the usual cause; clear it and this reconnects on its own.",
        server::PORT,
        server::PORT,
        server::PORT
    );
    let _ = host.chat_line(Text::literal(line));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_can_hand_the_health_row_back_to_the_game() {
        set_shown_in_game("hearts");
        assert!(game_draws("minecraft:player_health"));
        set_shown_in_game("");
        assert!(!game_draws("minecraft:player_health"));
        assert!(game_draws("minecraft:chat"));
    }

    #[test]
    fn xp_bar_owns_the_contextual_info_layer() {
        set_shown_in_game("xp_bar");
        assert!(game_draws("minecraft:contextual_info_bar"));
        assert!(!game_draws("minecraft:hotbar"));
    }
}
