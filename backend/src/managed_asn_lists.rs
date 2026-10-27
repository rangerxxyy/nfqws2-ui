use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Write, net::IpAddr, path::Path};

pub const STORE_FILE: &str = "managed-asn-lists.json";
const MAX_STORE_BYTES: usize = 64 * 1024;
const MAX_LIST_BYTES: usize = 512 * 1024;

fn previous_content_path(data_dir: &Path, asn: u32) -> std::path::PathBuf {
    data_dir.join(format!("managed-asn-{asn}.previous"))
}

fn save_previous_content(data_dir: &Path, asn: u32, content: &[u8]) -> Result<(), String> {
    if content.len() > MAX_LIST_BYTES {
        return Err("Прежний ASN-список превышает лимит 512 КиБ".into());
    }
    fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    let destination = previous_content_path(data_dir, asn);
    let temporary = data_dir.join(format!(".managed-asn-{asn}.{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("Не удалось создать резервный ASN-список: {error}"))?;
    if let Err(error) = file.write_all(content).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Не удалось записать резервный ASN-список: {error}"));
    }
    drop(file);
    fs::rename(&temporary, &destination).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Не удалось сохранить резервный ASN-список: {error}")
    })?;
    if let Ok(directory) = fs::File::open(data_dir) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn load_previous_content(
    data_dir: &Path,
    asn: u32,
    expected_sha256: &str,
) -> Result<Vec<u8>, String> {
    let path = previous_content_path(data_dir, asn);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("Не удалось найти резервный ASN-список: {error}"))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_LIST_BYTES as u64 {
        return Err("Резервный ASN-список имеет недопустимый тип или размер".into());
    }
    let content = fs::read(path)
        .map_err(|error| format!("Не удалось прочитать резервный ASN-список: {error}"))?;
    if sha256(&content) != expected_sha256 {
        return Err("Контрольная сумма резервного ASN-списка не совпала".into());
    }
    Ok(content)
}

fn remove_previous_content(data_dir: &Path, asn: u32) {
    let _ = fs::remove_file(previous_content_path(data_dir, asn));
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAsnList {
    pub asn: u32,
    pub list_id: String,
    pub prefix_count: usize,
    pub content_sha256: String,
    #[serde(default)]
    pub last_attempt_date: Option<String>,
    #[serde(default)]
    pub last_success_date: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub pending_update: Option<PendingAsnListUpdate>,
}

impl ManagedAsnList {
    pub fn new(asn: u32, list_id: &str, prefix_count: usize, content: &[u8]) -> Self {
        Self {
            asn,
            list_id: list_id.to_owned(),
            prefix_count,
            content_sha256: sha256(content),
            last_attempt_date: None,
            last_success_date: None,
            last_error: None,
            paused: false,
            pending_update: None,
        }
    }

    pub fn is_due(&self, local_date: &str) -> bool {
        self.pending_update.is_some()
            || (!self.paused
                && !local_date.is_empty()
                && self.last_attempt_date.as_deref() != Some(local_date))
    }

    pub fn content_matches(&self, content: &[u8]) -> bool {
        self.content_sha256 == sha256(content)
    }

    pub fn stage_update(&mut self, prefix_count: usize, content: &[u8], previous: &[u8]) {
        self.pending_update = Some(PendingAsnListUpdate {
            prefix_count,
            content_sha256: sha256(content),
            previous_content_sha256: Some(sha256(previous)),
        });
    }

    pub fn finish_update(&mut self, local_date: &str) -> bool {
        let Some(pending) = self.pending_update.take() else {
            return false;
        };
        self.prefix_count = pending.prefix_count;
        self.content_sha256 = pending.content_sha256;
        self.last_success_date = Some(local_date.to_owned());
        self.last_error = None;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAsnListUpdate {
    pub prefix_count: usize,
    pub content_sha256: String,
    #[serde(default)]
    pub previous_content_sha256: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAsnListStore {
    #[serde(default)]
    pub entries: Vec<ManagedAsnList>,
}

impl ManagedAsnListStore {
    pub fn load(data_dir: &Path) -> Self {
        fs::read(data_dir.join(STORE_FILE))
            .ok()
            .filter(|bytes| bytes.len() <= MAX_STORE_BYTES)
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, data_dir: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err("Список управляемых ASN превышает лимит размера".into());
        }
        fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
        let destination = data_dir.join(STORE_FILE);
        let temporary = data_dir.join(format!(".{STORE_FILE}.{}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Не удалось создать временный файл ASN: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| {
                let _ = fs::remove_file(&temporary);
                format!("Не удалось записать состояние ASN: {error}")
            })?;
        drop(file);
        #[cfg(unix)]
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!("Не удалось защитить состояние ASN: {error}")
        })?;
        fs::rename(&temporary, &destination).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!("Не удалось сохранить состояние ASN: {error}")
        })
    }

    pub fn register(&mut self, entry: ManagedAsnList) {
        self.entries.retain(|current| current.asn != entry.asn);
        self.entries.push(entry);
        self.entries.sort_by_key(|current| current.asn);
    }

    pub fn remove(&mut self, asn: u32) {
        self.entries.retain(|current| current.asn != asn);
    }
}

