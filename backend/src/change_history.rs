use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use sha2::Digest;

pub const MAX_TRACKED_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_HISTORY_EVENTS: usize = 500;
const MAX_HISTORY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TRACKED_TOTAL_BYTES: u64 = 60 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeHistoryEntry {
    pub id: String,
    #[serde(default)]
    pub batch: String,
    #[serde(default)]
    pub undo_of: Option<String>,
    pub ts: u64,
    pub file: String,
    pub path: String,
    pub source: String,
    pub note: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub added_lines: usize,
    pub removed_lines: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeHistoryDiff {
    pub from: Option<String>,
    pub to: Option<String>,
    pub current: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeHistoryOverview {
    pub items: Vec<ChangeHistoryEntry>,
    pub initial_at: Option<u64>,
    pub can_restore_initial: bool,
    pub can_undo_last: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Store {
    initial_at: Option<u64>,
    initial: BTreeMap<String, String>,
    current: BTreeMap<String, String>,
    entries: Vec<ChangeHistoryEntry>,
}

fn root(data_dir: &Path) -> PathBuf {
    data_dir.join("change-history")
}

fn store_path(data_dir: &Path) -> PathBuf {
    root(data_dir).join("index.json")
}

fn load(data_dir: &Path) -> Result<Store, String> {
    let path = store_path(data_dir);
    if !path.is_file() {
        return Ok(Store::default());
    }
    let bytes = fs::read(path).map_err(|error| format!("Не удалось прочитать историю: {error}"))?;
    let mut store: Store = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Повреждён индекс истории: {error}"))?;
    // Existing history indexes predate batch IDs. Treat each old event as a
    // one-event batch so it remains available to the undo action.
    for entry in &mut store.entries {
        if entry.batch.is_empty() {
            entry.batch.clone_from(&entry.id);
        }
    }
    Ok(store)
}

fn save(data_dir: &Path, store: &Store) -> Result<(), String> {
    let directory = root(data_dir);
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec(store).map_err(|error| error.to_string())?;
    let temporary = directory.join("index.json.tmp");
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, store_path(data_dir)).map_err(|error| error.to_string())
}

fn object_path(data_dir: &Path, digest: &str) -> PathBuf {
    root(data_dir).join("objects").join(format!("{digest}.gz"))
}

fn put_object(data_dir: &Path, bytes: &[u8]) -> Result<String, String> {
    let digest = hex::encode(sha2::Sha256::digest(bytes));
    let path = object_path(data_dir, &digest);
    if path.is_file() {
        return Ok(digest);
    }
    let directory = path.parent().expect("object has parent");
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|error| error.to_string())?;
    let compressed = encoder.finish().map_err(|error| error.to_string())?;
    fs::write(&path, compressed).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    Ok(digest)
}

fn get_object(data_dir: &Path, digest: Option<&str>) -> Result<Option<String>, String> {
    let Some(digest) = digest else {
        return Ok(None);
    };
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Некорректная ссылка на версию файла".into());
    }
    let file = fs::File::open(object_path(data_dir, digest))
        .map_err(|error| format!("Версия файла недоступна: {error}"))?;
    let decoder = GzDecoder::new(file);
    let mut bytes = Vec::new();
    decoder
        .take(MAX_TRACKED_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Не удалось распаковать версию файла: {error}"))?;
    if bytes.len() as u64 > MAX_TRACKED_FILE_BYTES {
        return Err("Версия файла превышает лимит истории".into());
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "История содержит файл не в UTF-8".into())
}

fn current_files(config_path: &Path) -> Result<BTreeMap<String, (String, Vec<u8>)>, String> {
    let config = config_path
        .canonicalize()
        .map_err(|error| format!("Не удалось найти конфигурацию: {error}"))?;
    if config.file_name().and_then(|name| name.to_str()) != Some("nfqws2.conf") {
        return Err("Активный конфиг не является nfqws2.conf".into());
    }
    let mut files = BTreeMap::new();
    add_file(&mut files, &config)?;
    let lists_dir = config
        .parent()
        .ok_or("Не удалось определить каталог конфигурации")?
        .join("lists");
    if let Ok(canonical_lists) = lists_dir.canonicalize()
        && canonical_lists == lists_dir
        && let Ok(entries) = fs::read_dir(&lists_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("list") {
                add_file(&mut files, &path)?;
            }
        }
    }
    if files.len() > 512 {
        return Err("В истории одновременно может быть не более 512 файлов".into());
    }
    let total_bytes = files.values().fold(0u64, |sum, (_, bytes)| {
        sum.saturating_add(bytes.len() as u64)
    });
    if total_bytes > MAX_TRACKED_TOTAL_BYTES {
        return Err("Общий размер файлов истории превышает 64 МиБ".into());
    }
    Ok(files)
}

