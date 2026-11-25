//! The one bind failure the mod cannot fix on its own.
//!
//! When port 48291 is taken, the usual cause is a leftover
//! `adb reverse tcp:48291 tcp:48291`: adbd holds the port and answers every
//! connection by dropping it. Clearing it means running the command above, which
//! the player cannot do from inside the game, and the mod cannot do either -- a
//! mod has the player's permissions and no shell. So it says what it found:
//! which process holds the port, and what to clear if adbd is that process.
//!
//! Nothing here decides anything for the game; it reads `/proc` and formats, so
//! it is plain code with plain tests.

/// The mod's port, as both halves of the release know it.
pub const PORT: u16 = 48291;

/// What the player should be told about the process on the port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holder {
    /// adbd: `adb reverse` made it the port's owner, and clearing it is the fix.
    Adb(u32),
    /// adbd, but the list this mod could read did not name the socket's owner.
    AdbUnnamed,
    /// Another process entirely; the player has to close it.
    Process(String),
    /// Nothing on the port, which makes the failure a permissions problem.
    Nothing,
    /// `/proc` could not be read, so there is nothing to say.
    Unknown,
}

/// Turns a bind error and what `/proc` said into the sentence shown in chat.
pub fn explain(error: &str, holder: &Holder) -> String {
    let ports = format!("tcp:{PORT} tcp:{PORT}");
    let failure = format!("Port {PORT} is in use, so the second screen cannot connect ({error}).");
    match holder {
        Holder::Adb(pid) => format!(
            "{failure} It is adbd (pid {pid}): a leftover 'adb reverse {ports}' is holding it. \
             Clear it with 'adb shell adb reverse --remove {ports}' (or replug the cable, or \
             reboot the Thor); this reconnects on its own."
        ),
        Holder::AdbUnnamed => format!(
            "{failure} It is adbd: a leftover 'adb reverse {ports}' is holding it. Clear it with \
             'adb shell adb reverse --remove {ports}' (or replug the cable, or reboot the Thor); \
             this reconnects on its own."
        ),
        Holder::Process(name) => format!(
            "{failure} It is '{name}', not the game. Close it; this reconnects on its own."
        ),
        Holder::Nothing => format!(
            "{failure} Nothing holds the port, so the game may not use the network."
        ),
        Holder::Unknown => failure,
    }
}

/// `/proc/net/tcp` and `/proc/net/tcp6` as one table, which is what [`holder`]
/// reads on the device.
pub fn listeners() -> String {
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reads `/proc` to find what holds `port`, the way `ss -tlnp` would.
///
/// `listeners` is `/proc/net/tcp` and `/proc/net/tcp6` concatenated; `cmdline`
/// answers the command line of a pid (`/proc/<pid>/cmdline`) or None. A socket
/// whose owner this process may not see is still a listener, so when no fd table
/// names it the answer is `AdbUnnamed` if an adbd is running and `Unknown` if
/// nothing at all can say.
pub fn holder(listeners: &str, port: u16, mut cmdline: impl FnMut(u32) -> Option<String>) -> Holder {
    let inodes = listening_inodes(listeners, port);
    if inodes.is_empty() {
        // A port that cannot be bound with no listener on it is permissions, not
        // a conflict, and saying "in use" then would send the player hunting.
        return Holder::Nothing;
    }
    let Some(processes) = process_list() else {
        return Holder::Unknown;
    };
    let mut adbd: Option<u32> = None;
    let mut other: Option<(u32, String)> = None;
    for (pid, command) in processes {
        let Some(line) = cmdline(pid) else {
            continue;
        };
        if !owns_any(&line, &inodes) {
            continue;
        }
        if is_adbd(&command, &line) {
            adbd = Some(pid);
            break;
        }
        // Prefer the first by pid, so the message does not change run to run.
        if other.as_ref().is_none_or(|(seen, _)| pid < *seen) {
            other = Some((pid, command));
        }
    }
    match (adbd, other) {
        (Some(pid), _) => Holder::Adb(pid),
        (None, Some((_, name))) => Holder::Process(name),
        // No process's fd table named the socket. On Android another app's fds
        // are closed to us, so this is where adbd usually lands; adbd running is
        // the only evidence available, and a wrong guess here only costs a
        // sentence that names both likely causes.
        (None, None) if adbd_running() => Holder::AdbUnnamed,
        (None, None) => Holder::Unknown,
    }
}

/// Whether this is adbd, which `adb reverse` gives the port: on the device it is
/// the process named `adbd` (sometimes started as `/system/bin/adbd`).
fn is_adbd(comm: &str, cmdline: &str) -> bool {
    let base = cmdline.split_whitespace().next().unwrap_or_default();
    let name = base.rsplit('/').next().unwrap_or_default();
    comm == "adbd" || name == "adbd" || cmdline.starts_with("adbd")
}

/// Whether a process named adbd is running at all: the fallback evidence when
/// Android hides the owning process's fds from this app.
fn adbd_running() -> bool {
    let Some(processes) = process_list() else {
        return false;
    };
    processes.iter().any(|(_, comm)| comm == "adbd")
}

fn process_list() -> Option<Vec<(u32, String)>> {
    let dir = std::fs::read_dir("/proc").ok()?;
    let mut out = Vec::new();
    for entry in dir.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        // `comm` is what the process calls itself, without arguments; where it
        // cannot be read (`hidepid`), the number stands in.
        let comm = std::fs::read_to_string(entry.path().join("comm"))
            .map(|c| c.trim().to_string())
            .unwrap_or_else(|_| pid.to_string());
        out.push((pid, comm));
    }
    Some(out)
}

