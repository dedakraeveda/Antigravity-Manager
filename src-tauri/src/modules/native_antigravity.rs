use crate::models::Account;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const MANAGER_DATA_DIR: &str = ".antigravity_tools";
const NATIVE_APP_DATA_DIR: &str = "antigravity";
const LAST_SWITCH_FILE: &str = "native_antigravity_switch.json";

#[derive(Debug, Clone, Serialize)]
pub struct NativeAntigravityState {
    pub storage_root: String,
    pub storage_root_exists: bool,
    pub conversations_dir_exists: bool,
    pub brain_dir_exists: bool,
    pub antigravity_state_exists: bool,
}

#[derive(Debug, Serialize)]
struct NativeSwitchMarker<'a> {
    email: &'a str,
    switched_at: i64,
    storage_root: String,
    credential_target: &'static str,
}

pub fn storage_root() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("failed_to_get_home_dir")?;
    Ok(home.join(".gemini").join(NATIVE_APP_DATA_DIR))
}

pub fn manager_data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("failed_to_get_home_dir")?;
    let dir = home.join(MANAGER_DATA_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("failed_to_create_manager_data_dir: {}", e))?;
    }
    Ok(dir)
}

pub fn inspect_state() -> Result<NativeAntigravityState, String> {
    let root = storage_root()?;
    Ok(NativeAntigravityState {
        storage_root: root.to_string_lossy().to_string(),
        storage_root_exists: root.exists(),
        conversations_dir_exists: root.join("conversations").exists(),
        brain_dir_exists: root.join("brain").exists(),
        antigravity_state_exists: root.join("antigravity_state.pbtxt").exists(),
    })
}

/// Native Antigravity 2.x is not a VS Code-style app and normally has no
/// User/globalStorage/storage.json. Its agent/session state lives under
/// ~/.gemini/antigravity, while OAuth tokens are read from the OS credential
/// store. This hook records what we switched and validates that the native
/// storage root is discoverable; it intentionally does not rewrite conversation
/// databases or Antigravity protobuf state.
pub fn after_keyring_switch(account: &Account) -> Result<(), String> {
    let state = inspect_state()?;
    if !state.storage_root_exists {
        crate::modules::logger::log_warn(&format!(
            "[Native Antigravity] storage root not found at {}; the app may create it on first launch",
            state.storage_root
        ));
    } else {
        crate::modules::logger::log_info(&format!(
            "[Native Antigravity] storage root detected: {} (conversations={}, brain={}, state={})",
            state.storage_root,
            state.conversations_dir_exists,
            state.brain_dir_exists,
            state.antigravity_state_exists
        ));
    }

    let marker = NativeSwitchMarker {
        email: &account.email,
        switched_at: chrono::Utc::now().timestamp(),
        storage_root: state.storage_root,
        credential_target: "gemini:antigravity",
    };
    let marker_path = manager_data_dir()?.join(LAST_SWITCH_FILE);
    let json = serde_json::to_string_pretty(&marker)
        .map_err(|e| format!("failed_to_serialize_native_switch_marker: {}", e))?;
    fs::write(&marker_path, json)
        .map_err(|e| format!("failed_to_write_native_switch_marker: {}", e))?;
    crate::modules::logger::log_info(&format!(
        "[Native Antigravity] last switch marker written: {:?}",
        marker_path
    ));
    Ok(())
}