fn add_file(files: &mut BTreeMap<String, (String, Vec<u8>)>, path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_TRACKED_FILE_BYTES {
        return Ok(());
    }
    let canonical = path.canonicalize().map_err(|error| error.to_string())?;
    let name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Недопустимое имя файла истории")?;
    if name != "nfqws2.conf" && !name.ends_with(".list") {
        return Ok(());
    }
    let bytes = fs::read(&canonical).map_err(|error| error.to_string())?;
    if std::str::from_utf8(&bytes).is_err() {
        return Ok(());
    }
    files.insert(
        canonical.to_string_lossy().into_owned(),
        (name.to_owned(), bytes),
    );
    Ok(())
}

fn line_delta(old: Option<&str>, new: Option<&str>) -> (usize, usize) {
    fn counts(content: Option<&str>) -> HashMap<&str, usize> {
        let mut result = HashMap::new();
        for line in content
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            *result.entry(line).or_insert(0) += 1;
        }
        result
    }
    let old = counts(old);
    let new = counts(new);
    let added = new
        .iter()
        .map(|(line, count)| count.saturating_sub(*old.get(line).unwrap_or(&0)))
        .sum();
    let removed = old
        .iter()
        .map(|(line, count)| count.saturating_sub(*new.get(line).unwrap_or(&0)))
        .sum();
    (added, removed)
}

fn classify_external_source(path: &Path, fallback: &str) -> String {
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|_| SystemTime::now());
    for suffix in ["-opkg", ".apk-new"] {
        if let Ok(package) = fs::metadata(parent.join(format!("{file}{suffix}")))
            && let Ok(package_time) = package.modified()
            && modified
                .duration_since(package_time)
                .or_else(|_| package_time.duration_since(modified))
                .is_ok_and(|duration| duration.as_secs() <= 30)
        {
            return "обновление пакета".into();
        }
    }
    fallback.to_owned()
}

pub fn scan(
    data_dir: &Path,
    config_path: &Path,
    source: &str,
    note: &str,
) -> Result<Vec<ChangeHistoryEntry>, String> {
    if !matches!(
        source,
        "интерфейс" | "вне интерфейса" | "обновление пакета" | "откат"
    ) {
        return Err("Недопустимый источник истории".into());
    }
    let files = current_files(config_path)?;
    let mut store = load(data_dir)?;
    let mut current = BTreeMap::new();
    for (path, (_, bytes)) in &files {
        current.insert(path.clone(), put_object(data_dir, bytes)?);
    }
    if store.initial_at.is_none() {
        store.initial_at = Some(now());
        store.initial = current.clone();
        store.current = current;
        enforce_storage_limit(data_dir, &mut store)?;
        save(data_dir, &store)?;
        return Ok(Vec::new());
    }

    let mut changed = Vec::new();
    let batch = uuid::Uuid::new_v4().simple().to_string();
    let paths: std::collections::BTreeSet<_> = store
        .current
        .keys()
        .chain(current.keys())
        .cloned()
        .collect();
    for path in paths {
        let from = store.current.get(&path).cloned();
        let to = current.get(&path).cloned();
        if from == to {
            continue;
        }
        let before = get_object(data_dir, from.as_deref())?;
        let after = get_object(data_dir, to.as_deref())?;
        let (added_lines, removed_lines) = line_delta(before.as_deref(), after.as_deref());
        let file = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("файл")
            .to_owned();
        let entry_source = if source == "вне интерфейса" {
            classify_external_source(Path::new(&path), source)
        } else {
            source.to_owned()
        };
        let entry = ChangeHistoryEntry {
            id: uuid::Uuid::new_v4().simple().to_string(),
            batch: batch.clone(),
            undo_of: None,
            ts: now(),
            file,
            path,
            source: entry_source,
            note: note.chars().take(300).collect(),
            from,
            to,
            added_lines,
            removed_lines,
        };
        store.entries.push(entry.clone());
        changed.push(entry);
    }
    store.current = current;
    if store.entries.len() > MAX_HISTORY_EVENTS {
        let remove = store.entries.len() - MAX_HISTORY_EVENTS;
        store.entries.drain(..remove);
    }
    enforce_storage_limit(data_dir, &mut store)?;
    save(data_dir, &store)?;
    Ok(changed)
}

