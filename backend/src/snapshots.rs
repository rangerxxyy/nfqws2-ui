use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ARCHIVE_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_FILES: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRemoteSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub dir: String,
    #[serde(default = "default_keep_local")]
    pub keep_local: usize,
}

fn default_port() -> u16 {
    22
}
fn default_keep_local() -> usize {
    5
}

impl Default for SnapshotRemoteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: String::new(),
            port: 22,
            user: String::new(),
            dir: String::new(),
            keep_local: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSettings {
    #[serde(default = "default_max_count")]
    pub max_count: usize,
    #[serde(default = "default_max_days")]
    pub max_days: u64,
    #[serde(default)]
    pub remote: SnapshotRemoteSettings,
}

fn default_max_count() -> usize {
    50
}
fn default_max_days() -> u64 {
    30
}

impl Default for SnapshotSettings {
    fn default() -> Self {
        Self {
            max_count: 50,
            max_days: 30,
            remote: SnapshotRemoteSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotItem {
    pub id: String,
    pub ts: u64,
    pub reason: String,
    #[serde(default)]
    pub note: String,
    pub size: u64,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default = "default_true")]
    pub local: bool,
    #[serde(default)]
    pub remote: bool,
    #[serde(default)]
    pub pinned: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotStore {
    #[serde(default)]
    pub items: Vec<SnapshotItem>,
    #[serde(default)]
    pub last_daily_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRemoteState {
    pub tool: Option<String>,
    pub public_key: String,
    pub on: bool,
    pub pending: usize,
    pub stored: usize,
    pub last_ok: Option<u64>,
    pub error: Option<String>,
    pub setup_command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotOverview {
    pub items: Vec<SnapshotItem>,
    pub settings: SnapshotSettings,
    pub directory: String,
    pub total_bytes: u64,
    pub local_count: usize,
    pub remote: SnapshotRemoteState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotHistoryEntry {
    pub id: String,
    pub ts: u64,
    pub reason: String,
    pub note: String,
    pub changed_files: Vec<String>,
}

pub fn snapshot_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("snapshots")
}
pub fn settings_path(data_dir: &Path) -> PathBuf {
    snapshot_dir(data_dir).join("settings.json")
}
pub fn index_path(data_dir: &Path) -> PathBuf {
    snapshot_dir(data_dir).join("index.json")
}
pub fn remote_key_path(data_dir: &Path) -> PathBuf {
    snapshot_dir(data_dir).join("remote_key")
}
pub fn remote_pub_path(data_dir: &Path) -> PathBuf {
    snapshot_dir(data_dir).join("remote_key.pub")
}
pub fn remote_state_path(data_dir: &Path) -> PathBuf {
    snapshot_dir(data_dir).join("remote.json")
}
pub fn known_hosts_path(data_dir: &Path) -> PathBuf {
    snapshot_dir(data_dir).join("known_hosts")
}

pub fn load_settings(data_dir: &Path) -> SnapshotSettings {
    fs::read(settings_path(data_dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_settings(data_dir: &Path, settings: &SnapshotSettings) -> Result<(), String> {
    let mut value = settings.clone();
    value.max_count = value.max_count.clamp(5, 500);
    value.max_days = value.max_days.clamp(1, 3650);
    value.remote.port = value.remote.port.clamp(1, u16::MAX);
    value.remote.keep_local = value.remote.keep_local.clamp(1, 50);
    write_private_json(&settings_path(data_dir), &value)
}

pub fn load_store(data_dir: &Path) -> SnapshotStore {
    fs::read(index_path(data_dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_store(data_dir: &Path, store: &SnapshotStore) -> Result<(), String> {
    write_private_json(&index_path(data_dir), store)
}

fn write_private_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path.parent().ok_or("Путь снимков не содержит каталог")?;
    fs::create_dir_all(parent).map_err(|e| format!("Не удалось создать каталог снимков: {e}"))?;
    set_private_dir(parent)?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(&temp, bytes).map_err(|e| format!("Не удалось записать настройки снимков: {e}"))?;
    set_private_file(&temp)?;
    fs::rename(&temp, path).map_err(|e| format!("Не удалось сохранить настройки снимков: {e}"))
}

#[cfg(unix)]
fn set_private_dir(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}
#[cfg(not(unix))]
fn set_private_dir(_: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}
#[cfg(not(unix))]
fn set_private_file(_: &Path) -> Result<(), String> {
    Ok(())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn safe_archive_name(path: &str) -> bool {
    let components: Vec<_> = Path::new(path).components().collect();
    if components.is_empty()
        || components
            .iter()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return false;
    }
    let path = path.replace('\\', "/");
    path == "etc/nfqws2/nfqws2.conf"
        || path == "opt/etc/nfqws2/nfqws2.conf"
        || path == "nfqws2-ui/monitoring.json"
        || path == "nfqws2-ui/appearance.json"
        || path == "nfqws2-ui/snapshot-settings.json"
        || [
            "etc/nfqws2/lists/",
            "opt/etc/nfqws2/lists/",
            "etc/nfqws2/blobs/",
            "opt/etc/nfqws2/blobs/",
        ]
        .iter()
        .any(|prefix| path.strip_prefix(prefix).is_some_and(safe_archive_leaf))
}

fn safe_archive_leaf(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn data_archive_path(name: &str) -> &'static str {
    match name {
        "monitoring.json" => "nfqws2-ui/monitoring.json",
        _ => "nfqws2-ui/snapshot-settings.json",
    }
}

fn archive_id() -> String {
    let stamp = std::process::Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .filter(|value| {
            value.len() == 15
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'-')
        })
        .unwrap_or_else(|| format!("{}-000000", now_seconds()));
    format!("{stamp}-{:06}", std::process::id() % 1_000_000)
}

fn collect_snapshot_files(
    data_dir: &Path,
    config_path: &Path,
) -> Result<Vec<(PathBuf, String)>, String> {
    let config = config_path
        .canonicalize()
        .map_err(|e| format!("Не удалось найти конфигурацию: {e}"))?;
    let nfq_root = config
        .parent()
        .ok_or("Не удалось определить каталог nfqws2")?
        .to_path_buf();
    let config_name = config
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if config_name != "nfqws2.conf" {
        return Err("Активная конфигурация не является nfqws2.conf".into());
    }
    let prefix = if config.starts_with("/opt/") {
        "opt/etc/nfqws2"
    } else {
        "etc/nfqws2"
    };
    let mut files = vec![(config, format!("{prefix}/nfqws2.conf"))];
    for (dir_name, archive_dir, extension) in
        [("lists", "lists", Some("list")), ("blobs", "blobs", None)]
    {
        let dir = nfq_root.join(dir_name);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let meta = match entry.file_type() {
                Ok(meta) if meta.is_file() => meta,
                _ => continue,
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if !safe_archive_leaf(&name)
                || extension.is_some_and(|ext| !name.ends_with(&format!(".{ext}")))
            {
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            let path = entry.path().canonicalize().map_err(|e| e.to_string())?;
            if !path.starts_with(&dir) {
                continue;
            }
            files.push((path, format!("{prefix}/{archive_dir}/{name}")));
        }
    }
    for (name, archive_path) in [
        ("monitoring.json", data_archive_path("monitoring.json")),
        ("appearance.json", "nfqws2-ui/appearance.json"),
        ("settings.json", data_archive_path("snapshot-settings.json")),
    ] {
        let path = if name == "settings.json" {
            settings_path(data_dir)
        } else {
            data_dir.join(name)
        };
        if path.is_file() {
            files.push((path, archive_path.to_owned()));
        }
    }
    Ok(files)
}

pub fn create_snapshot(
    data_dir: &Path,
    config_path: &Path,
    reason: &str,
    note: &str,
    force: bool,
) -> Result<SnapshotItem, String> {
    if !matches!(
        reason,
        "перед изменением" | "ежедневный" | "вручную" | "перед восстановлением" | "из файла"
    ) {
        return Err("Недопустимая причина снимка".into());
    }
    let files = collect_snapshot_files(data_dir, config_path)?;
    let input_bytes = files.iter().try_fold(0u64, |total, (path, _)| {
        fs::metadata(path)
            .map(|meta| total.saturating_add(meta.len()))
            .map_err(|error| format!("Не удалось прочитать размер {}: {error}", path.display()))
    })?;
    if input_bytes > MAX_ARCHIVE_UNPACKED_BYTES {
        return Err("Размер данных снимка превышает 256 МиБ".into());
    }
    let mut hashes = BTreeMap::new();
    for (source, name) in &files {
        let content = fs::read(source)
            .map_err(|e| format!("Не удалось прочитать {}: {e}", source.display()))?;
        hashes.insert(name.clone(), hex::encode(Sha256::digest(content)));
    }
    let mut store = load_store(data_dir);
    if reason == "перед изменением"
        && let Some(previous) = store.items.iter().rev().find(|item| item.reason == reason)
        && now_seconds().saturating_sub(previous.ts) < 900
    {
        return Ok(previous.clone());
    }
    if !force && store.items.last().is_some_and(|last| last.files == hashes) {
        return Ok(store.items.last().unwrap().clone());
    }
    let directory = snapshot_dir(data_dir);
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    set_private_dir(&directory)?;
    let base_id = archive_id();
    let mut id = base_id.clone();
    let mut suffix = 2u32;
    while directory.join(format!("{id}.tar.gz")).exists() {
        id = format!("{base_id}-{suffix}");
        suffix = suffix.saturating_add(1);
    }
    let archive_path = directory.join(format!("{id}.tar.gz"));
    let temporary_path = directory.join(format!(".{id}.tmp"));
    let file =
        File::create(&temporary_path).map_err(|e| format!("Не удалось создать снимок: {e}"))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for (source, archive_name) in &files {
        builder
            .append_path_with_name(source, archive_name)
            .map_err(|e| {
                format!(
                    "Не удалось добавить файл {} в снимок: {e}",
                    source.display()
                )
            })?;
    }
    let encoder = builder.into_inner().map_err(|e| e.to_string())?;
    encoder
        .finish()
        .map_err(|e| format!("Не удалось завершить архив снимка: {e}"))?;
    let size = fs::metadata(&temporary_path)
        .map_err(|e| e.to_string())?
        .len();
    if size > MAX_ARCHIVE_BYTES as u64 {
        let _ = fs::remove_file(&temporary_path);
        return Err("Архив снимка превышает лимит 64 МиБ".into());
    }
    set_private_file(&temporary_path)?;
    fs::rename(&temporary_path, &archive_path)
        .map_err(|e| format!("Не удалось сохранить архив снимка: {e}"))?;
    store.items.push(SnapshotItem {
        id,
        ts: now_seconds(),
        reason: reason.to_owned(),
        note: note.chars().take(300).collect(),
        size,
        files: hashes,
        local: true,
        remote: false,
        pinned: false,
    });
    prune(data_dir, &mut store);
    save_store(data_dir, &store)?;
    Ok(store.items.last().cloned().unwrap())
}

fn prune(data_dir: &Path, store: &mut SnapshotStore) {
    let settings = load_settings(data_dir);
    let now = now_seconds();
    let max_age = settings.max_days.clamp(1, 3650).saturating_mul(86_400);
    let max_count = if settings.remote.enabled {
        settings.remote.keep_local.clamp(5, 50)
    } else {
        settings.max_count.clamp(5, 500)
    };
    let mut local_count = store.items.iter().filter(|item| item.local).count();
    let mut remove = Vec::new();
    for (index, item) in store.items.iter().enumerate() {
        if !item.local
            || item.pinned
            || local_count <= 5
            || (settings.remote.enabled && !item.remote)
        {
            continue;
        }
        let too_old = !settings.remote.enabled && now.saturating_sub(item.ts) > max_age;
        let over_count = local_count > max_count;
        if too_old || over_count {
            remove.push(index);
            local_count = local_count.saturating_sub(1);
        }
    }
    for index in remove.into_iter().rev() {
        let mut item = store.items.remove(index);
        let _ = fs::remove_file(snapshot_dir(data_dir).join(format!("{}.tar.gz", item.id)));
        if item.remote {
            item.local = false;
            store.items.insert(index, item);
        }
    }
}

pub fn apply_retention(data_dir: &Path) -> Result<(), String> {
    let mut store = load_store(data_dir);
    prune(data_dir, &mut store);
    save_store(data_dir, &store)
}

pub fn delete_local_snapshot(data_dir: &Path, id: &str) -> Result<(), String> {
    if !safe_id(id) {
        return Err("Неверный ID резервной копии".into());
    }
    let mut store = load_store(data_dir);
    let Some(index) = store.items.iter().position(|item| item.id == id) else {
        return Err("Снимок не найден".into());
    };
    if store.items[index].pinned {
        return Err("Сначала открепите снимок, затем удалите его".into());
    }
    if !store.items[index].local {
        return Err("Локальная копия снимка отсутствует".into());
    }
    fs::remove_file(snapshot_dir(data_dir).join(format!("{id}.tar.gz")))
        .map_err(|error| format!("Не удалось удалить архив снимка: {error}"))?;
    if store.items[index].remote {
        store.items[index].local = false;
    } else {
        store.items.remove(index);
    }
    save_store(data_dir, &store)
}

pub fn change_history(data_dir: &Path, limit: usize) -> Vec<SnapshotHistoryEntry> {
    let mut items = load_store(data_dir).items;
    items.sort_by_key(|item| item.ts);
    let mut previous = std::collections::BTreeMap::new();
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let mut changed_files: Vec<_> = item
            .files
            .iter()
            .filter(|(name, hash)| previous.get(*name) != Some(*hash))
            .map(|(name, _)| name.clone())
            .collect();
        changed_files.extend(
            previous
                .keys()
                .filter(|name| !item.files.contains_key(*name))
                .cloned(),
        );
        changed_files.sort();
        changed_files.dedup();
        previous = item.files.clone();
        entries.push(SnapshotHistoryEntry {
            id: item.id,
            ts: item.ts,
            reason: item.reason,
            note: item.note,
            changed_files,
        });
    }
    entries
        .into_iter()
        .rev()
        .take(limit.clamp(1, 500))
        .collect()
}

pub fn list_snapshots(data_dir: &Path) -> SnapshotOverview {
    let mut store = load_store(data_dir);
    store.items.retain(|item| safe_id(&item.id));
    for item in &mut store.items {
        item.local = snapshot_dir(data_dir)
            .join(format!("{}.tar.gz", item.id))
            .is_file();
    }
    let settings = load_settings(data_dir);
    let local_count = store.items.iter().filter(|item| item.local).count();
    let total_bytes = store
        .items
        .iter()
        .filter(|item| item.local)
        .map(|item| item.size)
        .sum();
    let remote = remote_overview(data_dir, &store, &settings);
    SnapshotOverview {
        items: store.items.into_iter().rev().collect(),
        settings,
        directory: snapshot_dir(data_dir).display().to_string(),
        total_bytes,
        local_count,
        remote,
    }
}

pub fn archive_bytes(data_dir: &Path, id: &str) -> Result<Vec<u8>, String> {
    if !safe_id(id) {
        return Err("Неверный ID резервной копии".into());
    }
    let path = snapshot_dir(data_dir).join(format!("{id}.tar.gz"));
    fs::read(path).map_err(|e| format!("Не удалось прочитать архив: {e}"))
}

pub fn validate_and_restore(
    data_dir: &Path,
    config_path: &Path,
    archive_bytes: &[u8],
) -> Result<Vec<String>, String> {
    if archive_bytes.is_empty() || archive_bytes.len() > MAX_ARCHIVE_BYTES {
        return Err("Архив пустой или превышает 64 МиБ".into());
    }
    let decoder = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = tar::Archive::new(decoder);
    let mut content = BTreeMap::<String, Vec<u8>>::new();
    let mut unpacked_bytes = 0u64;
    let entries = archive
        .entries()
        .map_err(|_| "Файл не является корректным tar.gz архивом")?;
    for entry in entries {
        let entry = entry.map_err(|_| "Не удалось прочитать содержимое архива")?;
        if content.len() >= MAX_ARCHIVE_FILES {
            return Err("В архиве слишком много файлов".into());
        }
        if !entry.header().entry_type().is_file() {
            return Err("Архив содержит ссылки или неразрешённый тип файла".into());
        }
        let name = entry
            .path()
            .map_err(|_| "В архиве есть некорректный путь")?
            .to_string_lossy()
            .replace('\\', "/");
        if !safe_archive_name(&name) {
            return Err(format!("В архиве лишний или небезопасный путь: {name}"));
        }
        let size = entry.size();
        if size > MAX_ARCHIVE_BYTES as u64 {
            return Err("Файл в архиве превышает лимит 64 МиБ".into());
        }
        unpacked_bytes = unpacked_bytes.saturating_add(size);
        if unpacked_bytes > MAX_ARCHIVE_UNPACKED_BYTES {
            return Err("Распакованный архив превышает лимит 256 МиБ".into());
        }
        let mut bytes = Vec::with_capacity(size as usize);
        entry
            .take(size + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Не удалось распаковать файл из архива")?;
        if bytes.len() as u64 != size {
            return Err("Размер файла в архиве не совпадает с заголовком".into());
        }
        content.insert(name, bytes);
    }
    if !content.keys().any(|name| name.ends_with("/nfqws2.conf")) {
        return Err("В архиве нет конфигурации nfqws2".into());
    }
    let current_config = config_path
        .canonicalize()
        .map_err(|e| format!("Не удалось найти текущий конфиг: {e}"))?;
    let root = current_config
        .parent()
        .ok_or("Не удалось определить каталог nfqws2")?;
    let data_root = data_dir
        .canonicalize()
        .map_err(|e| format!("Не удалось определить каталог данных панели: {e}"))?;
    let mut destinations = Vec::new();
    for (name, bytes) in &content {
        let destination = if name.ends_with("/nfqws2.conf") {
            current_config.clone()
        } else if name.contains("/lists/") {
            root.join("lists")
                .join(Path::new(name).file_name().unwrap())
        } else if name.contains("/blobs/") {
            root.join("blobs")
                .join(Path::new(name).file_name().unwrap())
        } else if name == "nfqws2-ui/monitoring.json" {
            data_dir.join("monitoring.json")
        } else if name == "nfqws2-ui/appearance.json" {
            data_dir.join("appearance.json")
        } else if name == "nfqws2-ui/snapshot-settings.json" {
            settings_path(data_dir)
        } else {
            continue;
        };
        destinations.push((name.clone(), destination, bytes.clone()));
    }
    let mut restored = Vec::new();
    for (name, destination, bytes) in destinations {
        if destination.exists()
            && fs::symlink_metadata(&destination).is_ok_and(|meta| meta.file_type().is_symlink())
        {
            return Err(format!(
                "Целевой файл является символической ссылкой: {}",
                destination.display()
            ));
        }
        let allowed_root = if destination.starts_with(root) {
            root
        } else {
            data_root.as_path()
        };
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("Не удалось проверить каталог {}: {e}", parent.display()))?;
            if !canonical_parent.starts_with(allowed_root) {
                return Err(format!(
                    "Каталог восстановления выходит за разрешённую папку: {}",
                    parent.display()
                ));
            }
        }
        let temporary = destination.with_extension(format!("restore.{}.tmp", std::process::id()));
        fs::write(&temporary, bytes)
            .map_err(|e| format!("Не удалось записать {}: {e}", destination.display()))?;
        set_private_file(&temporary)?;
        fs::rename(&temporary, &destination)
            .map_err(|e| format!("Не удалось восстановить {}: {e}", destination.display()))?;
        restored.push(name);
    }
    Ok(restored)
}

pub fn validate_archive(archive_bytes: &[u8]) -> Result<(), String> {
    if archive_bytes.is_empty() || archive_bytes.len() > MAX_ARCHIVE_BYTES {
        return Err("Архив пустой или превышает 64 МиБ".into());
    }
    let decoder = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = tar::Archive::new(decoder);
    let mut count = 0usize;
    let mut unpacked_bytes = 0u64;
    let mut config_found = false;
    for entry in archive
        .entries()
        .map_err(|_| "Файл не является корректным tar.gz архивом")?
    {
        let entry = entry.map_err(|_| "Не удалось прочитать содержимое архива")?;
        count += 1;
        if count > MAX_ARCHIVE_FILES {
            return Err("В архиве слишком много файлов".into());
        }
        if !entry.header().entry_type().is_file() {
            return Err("Архив содержит ссылки или неразрешённый тип файла".into());
        }
        let name = entry
            .path()
            .map_err(|_| "В архиве есть некорректный путь")?
            .to_string_lossy()
            .replace('\\', "/");
        if !safe_archive_name(&name) {
            return Err(format!("В архиве лишний или небезопасный путь: {name}"));
        }
        if name.ends_with("/nfqws2.conf") {
            config_found = true;
        }
        let size = entry.size();
        if size > MAX_ARCHIVE_BYTES as u64 {
            return Err("Файл в архиве превышает лимит 64 МиБ".into());
        }
        unpacked_bytes = unpacked_bytes.saturating_add(size);
        if unpacked_bytes > MAX_ARCHIVE_UNPACKED_BYTES {
            return Err("Распакованный архив превышает лимит 256 МиБ".into());
        }
        let mut sink = std::io::sink();
        std::io::copy(&mut entry.take(size + 1), &mut sink)
            .map_err(|_| "Не удалось проверить содержимое архива")?;
    }
    if !config_found {
        return Err("В архиве нет конфигурации nfqws2".into());
    }
    Ok(())
}

pub fn is_due_daily(data_dir: &Path) -> bool {
    let Ok(output) = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M")
        .output()
    else {
        return false;
    };
    let clock = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let Some((date, time)) = clock.split_once(' ') else {
        return false;
    };
    if time != "04:05" {
        return false;
    }
    load_store(data_dir).last_daily_date.as_deref() != Some(date)
}

pub fn mark_daily_snapshot(data_dir: &Path) {
    let mut store = load_store(data_dir);
    let date = std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
    store.last_daily_date = date;
    let _ = save_store(data_dir, &store);
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

pub fn valid_snapshot_id(id: &str) -> bool {
    safe_id(id)
}

fn remote_overview(
    data_dir: &Path,
    store: &SnapshotStore,
    settings: &SnapshotSettings,
) -> SnapshotRemoteState {
    let public_key = fs::read_to_string(remote_pub_path(data_dir))
        .unwrap_or_default()
        .trim()
        .to_owned();
    let state: serde_json::Value = fs::read(remote_state_path(data_dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let tool = find_ssh_tool();
    let pending = store
        .items
        .iter()
        .filter(|item| item.local && !item.remote)
        .count();
    let stored = store.items.iter().filter(|item| item.remote).count();
    let on = settings.remote.enabled
        && !settings.remote.host.is_empty()
        && !settings.remote.user.is_empty()
        && !public_key.is_empty();
    let setup_command = if !public_key.is_empty() && !settings.remote.dir.is_empty() {
        remote_setup_command(&settings.remote.dir, &public_key)
    } else {
        String::new()
    };
    SnapshotRemoteState {
        tool,
        public_key,
        on,
        pending,
        stored,
        last_ok: state.get("ok").and_then(serde_json::Value::as_u64),
        error: state
            .get("error")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        setup_command,
    }
}

fn find_ssh_tool() -> Option<String> {
    for tool in ["dbclient", "ssh"] {
        if std::process::Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .output()
            .is_ok_and(|result| result.status.success())
        {
            return Some(tool.to_owned());
        }
    }
    None
}

fn remote_setup_command(dir: &str, public_key: &str) -> String {
    let script = r#"#!/bin/sh
dir=$1
set -f
set -- $SSH_ORIGINAL_COMMAND
set +f
op=$1 name=$2
case "$op" in ping|list) ;; put|get)
  case "$name" in
    index.json|[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9]-[0-9]*.tar.gz) ;;
    *) echo 'недопустимое имя файла' >&2; exit 2 ;;
  esac ;;
  *) echo 'недопустимая команда' >&2; exit 2 ;;
esac
mkdir -p "$dir" || exit 1
umask 077
case "$op" in
  ping) echo ok ;;
  list) cd "$dir" && for f in [0-9]*.tar.gz; do [ -f "$f" ] && echo "$f $(wc -c < "$f")"; done ;;
  get) [ -f "$dir/$name" ] && cat "$dir/$name" ;;
  put) if [ "$name" != index.json ] && [ -e "$dir/$name" ]; then cat >/dev/null; echo 'снимок уже существует' >&2; exit 3; fi; cat > "$dir/.$name.part" && mv -f "$dir/.$name.part" "$dir/$name" ;;
esac
"#;
    let line = format!("command=\"sh .nfqws-ui-store.sh {dir}\",restrict {public_key}");
    format!(
        "umask 077 && mkdir -p ~/.ssh && cat > ~/.nfqws-ui-store.sh <<'NFQWSUI'\n{script}NFQWSUI\ngrep -qF '{}' ~/.ssh/authorized_keys 2>/dev/null || echo '{}' >> ~/.ssh/authorized_keys",
        public_key.replace('\'', "'\\''"),
        line.replace('\'', "'\\''")
    )
}

#[cfg(test)]
mod manual_operations_tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn item(id: &str, ts: u64, files: &[(&str, &str)]) -> SnapshotItem {
        SnapshotItem {
            id: id.into(),
            ts,
            reason: "вручную".into(),
            note: String::new(),
            size: 3,
            files: files
                .iter()
                .map(|(name, hash)| ((*name).into(), (*hash).into()))
                .collect::<BTreeMap<_, _>>(),
            local: true,
            remote: false,
            pinned: false,
        }
    }

