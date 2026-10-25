//! Named configuration profiles stored separately from the active nfqws2 config.
//! They intentionally store complete validated config text because nfqws2's
//! argv profiles are runtime-derived and cannot be safely edited in isolation.

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const MAX_PROFILES: usize = 16;
const MAX_PROFILE_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigProfile {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigProfileSummary {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
    pub bytes: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigProfileStore {
    #[serde(default)]
    profiles: Vec<ConfigProfile>,
}

fn store_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("profiles.json")
}

fn load(data_dir: &Path) -> Result<ConfigProfileStore, String> {
    let path = store_path(data_dir);
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("Не удалось прочитать сохранённые профили: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ConfigProfileStore::default())
        }
        Err(error) => Err(format!("Не удалось прочитать сохранённые профили: {error}")),
    }
}

fn save(data_dir: &Path, store: &ConfigProfileStore) -> Result<(), String> {
    fs::create_dir_all(data_dir)
        .map_err(|error| format!("Не удалось сохранить хранилище профилей: {error}"))?;
    let path = store_path(data_dir);
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec(store).map_err(|error| error.to_string())?;
    fs::write(&temp, bytes)
        .map_err(|error| format!("Не удалось сохранить хранилище профилей: {error}"))?;
    set_private_file(&temp)
        .map_err(|error| format!("Не удалось сохранить хранилище профилей: {error}"))?;
    fs::rename(&temp, &path)
        .map_err(|error| format!("Не удалось сохранить хранилище профилей: {error}"))
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err("Название профиля должно содержать от 1 до 64 печатных символов".into());
    }
    Ok(name.to_owned())
}

fn validate_content(content: &str) -> Result<(), String> {
    if content.trim().is_empty() || content.len() > MAX_PROFILE_BYTES {
        return Err("Размер конфигурации профиля должен быть от 1 байта до 512 КиБ".into());
    }
    Ok(())
}

pub fn list(data_dir: &Path) -> Result<Vec<ConfigProfileSummary>, String> {
    let mut profiles = load(data_dir)?.profiles;
    profiles.sort_by_key(|profile| profile.name.to_lowercase());
    Ok(profiles
        .into_iter()
        .map(|profile| ConfigProfileSummary {
            id: profile.id,
            name: profile.name,
            updated_at: profile.updated_at,
            bytes: profile.content.len(),
        })
        .collect())
}

pub fn create(
    data_dir: &Path,
    name: &str,
    content: &str,
    now: u64,
) -> Result<ConfigProfileSummary, String> {
    let name = validate_name(name)?;
    validate_content(content)?;
    let mut store = load(data_dir)?;
    if store.profiles.len() >= MAX_PROFILES {
        return Err("Можно сохранить не больше 16 профилей".into());
    }
    if store
        .profiles
        .iter()
        .any(|profile| profile.name.to_lowercase() == name.to_lowercase())
    {
        return Err("Профиль с таким названием уже существует".into());
    }
    let profile = ConfigProfile {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name,
        updated_at: now,
        content: content.to_owned(),
    };
    let summary = ConfigProfileSummary {
        id: profile.id.clone(),
        name: profile.name.clone(),
        updated_at: now,
        bytes: content.len(),
    };
    store.profiles.push(profile);
    save(data_dir, &store)?;
    Ok(summary)
}

pub fn rename(
    data_dir: &Path,
    id: &str,
    name: &str,
    now: u64,
) -> Result<ConfigProfileSummary, String> {
    let name = validate_name(name)?;
    let mut store = load(data_dir)?;
    if store
        .profiles
        .iter()
        .any(|profile| profile.id != id && profile.name.to_lowercase() == name.to_lowercase())
    {
        return Err("Профиль с таким названием уже существует".into());
    }
    let profile = store
        .profiles
        .iter_mut()
        .find(|profile| profile.id == id)
        .ok_or("Профиль не найден")?;
    profile.name = name;
    profile.updated_at = now;
    let summary = ConfigProfileSummary {
        id: profile.id.clone(),
        name: profile.name.clone(),
        updated_at: now,
        bytes: profile.content.len(),
    };
    save(data_dir, &store)?;
    Ok(summary)
}

pub fn delete(data_dir: &Path, id: &str) -> Result<(), String> {
    let mut store = load(data_dir)?;
    let before = store.profiles.len();
    store.profiles.retain(|profile| profile.id != id);
    if store.profiles.len() == before {
        return Err("Профиль не найден".into());
    }
    save(data_dir, &store)
}

