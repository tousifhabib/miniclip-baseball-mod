//! Says why the game could not start, to a player who started it as an app
//! and so has no terminal to read it in.

/// What the alert is headed.
const HEADING: &str = "The game could not start";

/// Puts up a system alert saying `message`, and waits for it to be
/// dismissed. If that fails too there is nothing more to be done about it.
pub fn alert(message: &str) {
    put_up(message);
}

#[cfg(target_os = "macos")]
fn put_up(message: &str) {
    // AppleScript strings escape only the backslash and the double quote.
    let quoted = message.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("display alert \"{HEADING}\" message \"{quoted}\" as critical");
    let _ = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .status();
}

#[cfg(windows)]
fn put_up(message: &str) {
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(HEADING)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Anywhere else the game is only ever started from a terminal, which has
/// been told already.
#[cfg(not(any(target_os = "macos", windows)))]
fn put_up(_message: &str) {}
