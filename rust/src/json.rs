//! Minimal JSON writing. The wire format is newline-delimited JSON and every
//! message here is small and shaped by this mod alone, so a dependency on a
//! serializer would buy nothing and cost the launcher a crate to build.

/// A JSON string literal, quotes included.
pub fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A float as JSON. Whole numbers keep a `.0` so a reader that distinguishes
/// integers from floats (the app's parser does) sees the same type the Java mod
/// sent, and non-finite values become 0 rather than invalid JSON.
pub fn number(value: f32) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    if value == value.trunc() && value.abs() < 1e9 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// `"key":value` with a comma in front when it isn't the first field.
pub fn field(first: &mut bool, key: &str, value: &str, out: &mut String) {
    if !*first {
        out.push(',');
    }
    *first = false;
    out.push_str(&string(key));
    out.push(':');
    out.push_str(value);
}
