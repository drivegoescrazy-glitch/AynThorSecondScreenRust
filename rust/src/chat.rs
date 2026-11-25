//! Chat runs, flattened from the game's component tree.
//!
//! Minecraft's text is a tree of styled fragments. The app draws from a bitmap
//! font sheet, so only colour survives the flattening -- it's the part of the
//! styling that carries meaning in a log (team colours, a red death message).
//! This is the same trade the Java mod makes with `Text.visit`.

use mod_api::Text;
use mod_api::text::TextContent;

pub use crate::segment::Segment;

/// A message as runs, adjacent same-colour runs merged.
pub fn segments(text: &Text) -> Vec<Segment> {
    let mut out = Vec::new();
    flatten(text, None, &mut out);
    out
}

fn flatten(text: &Text, inherited: Option<u32>, out: &mut Vec<Segment>) {
    let color = text
        .style
        .color
        .as_deref()
        .and_then(color_value)
        .or(inherited);

    if let Some(run) = content_text(&text.content) {
        if !run.is_empty() {
            push(out, run, color);
        }
    }
    for child in &text.children {
        flatten(child, color, out);
    }
}

fn push(out: &mut Vec<Segment>, text: String, color: Option<u32>) {
    if let Some(last) = out.last_mut() {
        if last.color == color {
            last.text.push_str(&text);
            return;
        }
    }
    out.push(Segment { text, color });
}

/// The characters a component contributes before its children. A translation
/// key is shown as its fallback when it has one, else as the key itself: this
/// side has no language files to resolve it the way the game does.
fn content_text(content: &TextContent) -> Option<String> {
    match content {
        TextContent::Literal(text) => Some(text.clone()),
        TextContent::Translatable {
            key,
            fallback,
            arguments,
        } => {
            let template = fallback.clone().unwrap_or_else(|| key.clone());
            if arguments.is_empty() {
                Some(template)
            } else {
                // Vanilla's template uses %s; substitute the arguments' own
                // literal text so a death message still names the player.
                let mut out = String::new();
                let mut rest = template.as_str();
                for argument in arguments {
                    let Some(at) = rest.find("%s") else { break };
                    out.push_str(&rest[..at]);
                    out.push_str(&content_text(&argument.content).unwrap_or_default());
                    rest = &rest[at + 2..];
                }
                out.push_str(rest);
                Some(out)
            }
        }
        TextContent::Keybind(key) => Some(key.clone()),
        TextContent::Selector { pattern, .. } => Some(pattern.clone()),
        TextContent::Score { name, objective } => Some(format!("{name}:{objective}")),
        TextContent::Nbt { path, .. } => Some(path.clone()),
        TextContent::Object(_) => None,
    }
}

/// Vanilla's sixteen chat colour names, plus `#RRGGBB` components.
fn color_value(name: &str) -> Option<u32> {
    Some(match name {
        "black" => 0x000000,
        "dark_blue" => 0x0000AA,
        "dark_green" => 0x00AA00,
        "dark_aqua" => 0x00AAAA,
        "dark_red" => 0xAA0000,
        "dark_purple" => 0xAA00AA,
        "gold" => 0xFFAA00,
        "gray" => 0xAAAAAA,
        "dark_gray" => 0x555555,
        "blue" => 0x5555FF,
        "green" => 0x55FF55,
        "aqua" => 0x55FFFF,
        "red" => 0xFF5555,
        "light_purple" => 0xFF55FF,
        "yellow" => 0xFFFF55,
        "white" => 0xFFFFFF,
        hex if hex.starts_with('#') && hex.len() == 7 => {
            return u32::from_str_radix(&hex[1..], 16).ok();
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_colours_are_rgb() {
        assert_eq!(color_value("red"), Some(0xFF5555));
        assert_eq!(color_value("#12ab34"), Some(0x12ab34));
        assert_eq!(color_value("no_such_colour"), None);
    }
}
