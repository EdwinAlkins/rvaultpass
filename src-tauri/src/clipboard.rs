//! Secure clipboard with timed clear

use arboard::Clipboard;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static PENDING_CLEAR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

const DEFAULT_CLEAR_SECS: u64 = 30;

/// Copy a secret to the OS clipboard and schedule a clear if still unchanged.
pub fn copy_secret(secret: &str) -> Result<(), String> {
    copy_secret_with_timeout(secret, DEFAULT_CLEAR_SECS)
}

pub fn copy_secret_with_timeout(secret: &str, clear_after_secs: u64) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard unavailable: {}", e))?;
    clipboard
        .set_text(secret.to_string())
        .map_err(|e| format!("Failed to set clipboard: {}", e))?;

    {
        let mut pending = PENDING_CLEAR.lock().unwrap();
        *pending = Some(secret.to_string());
    }

    let expected = secret.to_string();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(clear_after_secs));
        clear_if_matches(&expected);
    });

    Ok(())
}

/// Clear clipboard only if it still contains our last copied secret.
pub fn clear_if_matches(expected: &str) {
    let should_clear = {
        let pending = PENDING_CLEAR.lock().unwrap();
        pending.as_deref() == Some(expected)
    };

    if !should_clear {
        return;
    }

    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(current) = clipboard.get_text() {
            if current == expected {
                let _ = clipboard.set_text(String::new());
            }
        }
    }

    let mut pending = PENDING_CLEAR.lock().unwrap();
    if pending.as_deref() == Some(expected) {
        *pending = None;
    }
}

/// Force-clear our pending clipboard secret (e.g. on lock).
pub fn clear_pending() {
    let expected = {
        let mut pending = PENDING_CLEAR.lock().unwrap();
        pending.take()
    };

    if let Some(expected) = expected {
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(current) = clipboard.get_text() {
                if current == expected {
                    let _ = clipboard.set_text(String::new());
                }
            }
        }
    }
}