pub fn accept_prefix_count(previous_count: usize, new_count: usize) -> bool {
    new_count > 0 && new_count.saturating_mul(3) >= previous_count
}

pub fn sha256(content: &[u8]) -> String {
    hex::encode(Sha256::digest(content))
}

pub fn build_list_content(asn: u32, prefixes: &[String]) -> Result<String, String> {
    if asn == 0 || prefixes.is_empty() {
        return Err("Пустые префиксы или неверный ASN".into());
    }
    let mut unique = BTreeSet::new();
    for prefix in prefixes {
        let (address, length) = prefix
            .split_once('/')
            .ok_or_else(|| format!("Некорректный IP-префикс: {prefix}"))?;
        let address = address
            .parse::<IpAddr>()
            .map_err(|_| format!("Некорректный IP-префикс: {prefix}"))?;
        let max = if address.is_ipv4() { 32 } else { 128 };
        let length = length
            .parse::<u8>()
            .ok()
            .filter(|length| *length <= max)
            .ok_or_else(|| format!("Некорректная длина IP-префикса: {prefix}"))?;
        unique.insert(format!("{address}/{length}"));
    }
    let content = format!(
        "# AS{asn} managed by nfqws2-ui via RIPEstat\n{}\n",
        unique.into_iter().collect::<Vec<_>>().join("\n")
    );
    if content.len() > MAX_LIST_BYTES {
        return Err("ASN-список превышает лимит 512 КиБ".into());
    }
    Ok(content)
}

