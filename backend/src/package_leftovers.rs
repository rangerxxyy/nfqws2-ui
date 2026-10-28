use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLeftover {
    pub name: String,
    pub base: String,
    pub size_bytes: u64,
    pub modified_at: Option<u64>,
    pub base_exists: bool,
    pub same_as_base: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLeftoverComparison {
    pub name: String,
    pub base: String,
    pub original_content: String,
    pub candidate_content: String,
}

const MAX_COMPARISON_BYTES: u64 = 512 * 1024;

pub fn list_package_leftovers(
    config: &Path,
    lists_dir: &Path,
) -> Result<Vec<PackageLeftover>, String> {
    let config = regular_file(config).ok_or_else(|| "Конфигурация nfqws2 не найдена".to_owned())?;
    let config_dir = config
        .parent()
        .ok_or_else(|| "Не удалось определить каталог конфигурации nfqws2".to_owned())?;
    let mut items = Vec::new();

    let config_entries = fs::read_dir(config_dir)
        .map_err(|error| format!("Не удалось прочитать каталог конфигурации nfqws2: {error}"))?;
    for entry in config_entries {
        let entry = entry
            .map_err(|error| format!("Не удалось прочитать имя файла конфигурации: {error}"))?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some((base, _)) = split_package_suffix(file_name) else {
            continue;
        };
        if !safe_candidate_name(file_name) {
            continue;
        }
        push_candidate(&mut items, &config_dir.join(file_name), file_name, base);
    }

    match fs::symlink_metadata(lists_dir) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            let canonical_lists = lists_dir
                .canonicalize()
                .map_err(|error| format!("Не удалось проверить каталог списков: {error}"))?;
            if !canonical_lists.starts_with(config_dir)
                || canonical_lists.parent() != Some(config_dir)
            {
                return Err("Каталог списков находится вне установки nfqws2".into());
            }
            let entries = fs::read_dir(&canonical_lists)
                .map_err(|error| format!("Не удалось прочитать каталог списков: {error}"))?;
            for entry in entries {
                let entry = entry
                    .map_err(|error| format!("Не удалось прочитать имя файла списка: {error}"))?;
                let file_name = entry.file_name();
                let Some(file_name) = file_name.to_str() else {
                    continue;
                };
                let Some((base, _)) = split_package_suffix(file_name) else {
                    continue;
                };
                if !safe_candidate_name(file_name) {
                    continue;
                }
                let candidate = canonical_lists.join(file_name);
                push_candidate(&mut items, &candidate, &format!("lists/{file_name}"), base);
            }
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Не удалось проверить каталог списков: {error}")),
    }

    items.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(items)
}

pub fn delete_package_leftovers(
    config: &Path,
    lists_dir: &Path,
    names: &[String],
) -> Result<usize, String> {
    let discovered = list_package_leftovers(config, lists_dir)?;
    let config_dir = regular_file(config)
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .ok_or_else(|| "Конфигурация nfqws2 не найдена".to_owned())?;
    let mut deleted = 0;
    let mut seen = std::collections::HashSet::new();

    for item in discovered {
        if !names.iter().any(|name| name == &item.name) || !seen.insert(item.name.clone()) {
            continue;
        }
        let (directory, file_name) = item
            .name
            .strip_prefix("lists/")
            .map(|name| (lists_dir, name))
            .unwrap_or((config_dir.as_path(), item.name.as_str()));
        if !safe_candidate_name(file_name) {
            continue;
        }
        let path = directory.join(file_name);
        let Some(file) = regular_file(&path) else {
            continue;
        };
        if file.parent() != directory.canonicalize().ok().as_deref() {
            continue;
        }
        if fs::remove_file(file).is_ok() {
            deleted += 1;
        }
    }
    Ok(deleted)
}

