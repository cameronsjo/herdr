use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

const HISTORY_FILE: &str = "palette-history.json";
pub(crate) const MAX_RECENT_COMMANDS: usize = 8;

#[derive(Debug, Default, Deserialize, Serialize)]
struct PaletteHistoryStore {
    recent_command_ids: Vec<String>,
}

pub(crate) fn store_path() -> PathBuf {
    crate::config::state_dir().join(HISTORY_FILE)
}

#[cfg(not(test))]
pub(crate) fn load() -> io::Result<Vec<String>> {
    load_from_path(&store_path())
}

#[cfg(not(test))]
pub(crate) fn save(recent_command_ids: &[String]) -> io::Result<()> {
    save_to_path(&store_path(), recent_command_ids)
}

pub(crate) fn remember(recent_command_ids: &mut Vec<String>, command_id: String) {
    recent_command_ids.retain(|existing| existing != &command_id);
    recent_command_ids.insert(0, command_id);
    recent_command_ids.truncate(MAX_RECENT_COMMANDS);
}

fn load_from_path(path: &Path) -> io::Result<Vec<String>> {
    let content: String = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let store: PaletteHistoryStore = serde_json::from_str(&content).map_err(io::Error::other)?;
    Ok(normalize_command_ids(store.recent_command_ids))
}

fn normalize_command_ids(command_ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for command_id in command_ids {
        if command_id.is_empty() || seen.contains(&command_id) {
            continue;
        }
        seen.insert(command_id.clone());
        normalized.push(command_id);
        if normalized.len() == MAX_RECENT_COMMANDS {
            break;
        }
    }
    normalized
}

fn save_to_path(path: &Path, recent_command_ids: &[String]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let store: PaletteHistoryStore = PaletteHistoryStore {
        recent_command_ids: normalize_command_ids(recent_command_ids.iter().cloned()),
    };
    let json: String = serde_json::to_string_pretty(&store).map_err(io::Error::other)?;
    let temporary_path: PathBuf = path.with_extension(format!("json.tmp.{}", std::process::id()));
    fs::write(&temporary_path, json)?;
    #[cfg(windows)]
    if path.exists() {
        if let Err(error) = fs::remove_file(path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "herdr-palette-history-{name}-{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn remember_moves_an_existing_command_to_the_front_and_caps_history() {
        let mut recent: Vec<String> = (0..MAX_RECENT_COMMANDS)
            .map(|index| format!("core:command-{index}"))
            .collect();

        remember(&mut recent, "core:command-4".to_string());
        assert_eq!(recent.first().map(String::as_str), Some("core:command-4"));
        assert_eq!(recent.len(), MAX_RECENT_COMMANDS);

        remember(&mut recent, "core:new-command".to_string());
        assert_eq!(recent.first().map(String::as_str), Some("core:new-command"));
        assert_eq!(recent.len(), MAX_RECENT_COMMANDS);
        assert!(!recent.iter().any(|id| id == "core:command-7"));
    }

    #[test]
    fn history_round_trip_normalizes_duplicates_empty_ids_and_overflow() {
        let path: PathBuf = temporary_path("round-trip");
        let ids: Vec<String> = vec![
            "core:new-tab".to_string(),
            "".to_string(),
            "core:new-tab".to_string(),
        ]
        .into_iter()
        .chain((0..MAX_RECENT_COMMANDS).map(|index| format!("core:extra-{index}")))
        .collect();
        save_to_path(&path, &ids).expect("history should save");

        let loaded: Vec<String> = load_from_path(&path).expect("history should load");
        assert_eq!(loaded.len(), MAX_RECENT_COMMANDS);
        assert_eq!(loaded.first().map(String::as_str), Some("core:new-tab"));
        assert_eq!(loaded.get(1).map(String::as_str), Some("core:extra-0"));

        save_to_path(&path, &["core:replacement".to_string()])
            .expect("existing history should be replaceable");
        assert_eq!(
            load_from_path(&path).expect("replacement history should load"),
            vec!["core:replacement"]
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn malformed_history_is_reported_instead_of_silently_discarded() {
        let path: PathBuf = temporary_path("malformed");
        fs::write(&path, "not json").expect("fixture should write");
        let error: io::Error = load_from_path(&path).expect_err("malformed history should fail");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        let _ = fs::remove_file(path);
    }
}