pub fn get(data_dir: &Path, id: &str) -> Result<ConfigProfile, String> {
    load(data_dir)?
        .profiles
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| "Профиль не найден".into())
}

pub fn update(
    data_dir: &Path,
    id: &str,
    name: &str,
    content: &str,
    now: u64,
) -> Result<ConfigProfileSummary, String> {
    let name = validate_name(name)?;
    validate_content(content)?;
    let mut store = load(data_dir)?;
    if store
        .profiles
        .iter()
        .any(|profile| profile.id != id && profile.name.to_lowercase() == name.to_lowercase())
    {
        return Err("Профиль с таким названием уже существует".into());
    }
    let profile = store
        .profiles
        .iter_mut()
        .find(|profile| profile.id == id)
        .ok_or("Профиль не найден")?;
    profile.name = name;
    profile.content = content.to_owned();
    profile.updated_at = now;
    let summary = ConfigProfileSummary {
        id: profile.id.clone(),
        name: profile.name.clone(),
        updated_at: now,
        bytes: profile.content.len(),
    };
    save(data_dir, &store)?;
    Ok(summary)
}

pub fn content(data_dir: &Path, id: &str) -> Result<String, String> {
    load(data_dir)?
        .profiles
        .into_iter()
        .find(|profile| profile.id == id)
        .map(|profile| profile.content)
        .ok_or_else(|| "Профиль не найден".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_lists_renames_and_deletes_named_configs() {
        let dir = tempdir().unwrap();
        let created = create(dir.path(), "Дом", "NFQWS_ARGS=\"--filter-tcp=443\"\n", 10).unwrap();
        assert_eq!(list(dir.path()).unwrap()[0].name, "Дом");
        assert_eq!(
            content(dir.path(), &created.id).unwrap(),
            "NFQWS_ARGS=\"--filter-tcp=443\"\n"
        );
        let renamed = rename(dir.path(), &created.id, "Работа", 11).unwrap();
        assert_eq!(renamed.updated_at, 11);
        delete(dir.path(), &created.id).unwrap();
        assert!(list(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn updates_saved_profile_name_and_content_without_changing_its_id() {
        let dir = tempdir().unwrap();
        let created = create(dir.path(), "Дом", "NFQWS_ARGS=\"--filter-tcp=443\"\n", 10).unwrap();
        let updated = update(
            dir.path(),
            &created.id,
            "Домашняя сеть",
            "NFQWS_ARGS=\"--filter-tcp=80,443\"\n",
            11,
        )
        .unwrap();

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "Домашняя сеть");
        assert_eq!(updated.updated_at, 11);
        assert_eq!(
            content(dir.path(), &created.id).unwrap(),
            "NFQWS_ARGS=\"--filter-tcp=80,443\"\n"
        );
    }

    #[test]
    fn rejects_empty_or_duplicate_profile_updates_without_changing_saved_data() {
        let dir = tempdir().unwrap();
        let first = create(dir.path(), "Дом", "before", 10).unwrap();
        let second = create(dir.path(), "Работа", "other", 10).unwrap();
        assert!(update(dir.path(), &first.id, "Дом", "  ", 11).is_err());
        assert!(update(dir.path(), &first.id, &second.name, "after", 11).is_err());
        assert_eq!(content(dir.path(), &first.id).unwrap(), "before");
    }

    #[test]
    fn rejects_duplicate_names_and_oversized_or_empty_configs() {
        let dir = tempdir().unwrap();
        create(dir.path(), "Дом", "config", 10).unwrap();
        assert!(create(dir.path(), "дом", "other", 11).is_err());
        assert!(create(dir.path(), "Пустой", "  ", 11).is_err());
        assert!(
            create(
                dir.path(),
                "Большой",
                &"x".repeat(MAX_PROFILE_BYTES + 1),
                11
            )
            .is_err()
        );
    }

    #[test]
    fn refuses_to_overwrite_a_corrupt_profile_store() {
        let dir = tempdir().unwrap();
        let path = store_path(dir.path());
        fs::write(&path, b"{not valid json").unwrap();

        assert!(create(dir.path(), "Дом", "config", 10).is_err());
        assert!(list(dir.path()).is_err());
        assert_eq!(fs::read(path).unwrap(), b"{not valid json");
    }
}
