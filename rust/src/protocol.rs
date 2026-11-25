//! The fixed message shapes. The HUD snapshot and the container come from their
//! own modules; everything here is a line with a constant or near-constant body.

use crate::segment::Segment;
use crate::json::{field, string};

pub const MOD_ID: &str = "aynthor_secondscreen";

/// Protocol 1, the same number the Java mod speaks. The app uses it to say
/// "update the other half" rather than misbehave against a mismatched one.
pub const PROTOCOL_VERSION: i32 = 1;

/// The first line on every connection. It says *what* answered on the port: a
/// leftover `adb reverse tcp:48291` puts adbd there, which accepts and instantly
/// drops every connection.
pub fn hello() -> String {
    format!(
        "{{\"type\":\"hello\",\"mod\":{},\"protocol\":{}}}",
        string(MOD_ID),
        PROTOCOL_VERSION
    )
}

/// Sent once when the player leaves a world. Silence would be indistinguishable
/// from a stalled game, and the app would keep drawing the last snapshot.
pub fn no_player() -> String {
    "{\"type\":\"noplayer\"}".into()
}

/// "We tried, there is no icon for this one." Without an explicit answer the
/// app cannot tell a slow render from a dropped request.
pub fn icon_failure(item_id: &str) -> String {
    format!(
        "{{\"type\":\"icon\",\"itemId\":{},\"data\":null}}",
        string(item_id)
    )
}

/// One chat message, as coloured runs.
pub fn chat(segments: &[Segment]) -> String {
    let runs: Vec<String> = segments
        .iter()
        .map(|segment| {
            let mut out = String::from("{");
            let mut first = true;
            field(&mut first, "text", &string(&segment.text), &mut out);
            if let Some(color) = segment.color {
                field(&mut first, "color", &color.to_string(), &mut out);
            }
            out.push('}');
            out
        })
        .collect();
    format!("{{\"type\":\"chat\",\"segments\":[{}]}}", runs.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_names_the_mod_and_protocol() {
        assert_eq!(
            hello(),
            "{\"type\":\"hello\",\"mod\":\"aynthor_secondscreen\",\"protocol\":1}"
        );
    }

    #[test]
    fn chat_omits_a_missing_colour() {
        let json = chat(&[Segment {
            text: "hello".into(),
            color: None,
        }]);
        assert_eq!(json, "{\"type\":\"chat\",\"segments\":[{\"text\":\"hello\"}]}");
    }
}