/// Whether the command line's fd entries include one of the socket inodes.
fn owns_any(cmdline: &str, inodes: &[u64]) -> bool {
    if cmdline.is_empty() || inodes.is_empty() {
        return false;
    }
    for entry in std::fs::read_dir(cmdline).into_iter().flatten().flatten() {
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        let text = target.to_string_lossy();
        if let Some(inode) = text
            .strip_prefix("socket:[")
            .and_then(|rest| rest.strip_suffix(']'))
            .and_then(|n| n.parse::<u64>().ok())
        {
            if inodes.contains(&inode) {
                return true;
            }
        }
    }
    false
}

/// The socket inodes listening on `port`, from `/proc/net/tcp`-shaped tables.
///
/// A row is `sl local_address rem_address st tx_queue:rx_queue tr:tm->when
/// retrnsmt uid timeout inode`, the address as hex `IP:PORT` and the state `0A`
/// for LISTEN. The inode is the last column; that is read from the row itself
/// rather than from the header, because kernels differ in how many optional
/// columns (`uid`, `timeout`, `ref`, `pointer`, the TCP info) they print and the
/// header shows only some of them.
pub fn listening_inodes(listeners: &str, port: u16) -> Vec<u64> {
    let mut out = Vec::new();
    for line in listeners.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        // A header cell "inode" is not a number, so headers fall out here too.
        let Some(inode) = columns.last().and_then(|last| last.parse::<u64>().ok()) else {
            continue;
        };
        if columns.len() < 10 || columns[3] != "0A" {
            continue;
        }
        let Some(entry_port) = columns[1]
            .rsplit(':')
            .next()
            .and_then(|p| u16::from_str_radix(p, 16).ok())
        else {
            continue;
        };
        if entry_port == port {
            out.push(inode);
        }
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TCP: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:BCA3 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 4101
   1: 0100007F:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 4102
   2: 0100007F:BCA3 0100007F:D430 01 00000000:00000000 00:00000000 00000000     0        0 4103
";

    #[test]
    fn only_a_listener_on_the_port_counts() {
        // BCA3 is 48291: the listener's inode is the answer, the established socket
        // on the same port is a client, and port 22 is neither.
        assert_eq!(listening_inodes(TCP, 48291), vec![4101]);
        assert!(listening_inodes(TCP, 48290).is_empty());
    }

    #[test]
    fn a_second_table_is_read_as_more_rows_and_a_header_alone_names_nothing() {
        // `/proc/net/tcp6` follows `/proc/net/tcp` with its own header, and both
        // tables' listeners count.
        let v6 = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   0: 00000000000000000000000000000000:BCA3 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 4104";
        let both = format!("{TCP}{v6}");
        assert_eq!(listening_inodes(&both, 48291), vec![4101, 4104]);
        // A header names no listener, and neither does an empty table.
        assert!(listening_inodes(TCP.lines().next().unwrap(), 48291).is_empty());
        assert!(listening_inodes("", 48291).is_empty());
    }

    #[test]
    fn adbd_holding_the_port_gets_the_clear_instructions() {
        let holder = Holder::Adb(7);
        let line = explain("Address already in use", &holder);
        assert!(line.contains("adb reverse --remove tcp:48291 tcp:48291"), "{line}");
        assert!(line.contains("reconnects on its own"), "{line}");
    }

    #[test]
    fn another_program_is_named_by_its_command() {
        // The socket's owner is found through its own fd table, which no table
        // here can stand in for; the wording is what this covers.
        let line = explain("Address already in use", &Holder::Process("/data/app/proxy".into()));
        assert!(line.contains("'/data/app/proxy'"), "{line}");
        assert!(line.contains("Close it"), "{line}");
    }

    #[test]
    fn a_free_port_is_not_reported_as_a_conflict() {
        let holder = holder(TCP, 48290, |_| None);
        assert_eq!(holder, Holder::Nothing);
        let line = explain("Permission denied", &holder);
        assert!(!line.contains("adb"), "{line}");
        assert!(line.contains("Permission denied"), "{line}");
    }

    #[test]
    fn the_failure_is_always_reported_with_its_own_text() {
        let line = explain("Address already in use (os error 98)", &Holder::Unknown);
        assert!(line.contains("Address already in use (os error 98)"), "{line}");
        assert!(line.contains("48291"), "{line}");
    }

    #[test]
    fn adbd_is_recognised_by_its_path_too() {
        assert!(is_adbd("adbd", "adbd"));
        assert!(is_adbd("adbd", "/system/bin/adbd --root_seclabel=u:r:su:s0"));
        assert!(!is_adbd("ssh", "/usr/sbin/sshd -D"));
    }

    #[test]
    fn the_explanation_never_spans_lines() {
        // `chat_line` takes one line; a newline would split the warning in two.
        assert!(!explain("x", &Holder::Nothing).contains('\n'));
        assert!(!explain("x", &Holder::Adb(1)).contains('\n'));
    }
}