fn enforce_storage_limit(data_dir: &Path, store: &mut Store) -> Result<(), String> {
    let directory = root(data_dir).join("objects");
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(&directory) {
        for entry in entries.flatten() {
            total = total.saturating_add(entry.metadata().map(|meta| meta.len()).unwrap_or(0));
        }
    }
    while total > MAX_HISTORY_BYTES && !store.entries.is_empty() {
        store.entries.remove(0);
        let retained: std::collections::HashSet<_> = store
            .initial
            .values()
            .chain(store.current.values())
            .chain(
                store
                    .entries
                    .iter()
                    .flat_map(|entry| entry.from.iter().chain(entry.to.iter())),
            )
            .cloned()
            .collect();
        if let Ok(entries) = fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let digest = entry
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".gz")
                    .to_owned();
                if !retained.contains(&digest) {
                    total =
                        total.saturating_sub(entry.metadata().map(|meta| meta.len()).unwrap_or(0));
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
        if total > MAX_HISTORY_BYTES && store.entries.is_empty() {
            return Err("История превышает предел хранения 64 МиБ".into());
        }
    }
    Ok(())
}

pub fn list(data_dir: &Path, limit: usize) -> Result<ChangeHistoryOverview, String> {
    let store = load(data_dir)?;
    let can_undo_last = latest_undoable_batch(&store.entries).is_some();
    let mut items = store.entries;
    items.reverse();
    items.truncate(limit.clamp(1, MAX_HISTORY_EVENTS));
    Ok(ChangeHistoryOverview {
        can_restore_initial: store.initial_at.is_some() && store.initial != store.current,
        can_undo_last,
        initial_at: store.initial_at,
        items,
    })
}

fn latest_undoable_batch(entries: &[ChangeHistoryEntry]) -> Option<Vec<ChangeHistoryEntry>> {
    let undone: std::collections::HashSet<_> = entries
        .iter()
        .filter_map(|entry| entry.undo_of.as_deref())
        .collect();
    let batch = entries.iter().rev().find_map(|entry| {
        (entry.source == "интерфейс" && !undone.contains(entry.batch.as_str()))
            .then_some(entry.batch.as_str())
    })?;
    Some(
        entries
            .iter()
            .filter(|entry| entry.batch == batch && entry.source == "интерфейс")
            .cloned()
            .collect(),
    )
}

pub fn undoable_batch(data_dir: &Path) -> Result<Option<Vec<ChangeHistoryEntry>>, String> {
    Ok(latest_undoable_batch(&load(data_dir)?.entries))
}

pub fn mark_undo_of_latest(data_dir: &Path, path: &Path, batch: &str) -> Result<(), String> {
    let path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let mut store = load(data_dir)?;
    let entry = store
        .entries
        .iter_mut()
        .rev()
        .find(|entry| entry.path == path && entry.source == "откат")
        .ok_or("Не удалось найти событие отмены в истории")?;
    entry.undo_of = Some(batch.to_owned());
    save(data_dir, &store)
}