pub fn replace_list_atomically(
    destination: &Path,
    expected_sha256: &str,
    new_content: &[u8],
) -> Result<Vec<u8>, String> {
    if new_content.len() > MAX_LIST_BYTES {
        return Err("ASN-список превышает лимит 512 КиБ".into());
    }
    let metadata = fs::symlink_metadata(destination)
        .map_err(|error| format!("Не удалось проверить ASN-список: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err("Путь ASN-списка не является обычным файлом".into());
    }
    let previous = fs::read(destination)
        .map_err(|error| format!("Не удалось прочитать ASN-список: {error}"))?;
    if sha256(&previous) != expected_sha256 {
        return Err("ASN-список изменён вручную; автоматическое обновление остановлено".into());
    }
    let parent = destination
        .parent()
        .ok_or("Путь ASN-списка не содержит каталог")?;
    let leaf = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Некорректное имя ASN-списка")?;
    let temporary = parent.join(format!(".{leaf}.{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(metadata.permissions().mode() & 0o7777);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("Не удалось создать временный ASN-список: {error}"))?;
    if let Err(error) = file.write_all(new_content).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Не удалось записать ASN-список: {error}"));
    }
    drop(file);
    if let Err(error) = fs::rename(&temporary, destination) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Не удалось заменить ASN-список атомарно: {error}"));
    }
    if let Ok(directory) = fs::File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(previous)
}

pub async fn refresh_managed_list_with<F, Fut, R, RFut>(
    feed: &mut ManagedAsnList,
    store: &mut ManagedAsnListStore,
    data_dir: &Path,
    path: &Path,
    local_date: &str,
    fetch: F,
    reload: R,
) -> Result<(), String>
where
    F: FnOnce(u32) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<String>, String>>,
    R: FnOnce() -> RFut,
    RFut: std::future::Future<Output = Result<(), String>>,
{
    let current = tokio::fs::read(path)
        .await
        .map_err(|error| format!("Не удалось прочитать ASN-список: {error}"))?;
    if let Some(pending) = feed.pending_update.clone() {
        if sha256(&current) == pending.content_sha256 {
            if let Err(reload_error) = reload().await {
                let previous_sha256 = pending.previous_content_sha256.as_deref().ok_or_else(|| {
                    format!(
                        "{reload_error}; контрольная сумма прежнего ASN-списка отсутствует для отката"
                    )
                })?;
                let previous = load_previous_content(data_dir, feed.asn, previous_sha256).map_err(
                    |restore_error| {
                        format!(
                            "{reload_error}; прежний ASN-список не восстановлен: {restore_error}"
                        )
                    },
                )?;
                replace_list_atomically(path, &pending.content_sha256, &previous).map_err(
                    |restore_error| {
                        format!(
                            "{reload_error}; прежний ASN-список не восстановлен: {restore_error}"
                        )
                    },
                )?;
                feed.pending_update = None;
                feed.last_error = Some(reload_error.clone());
                store.register(feed.clone());
                store.save(data_dir)?;
                remove_previous_content(data_dir, feed.asn);
                return Err(reload_error);
            }
            let commit_date = feed
                .last_attempt_date
                .clone()
                .unwrap_or_else(|| local_date.to_owned());
            feed.finish_update(&commit_date);
            store.register(feed.clone());
            store.save(data_dir)?;
            remove_previous_content(data_dir, feed.asn);
            return Ok(());
        }
        if feed.content_matches(&current) {
            feed.pending_update = None;
            feed.last_error = Some("Предыдущее обновление прервано до замены файла".into());
            store.register(feed.clone());
            store.save(data_dir)?;
            remove_previous_content(data_dir, feed.asn);
            return Err(feed.last_error.clone().unwrap_or_default());
        } else {
            feed.paused = true;
            feed.pending_update = None;
            feed.last_error =
                Some("ASN-список изменён вручную; автоматическое обновление остановлено".into());
            store.register(feed.clone());
            store.save(data_dir)?;
            remove_previous_content(data_dir, feed.asn);
            return Err(feed.last_error.clone().unwrap_or_default());
        }
    }
    if !feed.is_due(local_date) {
        return Ok(());
    }
    if !feed.content_matches(&current) {
        feed.paused = true;
        feed.last_attempt_date = Some(local_date.to_owned());
        feed.last_error =
            Some("ASN-список изменён вручную; автоматическое обновление остановлено".into());
        store.register(feed.clone());
        store.save(data_dir)?;
        return Err(feed.last_error.clone().unwrap_or_default());
    }
    feed.last_attempt_date = Some(local_date.to_owned());
    feed.last_error = None;
    store.register(feed.clone());
    store.save(data_dir)?;

    let prefixes = match fetch(feed.asn).await {
        Ok(prefixes) => prefixes,
        Err(message) => {
            feed.last_error = Some(message.clone());
            store.register(feed.clone());
            store.save(data_dir)?;
            return Err(message);
        }
    };
    if !accept_prefix_count(feed.prefix_count, prefixes.len()) {
        let message = format!(
            "Ответ RIPEstat подозрительно мал ({} вместо {} префиксов); прежний список сохранён",
            prefixes.len(),
            feed.prefix_count
        );
        feed.last_error = Some(message.clone());
        store.register(feed.clone());
        store.save(data_dir)?;
        return Err(message);
    }
    let content = match build_list_content(feed.asn, &prefixes) {
        Ok(content) => content,
        Err(message) => {
            feed.last_error = Some(message.clone());
            store.register(feed.clone());
            store.save(data_dir)?;
            return Err(message);
        }
    };
    if let Err(message) = save_previous_content(data_dir, feed.asn, &current) {
        feed.last_error = Some(message.clone());
        store.register(feed.clone());
        store.save(data_dir)?;
        return Err(message);
    }
    feed.stage_update(prefixes.len(), content.as_bytes(), &current);
    store.register(feed.clone());
    if let Err(error) = store.save(data_dir) {
        feed.pending_update = None;
        remove_previous_content(data_dir, feed.asn);
        return Err(error);
    }
    let old_sha256 = feed.content_sha256.clone();
    let old_content = match replace_list_atomically(path, &old_sha256, content.as_bytes()) {
        Ok(previous) => previous,
        Err(message) => {
            if message.contains("изменён вручную") {
                feed.paused = true;
            }
            feed.pending_update = None;
            feed.last_error = Some(message.clone());
            store.register(feed.clone());
            store.save(data_dir)?;
            remove_previous_content(data_dir, feed.asn);
            return Err(message);
        }
    };
    if let Err(reload_error) = reload().await {
        match replace_list_atomically(path, &sha256(content.as_bytes()), &old_content) {
            Ok(_) => {
                feed.pending_update = None;
                feed.last_error = Some(reload_error.clone());
                store.register(feed.clone());
                store.save(data_dir)?;
                remove_previous_content(data_dir, feed.asn);
                return Err(reload_error);
            }
            Err(restore_error) => {
                feed.last_error = Some(format!(
                    "Не удалось перечитать список ({reload_error}) или вернуть прежний файл ({restore_error})"
                ));
                store.register(feed.clone());
                store.save(data_dir)?;
                return Err(feed.last_error.clone().unwrap_or_default());
            }
        }
    }
    feed.finish_update(local_date);
    store.register(feed.clone());
    store.save(data_dir)?;
    remove_previous_content(data_dir, feed.asn);
    Ok(())
}

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> ManagedAsnList {
        let content = b"# AS15169 RIPEstat\n8.8.8.0/24\n8.8.4.0/24\n";
        ManagedAsnList::new(15169, "ipset_as15169.list", 2, content)
    }

    #[test]
    fn refresh_is_due_once_per_local_day_and_skips_paused_feeds() {
        let mut feed = record();
        assert!(feed.is_due("2026-10-27"));
        feed.last_attempt_date = Some("2026-10-27".into());
        assert!(!feed.is_due("2026-10-27"));
        assert!(feed.is_due("2026-10-28"));
        feed.paused = true;
        assert!(!feed.is_due("2026-10-28"));
        assert!(!feed.is_due(""));
        feed.pending_update = Some(PendingAsnListUpdate {
            prefix_count: 3,
            content_sha256: sha256(b"pending"),
            previous_content_sha256: Some(feed.content_sha256.clone()),
        });
        assert!(
            feed.is_due("2026-10-28"),
            "pending update needs crash recovery"
        );
    }

    #[test]
    fn only_nonempty_lists_at_least_one_third_of_previous_count_are_accepted() {
        assert!(accept_prefix_count(60, 20));
        assert!(!accept_prefix_count(60, 19));
        assert!(!accept_prefix_count(60, 0));
        assert!(accept_prefix_count(0, 1));
    }

    #[test]
    fn ownership_check_detects_manual_edits_to_managed_list() {
        let feed = record();
        let unchanged = b"# AS15169 RIPEstat\n8.8.8.0/24\n8.8.4.0/24\n";
        let edited = b"# local exception\n8.8.8.0/24\n";
        assert!(feed.content_matches(unchanged));
        assert!(!feed.content_matches(edited));
    }

    #[test]
    fn previous_content_backup_is_bounded_replaceable_and_hash_checked() {
        let directory = tempfile::tempdir().unwrap();
        let first = b"8.8.8.0/24\n";
        let second = b"8.8.4.0/24\n";
        save_previous_content(directory.path(), 15169, first).unwrap();
        save_previous_content(directory.path(), 15169, second).unwrap();
        assert_eq!(
            load_previous_content(directory.path(), 15169, &sha256(second)).unwrap(),
            second
        );
        assert!(load_previous_content(directory.path(), 15169, &sha256(first)).is_err());
        assert!(
            save_previous_content(directory.path(), 15169, &vec![b'x'; MAX_LIST_BYTES + 1])
                .is_err()
        );
    }

    #[test]
    fn list_store_round_trips_registration_and_last_error() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = ManagedAsnListStore::default();
        let mut feed = record();
        feed.last_attempt_date = Some("2026-10-27".into());
        feed.last_error = Some("RIPEstat не ответил".into());
        store.register(feed);
        store.save(directory.path()).unwrap();
        assert_eq!(ManagedAsnListStore::load(directory.path()), store);
        let mut changed = store.clone();
        changed.entries[0].last_error = None;
        changed.save(directory.path()).unwrap();
        assert_eq!(ManagedAsnListStore::load(directory.path()), changed);
    }

    #[test]
    fn staged_update_is_committed_only_after_file_replacement() {
        let old = b"# AS15169 RIPEstat\n8.8.8.0/24\n";
        let new = b"# AS15169 RIPEstat\n8.8.8.0/24\n8.8.4.0/24\n";
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 1, old);
        feed.last_attempt_date = Some("2026-10-27".into());
        feed.stage_update(2, new, old);
        assert_eq!(feed.prefix_count, 1);
        assert!(feed.is_due("2026-10-27"));
        assert!(feed.finish_update("2026-10-27"));
        assert_eq!(feed.prefix_count, 2);
        assert_eq!(feed.content_sha256, sha256(new));
        assert_eq!(feed.last_success_date.as_deref(), Some("2026-10-27"));
        assert!(!feed.is_due("2026-10-27"));
    }

    #[test]
    fn list_builder_validates_and_deduplicates_ripe_prefixes() {
        let content = build_list_content(
            15169,
            &[
                "8.8.8.0/24".into(),
                "8.8.4.0/24".into(),
                "8.8.8.0/24".into(),
            ],
        )
        .unwrap();
        assert_eq!(
            content,
            "# AS15169 managed by nfqws2-ui via RIPEstat\n8.8.4.0/24\n8.8.8.0/24\n"
        );
        assert!(build_list_content(0, &["8.8.8.0/24".into()]).is_err());
        assert!(build_list_content(15169, &["invalid".into()]).is_err());
    }

    #[test]
    fn list_builder_rejects_content_over_the_nfqw2_list_limit() {
        let many_prefixes = (0..50_000)
            .map(|index| format!("10.{}.{}.0/24", index / 256, index % 256))
            .collect::<Vec<_>>();
        assert!(build_list_content(15169, &many_prefixes).is_err());
    }

    #[test]
    fn replacement_refuses_stale_or_symlinked_files_and_preserves_current_content() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ipset_as15169.list");
        let old = b"# AS15169 managed by nfqws2-ui via RIPEstat\n8.8.8.0/24\n";
        let new = b"# AS15169 managed by nfqws2-ui via RIPEstat\n8.8.8.0/24\n8.8.4.0/24\n";
        std::fs::write(&path, old).unwrap();
        let stale = sha256(b"unexpected");
        assert!(replace_list_atomically(&path, &stale, new).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), old);
        replace_list_atomically(&path, &sha256(old), new).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), new);
    }

    #[cfg(unix)]
    #[test]
    fn replacement_refuses_a_symlink_at_the_managed_list_path() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.list");
        let path = directory.path().join("ipset_as15169.list");
        let old = b"8.8.8.0/24\n";
        std::fs::write(&target, old).unwrap();
        symlink(&target, &path).unwrap();
        assert!(replace_list_atomically(&path, &sha256(old), b"8.8.4.0/24\n").is_err());
        assert_eq!(std::fs::read(&target).unwrap(), old);
    }

    #[tokio::test]
    async fn successful_refresh_atomically_updates_file_metadata_and_requests_reload() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("data");
        let lists = directory.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        let path = lists.join("ipset_as15169.list");
        let old = build_list_content(15169, &["8.8.8.0/24".into()]).unwrap();
        std::fs::write(&path, &old).unwrap();
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 1, old.as_bytes());
        let mut store = ManagedAsnListStore::default();
        store.register(feed.clone());
        let mut reloaded = false;
        refresh_managed_list_with(
            &mut feed,
            &mut store,
            &data,
            &path,
            "2026-10-27",
            |_| async { Ok(vec!["8.8.8.0/24".into(), "8.8.4.0/24".into()]) },
            || async {
                reloaded = true;
                Ok(())
            },
        )
        .await
        .unwrap();
        assert!(reloaded);
        let new = std::fs::read(&path).unwrap();
        assert_ne!(new, old.as_bytes());
        assert!(feed.content_matches(&new));
        assert_eq!(feed.prefix_count, 2);
        assert_eq!(feed.last_success_date.as_deref(), Some("2026-10-27"));
        assert_eq!(ManagedAsnListStore::load(&data).entries, vec![feed]);
        assert!(!previous_content_path(&data, 15169).exists());
    }

    #[tokio::test]
    async fn refresh_rejects_suspicious_reply_and_preserves_file_without_reload() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("data");
        let path = directory.path().join("ipset_as15169.list");
        let old = build_list_content(
            15169,
            &[
                "8.8.8.0/24".into(),
                "8.8.4.0/24".into(),
                "1.1.1.0/24".into(),
                "1.0.0.0/24".into(),
            ],
        )
        .unwrap();
        std::fs::write(&path, &old).unwrap();
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 4, old.as_bytes());
        let mut store = ManagedAsnListStore::default();
        store.register(feed.clone());
        let mut reload_called = false;
        assert!(
            refresh_managed_list_with(
                &mut feed,
                &mut store,
                &data,
                &path,
                "2026-10-27",
                |_| async { Ok(vec!["8.8.8.0/24".into()]) },
                || async {
                    reload_called = true;
                    Ok(())
                },
            )
            .await
            .is_err()
        );
        assert!(!reload_called);
        assert_eq!(std::fs::read(&path).unwrap(), old.as_bytes());
        assert_eq!(feed.prefix_count, 4);
        assert!(feed.last_error.is_some());
        assert!(!feed.is_due("2026-10-27"));
    }

    #[tokio::test]
    async fn reload_failure_restores_old_file_and_clears_pending_update() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("data");
        let path = directory.path().join("ipset_as15169.list");
        let old = build_list_content(15169, &["8.8.8.0/24".into()]).unwrap();
        std::fs::write(&path, &old).unwrap();
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 1, old.as_bytes());
        let mut store = ManagedAsnListStore::default();
        store.register(feed.clone());
        let error = refresh_managed_list_with(
            &mut feed,
            &mut store,
            &data,
            &path,
            "2026-10-27",
            |_| async { Ok(vec!["8.8.8.0/24".into(), "8.8.4.0/24".into()]) },
            || async { Err("SIGHUP failed".into()) },
        )
        .await
        .unwrap_err();
        assert!(error.contains("SIGHUP failed"));
        assert_eq!(std::fs::read(&path).unwrap(), old.as_bytes());
        assert_eq!(feed.prefix_count, 1);
        assert!(feed.pending_update.is_none());
        assert_eq!(feed.last_error.as_deref(), Some("SIGHUP failed"));
        assert!(!previous_content_path(&data, 15169).exists());
    }

    #[tokio::test]
    async fn interrupted_refresh_recovery_restores_backup_when_sighup_fails() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("data");
        let path = directory.path().join("ipset_as15169.list");
        let old = build_list_content(15169, &["8.8.8.0/24".into()]).unwrap();
        let new = build_list_content(15169, &["8.8.8.0/24".into(), "8.8.4.0/24".into()]).unwrap();
        std::fs::write(&path, &new).unwrap();
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 1, old.as_bytes());
        feed.last_attempt_date = Some("2026-10-27".into());
        feed.stage_update(2, new.as_bytes(), old.as_bytes());
        let mut store = ManagedAsnListStore::default();
        store.register(feed.clone());
        store.save(&data).unwrap();
        std::fs::write(data.join("managed-asn-15169.previous"), &old).unwrap();

        let error = refresh_managed_list_with(
            &mut feed,
            &mut store,
            &data,
            &path,
            "2026-10-28",
            |_| async { panic!("pending updates must recover without refetching") },
            || async { Err("SIGHUP failed after restart".into()) },
        )
        .await
        .unwrap_err();

        assert!(error.contains("SIGHUP failed after restart"));
        assert_eq!(std::fs::read(&path).unwrap(), old.as_bytes());
        assert!(feed.pending_update.is_none());
        assert_eq!(feed.content_sha256, sha256(old.as_bytes()));
        assert_eq!(
            feed.last_error.as_deref(),
            Some("SIGHUP failed after restart")
        );
        assert!(!data.join("managed-asn-15169.previous").exists());
    }

    #[tokio::test]
    async fn manually_changed_list_pauses_feed_without_fetch_or_overwrite() {
        let directory = tempfile::tempdir().unwrap();
        let data = directory.path().join("data");
        let path = directory.path().join("ipset_as15169.list");
        let original = build_list_content(15169, &["8.8.8.0/24".into()]).unwrap();
        let edited = b"# keep my change\n8.8.8.0/24\n";
        std::fs::write(&path, edited).unwrap();
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 1, original.as_bytes());
        let mut store = ManagedAsnListStore::default();
        store.register(feed.clone());
        let mut fetch_called = false;
        assert!(
            refresh_managed_list_with(
                &mut feed,
                &mut store,
                &data,
                &path,
                "2026-10-27",
                |_| async {
                    fetch_called = true;
                    Ok(vec!["8.8.8.0/24".into()])
                },
                || async { Ok(()) },
            )
            .await
            .is_err()
        );
        assert!(!fetch_called);
        assert!(feed.paused);
        assert_eq!(std::fs::read(&path).unwrap(), edited);
    }
}
