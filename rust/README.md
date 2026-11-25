# AYN Thor Second Screen — Horizon native mod

The Rust half of the second-screen HUD. It speaks the same wire protocol as the
Fabric mod (`127.0.0.1:48291`, newline-delimited JSON), so the companion app
does not need to know which mod loader the game runs.

## What it does

- Hides the game's own health/armor/food/air row, hotbar and XP bar while the
  app draws them on the bottom screen. The app's `HUD:<keys>` message hands any
  of those back to the top screen, the same full-state contract the Fabric mod
  uses.
- Streams the HUD snapshot (health, absorption, armor, food, XP, selected slot,
  air, game mode, heart type, nine hotbar slots, off-hand) on change.
- Streams the open menu (inventory or container) on change, with slot roles, so
  the app can draw it and click into it.
- Relays chat both ways, with colour runs, and replays a 100-message backlog to
  a reconnecting app.
- Answers `ICON:` requests with an explicit failure so the app falls back
  instead of waiting forever.
- Takes `1`–`9` hotbar taps, `BIND:<id>` presses, the legacy letter codes,
  `CHAT:` and `SLOT:` clicks. The bind retries every 5 s and says so in the
  player's chat, because a taken port is the one failure the second screen
  cannot report.

## What it does not do (yet)

- **Map tiles.** The app shows its "waiting for map data" state.
- **Item icons.** No offscreen item rendering on this side; the app's bundled
  fallback and placeholder draw instead.
- **Vanilla HUD textures.** Horizon does not expose the GUI atlas to mods the
  way Minecraft's resource manager does, so no `asset` messages are sent. The
  app's bundled textures are used.
- **The enchantment glint flag.** Horizon has no public foil read; `hasGlint`
  is always false.
- **Hardcore hearts.** No world-property read is exposed; `hardcore` is false.
- **A full binding registry.** `bindings` is a fixed list of common actions,
  not every mapping the game has.

## Build

The mod is native source: the launcher compiles it on the player's machine. To
check it yourself you need the pinned build's SDK in `.horizon/sdk/`, which
`cargo horizon-mod` fetches:

```sh
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
cd rust
cargo horizon-mod check
```

Package it without the tool:

```sh
python3 tools/pack_hmod.py
```

The manifest's `[native]` names the hook points' version (`hooks = 2`), so the
launcher builds the bundle on any Horizon whose API has them, including newer
Nightlies; `horizon` and `mixins` carry the commits of the newest reviewed
native mods in `horizon-rust/horizon-mods` for reference. If the launcher
refuses the package as made for another build, re-pin it to the one you play:

```sh
cargo horizon-mod update
```

That command fetches the installed build's SDK, which is only available while
Horizon publishes it.