pub fn diff(data_dir: &Path, id: &str) -> Result<ChangeHistoryDiff, String> {
    let store = load(data_dir)?;
    let entry = store
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .ok_or("Нет такой записи истории")?;
    let current = fs::read(&entry.path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
    Ok(ChangeHistoryDiff {
        from: get_object(data_dir, entry.from.as_deref())?,
        to: get_object(data_dir, entry.to.as_deref())?,
        current,
    })
}

pub fn version_before(
    data_dir: &Path,
    id: &str,
) -> Result<(ChangeHistoryEntry, Option<String>), String> {
    let store = load(data_dir)?;
    let entry = store
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
        .ok_or("Нет такой записи истории")?;
    let content = get_object(data_dir, entry.from.as_deref())?;
    Ok((entry, content))
}

pub fn initial_versions(data_dir: &Path) -> Result<(u64, BTreeMap<String, String>), String> {
    let store = load(data_dir)?;
    Ok((
        store
            .initial_at
            .ok_or("Исходное состояние истории ещё не создано")?,
        store.initial,
    ))
}

pub fn current_versions(data_dir: &Path) -> Result<BTreeMap<String, String>, String> {
    Ok(load(data_dir)?.current)
}

pub fn mark_latest(data_dir: &Path, path: &Path, source: &str, note: &str) -> Result<(), String> {
    let path = path.to_string_lossy();
    let mut store = load(data_dir)?;
    let entry = store
        .entries
        .iter_mut()
        .rev()
        .find(|entry| entry.path == path)
        .ok_or("Не удалось найти только что записанное событие истории")?;
    entry.source = source.to_owned();
    entry.note = note.chars().take(300).collect();
    save(data_dir, &store)
}

pub fn initial_archive(data_dir: &Path, config_path: &Path) -> Result<Vec<u8>, String> {
    let store = load(data_dir)?;
    if store.initial.is_empty() {
        return Err("Исходное состояние истории ещё не создано".into());
    }
    let config = config_path
        .canonicalize()
        .map_err(|error| format!("Не удалось найти конфигурацию: {error}"))?;
    let root = config
        .parent()
        .ok_or("Не удалось определить каталог конфигурации")?;
    let prefix = if config.starts_with("/opt/") {
        "opt/etc/nfqws2"
    } else {
        "etc/nfqws2"
    };
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut archive = tar::Builder::new(&mut encoder);
        let mut config_found = false;
        for (path, digest) in store.initial {
            let path = PathBuf::from(path);
            let safe = path == config
                || (path.extension().and_then(|ext| ext.to_str()) == Some("list")
                    && path.parent() == Some(root.join("lists").as_path()));
            if !safe {
                continue;
            }
            let relative = if path == config {
                config_found = true;
                format!("{prefix}/nfqws2.conf")
            } else {
                format!(
                    "{prefix}/lists/{}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default()
                )
            };
            let content = get_object(data_dir, Some(&digest))?
                .ok_or("В исходном состоянии отсутствует содержимое файла")?;
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o600);
            header.set_cksum();
            archive
                .append_data(&mut header, relative, content.as_bytes())
                .map_err(|error| format!("Не удалось собрать исходное состояние: {error}"))?;
        }
        if !config_found {
            return Err("Исходная история не содержит конфигурацию".into());
        }
        archive
            .finish()
            .map_err(|error| format!("Не удалось завершить архив исходного состояния: {error}"))?;
    }
    encoder
        .finish()
        .map_err(|error| format!("Не удалось сжать исходное состояние: {error}"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn scan_records_file_content_changes_and_diff() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("nfqws2");
        let lists = config_dir.join("lists");
        fs::create_dir_all(&lists).unwrap();
        let config = config_dir.join("nfqws2.conf");
        fs::write(&config, "--filter-tcp=443\n").unwrap();
        let list = lists.join("user.list");
        fs::write(&list, "one.example\n").unwrap();
        let data = temp.path().join("data");

        super::scan(&data, &config, "интерфейс", "начало истории").unwrap();
        fs::write(&config, "--filter-tcp=443\n--filter-udp=443\n").unwrap();
        fs::write(&list, "one.example\ntwo.example\n").unwrap();
        let changed = super::scan(&data, &config, "интерфейс", "сохранение").unwrap();

        assert_eq!(changed.len(), 2);
        let config_event = changed
            .iter()
            .find(|entry| entry.file == "nfqws2.conf")
            .unwrap();
        let diff = super::diff(&data, &config_event.id).unwrap();
        assert_eq!(diff.from.as_deref(), Some("--filter-tcp=443\n"));
        assert_eq!(
            diff.to.as_deref(),
            Some("--filter-tcp=443\n--filter-udp=443\n")
        );
        assert_eq!(config_event.added_lines, 1);
    }

    #[test]
    fn undoing_a_ui_batch_marks_it_undone_and_enables_the_next_batch() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("nfqws2");
        fs::create_dir_all(config_dir.join("lists")).unwrap();
        let config = config_dir.join("nfqws2.conf");
        fs::write(&config, "one\n").unwrap();
        let data = temp.path().join("data");
        super::scan(&data, &config, "интерфейс", "baseline").unwrap();

        fs::write(&config, "two\n").unwrap();
        let first_batch = super::scan(&data, &config, "интерфейс", "изменение 1").unwrap();
        assert_eq!(first_batch.len(), 1);
        assert!(!first_batch[0].batch.is_empty());
        assert!(super::list(&data, 20).unwrap().can_undo_last);

        fs::write(&config, "one\n").unwrap();
        let rollback = super::scan(&data, &config, "откат", "отмена изменения 1").unwrap();
        assert_eq!(rollback.len(), 1);
        assert_eq!(rollback[0].source, "откат");
        assert_eq!(rollback[0].path, first_batch[0].path);
        super::mark_undo_of_latest(&data, &config, &first_batch[0].batch).unwrap();
        assert!(!super::list(&data, 20).unwrap().can_undo_last);

        fs::write(&config, "three\n").unwrap();
        super::scan(&data, &config, "интерфейс", "изменение 2").unwrap();
        assert!(super::list(&data, 20).unwrap().can_undo_last);
    }

    #[test]
    fn tracks_created_and_deleted_lists_and_limits_history() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("nfqws2");
        let lists = config_dir.join("lists");
        fs::create_dir_all(&lists).unwrap();
        let config = config_dir.join("nfqws2.conf");
        fs::write(&config, "config\n").unwrap();
        let data = temp.path().join("data");
        super::scan(&data, &config, "интерфейс", "baseline").unwrap();

        let created = lists.join("added.list");
        fs::write(&created, "new.example\n").unwrap();
        let added = super::scan(&data, &config, "вне интерфейса", "внешняя правка").unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].from, None);
        assert_eq!(added[0].to.as_ref().unwrap().len(), 64);

        fs::remove_file(created).unwrap();
        let removed = super::scan(&data, &config, "интерфейс", "удаление").unwrap();
        assert_eq!(removed.len(), 1);
        assert!(removed[0].from.is_some());
        assert_eq!(removed[0].to, None);
        assert!(!super::list(&data, 20).unwrap().can_restore_initial);
    }

    #[test]
    fn ignores_non_list_files_and_rejects_invalid_sources() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join("nfqws2");
        fs::create_dir_all(config_dir.join("lists")).unwrap();
        let config = config_dir.join("nfqws2.conf");
        fs::write(&config, "config\n").unwrap();
        fs::write(config_dir.join("lists/notes.txt"), "ignored\n").unwrap();
        let data = temp.path().join("data");
        assert!(super::scan(&data, &config, "unknown", "").is_err());
        super::scan(&data, &config, "интерфейс", "baseline").unwrap();
        fs::write(config_dir.join("lists/notes.txt"), "changed\n").unwrap();
        assert!(
            super::scan(&data, &config, "вне интерфейса", "scan")
                .unwrap()
                .is_empty()
        );
    }
}