pub fn compare_package_leftover(
    config: &Path,
    lists_dir: &Path,
    name: &str,
) -> Result<PackageLeftoverComparison, String> {
    let item = list_package_leftovers(config, lists_dir)?
        .into_iter()
        .find(|item| item.name == name)
        .ok_or_else(|| "Файл больше не найден среди остатков обновления".to_owned())?;
    let is_supported_base = item.name == "nfqws2.conf-opkg"
        || item.name == "nfqws2.conf.apk-new"
        || (item.name.starts_with("lists/") && item.base.ends_with(".list"));
    if !is_supported_base || !item.base_exists || item.same_as_base != Some(false) {
        return Err("Сравнение доступно только для изменённого файла с исходной версией".into());
    }

    let config_path =
        regular_file(config).ok_or_else(|| "Конфигурация nfqws2 не найдена".to_owned())?;
    let config_dir = config_path
        .parent()
        .ok_or_else(|| "Не удалось определить каталог конфигурации nfqws2".to_owned())?;
    let (directory, candidate_name, base_name) =
        if let Some(candidate_name) = item.name.strip_prefix("lists/") {
            let canonical_lists = lists_dir
                .canonicalize()
                .map_err(|_| "Каталог списков nfqws2 не найден".to_owned())?;
            if canonical_lists.parent() != Some(config_dir) {
                return Err("Каталог списков находится вне установки nfqws2".into());
            }
            (
                canonical_lists,
                candidate_name,
                item.base
                    .rsplit('/')
                    .next()
                    .unwrap_or(&item.base)
                    .to_owned(),
            )
        } else {
            (
                config_dir.to_path_buf(),
                item.name.as_str(),
                item.base.clone(),
            )
        };
    if !safe_candidate_name(candidate_name) || !safe_candidate_name(&format!("{base_name}-opkg")) {
        return Err("Недопустимое имя файла для сравнения".into());
    }
    let candidate_path = regular_file(&directory.join(candidate_name))
        .ok_or_else(|| "Файл-кандидат больше не доступен".to_owned())?;
    let base_path = regular_file(&directory.join(&base_name))
        .ok_or_else(|| "Исходный файл больше не доступен".to_owned())?;
    let original_content = read_comparison_file(&base_path)?;
    let candidate_content = read_comparison_file(&candidate_path)?;
    Ok(PackageLeftoverComparison {
        name: item.name,
        base: item.base,
        original_content,
        candidate_content,
    })
}

fn read_comparison_file(path: &Path) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|error| format!("Не удалось прочитать файл для сравнения: {error}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_COMPARISON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Не удалось прочитать файл для сравнения: {error}"))?;
    if bytes.len() as u64 > MAX_COMPARISON_BYTES {
        return Err("Файл слишком большой для просмотра сравнения (лимит 512 КиБ)".into());
    }
    String::from_utf8(bytes)
        .map_err(|_| "Файл содержит данные не в UTF-8 и не может быть показан как текст".into())
}

fn push_candidate(items: &mut Vec<PackageLeftover>, path: &Path, name: &str, base: &str) {
    let Some(path) = regular_file(path) else {
        return;
    };
    let Ok(metadata) = fs::metadata(&path) else {
        return;
    };
    let Some(parent) = path.parent() else { return };
    let base_path = parent.join(base.rsplit('/').next().unwrap_or(base));
    let base_file = regular_file(&base_path);
    let base_exists = base_file.is_some();
    let same_as_base = base_file
        .as_ref()
        .and_then(|base_path| files_equal(&path, base_path));
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    items.push(PackageLeftover {
        name: name.to_owned(),
        base: base.to_owned(),
        size_bytes: metadata.len(),
        modified_at,
        base_exists,
        same_as_base,
    });
}

fn files_equal(left: &Path, right: &Path) -> Option<bool> {
    fn digest(path: &Path) -> Option<[u8; 32]> {
        let mut file = File::open(path).ok()?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let count = file.read(&mut buffer).ok()?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Some(hasher.finalize().into())
    }
    digest(left)
        .zip(digest(right))
        .map(|(left, right)| left == right)
}

fn regular_file(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return None;
    }
    let canonical = path.canonicalize().ok()?;
    (canonical.parent() == path.parent()?.canonicalize().ok().as_deref()).then_some(canonical)
}

fn split_package_suffix(name: &str) -> Option<(&str, &str)> {
    ["-opkg", ".apk-new"]
        .into_iter()
        .find_map(|suffix| name.strip_suffix(suffix).map(|base| (base, suffix)))
}