    #[test]
    fn deletes_only_local_unpinned_archives_and_keeps_remote_copy_metadata() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(snapshot_dir(dir.path())).unwrap();
        fs::write(snapshot_dir(dir.path()).join("1.tar.gz"), b"one").unwrap();
        fs::write(snapshot_dir(dir.path()).join("2.tar.gz"), b"two").unwrap();
        let mut remote = item("2", 2, &[("nfqws2.conf", "abc")]);
        remote.remote = true;
        save_store(
            dir.path(),
            &SnapshotStore {
                items: vec![item("1", 1, &[]), remote],
                last_daily_date: None,
            },
        )
        .unwrap();

        delete_local_snapshot(dir.path(), "1").unwrap();
        delete_local_snapshot(dir.path(), "2").unwrap();
        assert!(!snapshot_dir(dir.path()).join("1.tar.gz").exists());
        let remaining = load_store(dir.path());
        assert_eq!(remaining.items.len(), 1);
        assert!(!remaining.items[0].local);
        assert!(remaining.items[0].remote);
        assert_eq!(remaining.items[0].files.len(), 1);
    }

    #[test]
    fn refuses_deleting_pinned_snapshots_and_lists_hash_changes_newest_first() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(snapshot_dir(dir.path())).unwrap();
        fs::write(snapshot_dir(dir.path()).join("3.tar.gz"), b"one").unwrap();
        let mut pinned = item("3", 3, &[]);
        pinned.pinned = true;
        save_store(
            dir.path(),
            &SnapshotStore {
                items: vec![
                    item("old", 1, &[("a", "1")]),
                    item("new", 2, &[("a", "2"), ("b", "1")]),
                    pinned,
                ],
                last_daily_date: None,
            },
        )
        .unwrap();
        assert!(delete_local_snapshot(dir.path(), "3").is_err());
        let history = change_history(dir.path(), 20);
        assert_eq!(history[1].changed_files, vec!["a", "b"]);
        assert_eq!(history[2].changed_files, vec!["a"]);
    }
}
