//! Secure clipboard with timed clear.

use arboard::Clipboard;
use secrecy::{ExposeSecret, Secret};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use zeroize::Zeroize;

const DEFAULT_CLEAR_SECS: u64 = 30;

struct Pending {
    generation: u64,
    secret: Secret<String>,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Copy a secret to the OS clipboard and schedule a clear if still unchanged.
pub fn copy_secret(secret: &str) -> Result<(), String> {
    copy_secret_with_timeout(secret, DEFAULT_CLEAR_SECS)
}

pub fn copy_secret_with_timeout(secret: &str, clear_after_secs: u64) -> Result<(), String> {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let owned = Secret::new(secret.to_string());
    {
        let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
        clipboard
            .set_text(owned.expose_secret().clone())
            .map_err(|e| format!("Failed to set clipboard: {e}"))?;
    }
    {
        let mut pending = PENDING.lock().map_err(|_| "Clipboard state is unavailable".to_string())?;
        *pending = Some(Pending {
            generation,
            secret: owned,
        });
    }

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(clear_after_secs));
        clear_generation(generation);
    });
    Ok(())
}

fn clear_generation(generation: u64) {
    let expected = {
        let pending = PENDING.lock().ok();
        let Some(pending) = pending else {
            return;
        };
        match pending.as_ref() {
            Some(item) if item.generation == generation => Some(item.secret.expose_secret().clone()),
            _ => None,
        }
    };
    let Some(expected) = expected else {
        return;
    };
    clear_clipboard_if_equals(&expected);
    let mut wipe = expected;
    wipe.zeroize();
    if let Ok(mut pending) = PENDING.lock() {
        if pending.as_ref().is_some_and(|item| item.generation == generation) {
            *pending = None;
        }
    }
}

/// Force-clear our pending clipboard secret (for example on lock).
pub fn clear_pending() {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let expected = {
        let mut pending = match PENDING.lock() {
            Ok(pending) => pending,
            Err(_) => return,
        };
        pending.take().map(|item| item.secret.expose_secret().clone())
    };
    let _ = generation;
    if let Some(expected) = expected {
        clear_clipboard_if_equals(&expected);
        let mut wipe = expected;
        wipe.zeroize();
    }
}

fn clear_clipboard_if_equals(expected: &str) {
    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(current) = clipboard.get_text() {
            if current == expected {
                let _ = clipboard.set_text(String::new());
            }
        }
    }
}