fn safe_candidate_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && split_package_suffix(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    fn install_tree(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let config_dir = root.join("etc/nfqws2");
        let lists_dir = config_dir.join("lists");
        fs::create_dir_all(&lists_dir).unwrap();
        fs::write(config_dir.join("nfqws2.conf"), b"active config").unwrap();
        (config_dir.join("nfqws2.conf"), lists_dir)
    }

    #[test]
    fn lists_every_package_manager_suffix_in_the_config_and_lists_directories() {
        let root = tempdir().unwrap();
        let (config, lists) = install_tree(root.path());
        fs::write(config.with_file_name("nfqws2.conf-opkg"), b"new config").unwrap();
        fs::write(
            config.with_file_name("nfqws2.conf.apk-new"),
            b"new apk config",
        )
        .unwrap();
        fs::write(config.with_file_name("other.conf-opkg"), b"unrelated").unwrap();
        fs::write(lists.join("user.list"), b"active list").unwrap();
        fs::write(lists.join("user.list-opkg"), b"new list").unwrap();
        fs::write(lists.join("mirror.list"), b"same bytes").unwrap();
        fs::write(lists.join("mirror.list-opkg"), b"same bytes").unwrap();
        fs::write(lists.join("ipset.list.apk-new"), b"new ip list").unwrap();
        fs::write(lists.join("user.list-old"), b"old file").unwrap();
        fs::write(lists.join("strange name.list-opkg"), b"package copy").unwrap();
        fs::create_dir(lists.join("folder.list-opkg")).unwrap();

        let items = list_package_leftovers(&config, &lists).unwrap();
        let names = items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                "lists/ipset.list.apk-new",
                "lists/mirror.list-opkg",
                "lists/strange name.list-opkg",
                "lists/user.list-opkg",
                "nfqws2.conf-opkg",
                "nfqws2.conf.apk-new",
                "other.conf-opkg",
            ]
        );
        assert!(
            items
                .iter()
                .find(|item| item.name == "lists/user.list-opkg")
                .unwrap()
                .base_exists
        );
        assert_eq!(
            items
                .iter()
                .find(|item| item.name == "lists/mirror.list-opkg")
                .unwrap()
                .same_as_base,
            Some(true)
        );
        assert!(items.iter().all(|item| item.size_bytes > 0));
        let serialized = serde_json::to_string(&items).unwrap();
        assert!(!serialized.contains("active config"));
        assert!(!serialized.contains("new config"));
    }

    #[test]
    fn deletion_removes_only_exactly_discovered_candidates_and_keeps_base_files() {
        let root = tempdir().unwrap();
        let (config, lists) = install_tree(root.path());
        fs::write(config.with_file_name("nfqws2.conf-opkg"), b"new config").unwrap();
        fs::write(lists.join("user.list"), b"active list").unwrap();
        fs::write(lists.join("user.list.apk-new"), b"new list").unwrap();
        fs::write(root.path().join("outside.list-opkg"), b"outside").unwrap();

        let deleted = delete_package_leftovers(
            &config,
            &lists,
            &[
                "nfqws2.conf-opkg".into(),
                "lists/user.list.apk-new".into(),
                "../../outside.list-opkg".into(),
                "nfqws2.conf".into(),
                "nfqws2.conf-opkg".into(),
            ],
        )
        .unwrap();

        assert_eq!(deleted, 2);
        assert!(!config.with_file_name("nfqws2.conf-opkg").exists());
        assert!(!lists.join("user.list.apk-new").exists());
        assert_eq!(fs::read(config).unwrap(), b"active config");
        assert_eq!(fs::read(lists.join("user.list")).unwrap(), b"active list");
        assert_eq!(
            fs::read(root.path().join("outside.list-opkg")).unwrap(),
            b"outside"
        );
    }

    #[test]
    fn comparison_returns_contents_only_for_a_changed_discovered_file_with_a_base() {
        let root = tempdir().unwrap();
        let (config, lists) = install_tree(root.path());
        fs::write(lists.join("user.list"), b"original\n").unwrap();
        fs::write(lists.join("user.list-opkg"), b"updated\n").unwrap();

        let comparison = compare_package_leftover(&config, &lists, "lists/user.list-opkg").unwrap();

        assert_eq!(comparison.base, "user.list");
        assert_eq!(comparison.original_content, "original\n");
        assert_eq!(comparison.candidate_content, "updated\n");
    }

    #[test]
    fn comparison_rejects_unknown_unchanged_missing_base_and_unsafe_names() {
        let root = tempdir().unwrap();
        let (config, lists) = install_tree(root.path());
        fs::write(lists.join("same.list"), b"same\n").unwrap();
        fs::write(lists.join("same.list-opkg"), b"same\n").unwrap();
        fs::write(lists.join("orphan.list-opkg"), b"candidate\n").unwrap();

        assert!(compare_package_leftover(&config, &lists, "lists/same.list-opkg").is_err());
        assert!(compare_package_leftover(&config, &lists, "lists/orphan.list-opkg").is_err());
        assert!(compare_package_leftover(&config, &lists, "../../outside.list-opkg").is_err());
        assert!(compare_package_leftover(&config, &lists, "other.list-opkg").is_err());
    }
}
