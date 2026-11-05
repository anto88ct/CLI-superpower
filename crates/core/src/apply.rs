//! Apply a prepared plan and track the files owned by the integration.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize)]
pub struct PreviewFile {
    pub path: PathBuf,
    pub content: String,
}

pub struct PlannedFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub preview: String,
    // A shared file is editable only if it still matches the input to the planner.
    pub shared_input: Option<Option<Vec<u8>>>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Record {
    original: Option<Vec<u8>>,
    hash: String,
}

type Manifest = BTreeMap<PathBuf, Record>;

#[derive(Clone, Deserialize, Serialize)]
struct Change {
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Journal {
    changes: Vec<Change>,
    // Old journals tracked the manifest separately from other files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_manifest: Option<Vec<u8>>,
}

#[derive(Deserialize, Serialize)]
struct HistoryRecord {
    id: String,
    timestamp: u64,
    changes: Vec<Change>,
    manifest_before: Option<Vec<u8>>,
    manifest_after: String,
}

const HISTORY_LIMIT: usize = 10;

pub fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => Err(format!(
            "Il percorso non è un file regolare: {}",
            path.display()
        )),
        Ok(_) => fs::read(path).map(Some).map_err(|e| io_error(path, e)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(path, e)),
    }
}

fn io_error(path: &Path, error: io::Error) -> String {
    format!("{}: {error}", path.display())
}

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Percorso senza cartella: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    let temp = parent.join(format!(
        ".csp-{}-{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed),
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|e| io_error(&temp, e))?;
    let result = file.write_all(contents).and_then(|()| file.sync_all());
    drop(file);
    let result = result.and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|e| io_error(path, e))
}

fn replace(path: &Path, contents: Option<&[u8]>) -> Result<(), String> {
    if let Some(contents) = contents {
        atomic_write(path, contents)
    } else {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(path, e)),
        }
    }
}

fn fingerprint(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}

fn rollback(manifest_path: &Path, journal: &Journal) -> Result<(), String> {
    for change in journal.changes.iter().rev() {
        rollback_needed(change)?;
    }
    for change in journal.changes.iter().rev() {
        if rollback_needed(change)? {
            replace(&change.path, change.before.as_deref())?;
        }
    }
    if !journal
        .changes
        .iter()
        .any(|change| change.path == manifest_path)
    {
        replace(manifest_path, journal.previous_manifest.as_deref())?;
    }
    Ok(())
}

fn rollback_needed(change: &Change) -> Result<bool, String> {
    let current = read(&change.path)?;
    if current == change.before {
        return Ok(false);
    }
    if current.as_deref().map(fingerprint) != change.after {
        return Err(restore_conflict(&change.path));
    }
    Ok(true)
}

fn restore_conflict(path: &Path) -> String {
    format!(
        "Ripristino bloccato: {} è stato modificato esternamente.",
        path.display()
    )
}

pub fn lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|e| io_error(root, e))?;
    let path = root.join("apply.lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| io_error(&path, e))?;
    lock.lock().map_err(|e| io_error(&path, e))?;
    Ok(lock)
}

pub fn recover(root: &Path) -> Result<(), String> {
    let pending = root.join("pending.json");
    if let Some(bytes) = read(&pending)? {
        let journal =
            serde_json::from_slice(&bytes).map_err(|e| format!("Registro di ripristino: {e}"))?;
        rollback(&root.join("manifest.json"), &journal)?;
        replace(&pending, None)?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: u64,
    pub file_count: usize,
    pub can_restore: bool,
}

/// The caller holds the integration lock and has run recovery.
pub fn history(root: &Path) -> Result<Vec<HistoryEntry>, String> {
    let bytes = read(&root.join("history.json"))?;
    Ok(parse_history(bytes.as_deref())?
        .into_iter()
        .rev()
        .enumerate()
        .map(|(index, record)| HistoryEntry {
            can_restore: index == 0
                && record
                    .changes
                    .iter()
                    .all(|change| restore_input(&change.path, change.after.as_deref()).is_ok())
                && restore_input(&root.join("manifest.json"), Some(&record.manifest_after)).is_ok(),
            id: record.id,
            timestamp: record.timestamp,
            file_count: record.changes.len(),
        })
        .collect())
}

/// The caller holds the integration lock and has run recovery.
pub fn restore_latest(root: &Path) -> Result<(), String> {
    let history_path = root.join("history.json");
    let previous_history = read(&history_path)?;
    let mut history = parse_history(previous_history.as_deref())?;
    let record = history
        .pop()
        .ok_or("Nessuna transazione da ripristinare.")?;
    let manifest_path = root.join("manifest.json");
    let mut changes = Vec::new();
    let mut desired = Vec::new();
    for change in record.changes {
        let current = restore_input(&change.path, change.after.as_deref())?;
        stage(
            &mut changes,
            &mut desired,
            change.path,
            current,
            change.before,
        );
    }
    let current = restore_input(&manifest_path, Some(&record.manifest_after))?;
    stage(
        &mut changes,
        &mut desired,
        manifest_path,
        current,
        record.manifest_before,
    );
    let next_history = if history.is_empty() {
        None
    } else {
        Some(serde_json::to_vec(&history).map_err(|e| e.to_string())?)
    };
    stage(
        &mut changes,
        &mut desired,
        history_path,
        previous_history,
        next_history,
    );
    commit(root, changes, desired)
}

fn parse_history(bytes: Option<&[u8]>) -> Result<Vec<HistoryRecord>, String> {
    bytes
        .map(serde_json::from_slice)
        .transpose()
        .map_err(|e| format!("Cronologia non valida: {e}"))
        .map(Option::unwrap_or_default)
}

fn restore_input(path: &Path, hash: Option<&str>) -> Result<Option<Vec<u8>>, String> {
    let current = read(path)?;
    if current.as_deref().map(fingerprint).as_deref() != hash {
        return Err(restore_conflict(path));
    }
    Ok(current)
}

pub fn removals(root: &Path, files: &[PlannedFile]) -> Result<Vec<PreviewFile>, String> {
    let paths: BTreeSet<_> = files.iter().map(|file| &file.path).collect();
    let previous = read(&root.join("manifest.json"))?;
    Ok(parse_manifest(previous.as_deref())?
        .into_iter()
        .filter(|(path, _)| !paths.contains(path))
        .map(|(path, record)| PreviewFile {
            path,
            content: if record.original.is_some() {
                "Verrà ripristinato il file originale dal backup.".to_owned()
            } else {
                "Verrà rimosso il file generato.".to_owned()
            },
        })
        .collect())
}

fn parse_manifest(bytes: Option<&[u8]>) -> Result<Manifest, String> {
    bytes
        .map(serde_json::from_slice)
        .transpose()
        .map_err(|e| format!("Manifest non valido: {e}"))
        .map(Option::unwrap_or_default)
}

// A missing file the app created holds no user data, so it cannot conflict.
fn vanished(record: &Record, current: Option<&[u8]>) -> bool {
    current.is_none() && record.original.is_none()
}

/// The caller holds the integration lock and has run recovery before planning.
pub fn apply(
    root: &Path,
    files: Vec<PlannedFile>,
    preferences: Option<(PathBuf, Vec<u8>)>,
) -> Result<(), String> {
    let manifest_path = root.join("manifest.json");
    let previous_manifest = read(&manifest_path)?;
    let old = parse_manifest(previous_manifest.as_deref())?;
    let mut paths = BTreeSet::new();
    let mut manifest = Manifest::new();
    let mut changes = Vec::new();
    let mut desired = Vec::new();

    for file in files {
        if !paths.insert(file.path.clone()) {
            return Err(format!(
                "Due integrazioni puntano a {}",
                file.path.display()
            ));
        }
        let current = read(&file.path)?;
        let previous = old.get(&file.path);
        let current_hash = current.as_deref().map(fingerprint);
        let approved = match (previous, file.shared_input) {
            (Some(previous), _) if vanished(previous, current.as_deref()) => true,
            (Some(previous), _) if current_hash.as_ref() != Some(&previous.hash) => false,
            (_, Some(input)) => current == input,
            (None, None) => current.is_none(),
            (Some(_), None) => true,
        };
        if !approved {
            return Err(format!(
                "Conflitto: {} è già presente o è stato modificato esternamente.",
                file.path.display()
            ));
        }
        let hash = fingerprint(&file.bytes);
        let original = previous.map_or_else(|| current.clone(), |record| record.original.clone());
        manifest.insert(
            file.path.clone(),
            Record {
                original,
                hash: hash.clone(),
            },
        );
        if current_hash.as_ref() != Some(&hash) {
            changes.push(Change {
                path: file.path,
                before: current,
                after: Some(hash),
            });
            desired.push(Some(file.bytes));
        }
    }
    for (path, record) in old {
        if paths.contains(&path) {
            continue;
        }
        let current = read(&path)?;
        if !vanished(&record, current.as_deref())
            && current.as_deref().map(fingerprint).as_ref() != Some(&record.hash)
        {
            return Err(format!(
                "Conflitto durante il ripristino: {}",
                path.display()
            ));
        }
        changes.push(Change {
            path,
            before: current,
            after: record.original.as_deref().map(fingerprint),
        });
        desired.push(record.original);
    }
    if let Some((path, bytes)) = preferences {
        if !paths.insert(path.clone()) || changes.iter().any(|change| change.path == path) {
            return Err(format!(
                "Percorso delle preferenze già gestito: {}",
                path.display()
            ));
        }
        let current = read(&path)?;
        if current.as_deref() != Some(bytes.as_slice()) {
            changes.push(Change {
                path,
                before: current,
                after: Some(fingerprint(&bytes)),
            });
            desired.push(Some(bytes));
        }
    }
    if changes.is_empty() {
        return Ok(());
    }
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let history_path = root.join("history.json");
    let previous_history = read(&history_path)?;
    let mut history = parse_history(previous_history.as_deref())?;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    history.push(HistoryRecord {
        id: format!(
            "{}-{}-{}",
            time.as_nanos(),
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ),
        timestamp: time.as_secs(),
        changes: changes.clone(),
        manifest_before: previous_manifest.clone(),
        manifest_after: fingerprint(&manifest_bytes),
    });
    let excess = history.len().saturating_sub(HISTORY_LIMIT);
    history.drain(..excess);
    let history_bytes = serde_json::to_vec(&history).map_err(|e| e.to_string())?;
    stage(
        &mut changes,
        &mut desired,
        manifest_path,
        previous_manifest,
        Some(manifest_bytes),
    );
    stage(
        &mut changes,
        &mut desired,
        history_path,
        previous_history,
        Some(history_bytes),
    );
    commit(root, changes, desired)
}

fn stage(
    changes: &mut Vec<Change>,
    desired: &mut Vec<Option<Vec<u8>>>,
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
) {
    changes.push(Change {
        path,
        before,
        after: after.as_deref().map(fingerprint),
    });
    desired.push(after);
}

fn commit(root: &Path, changes: Vec<Change>, desired: Vec<Option<Vec<u8>>>) -> Result<(), String> {
    let pending = root.join("pending.json");
    let lock = root.join("apply.lock");
    let mut seen = BTreeSet::new();
    for change in &changes {
        if change.path == pending || change.path == lock || !seen.insert(&change.path) {
            return Err(format!(
                "Percorso di transazione non valido: {}",
                change.path.display()
            ));
        }
    }
    let journal = Journal {
        changes,
        previous_manifest: None,
    };
    let journal_bytes = serde_json::to_vec(&journal).map_err(|e| e.to_string())?;
    atomic_write(&pending, &journal_bytes)?;
    let result = (|| {
        for (change, bytes) in journal.changes.iter().zip(&desired) {
            if read(&change.path)? != change.before {
                return Err(format!(
                    "File modificato durante la transazione: {}",
                    change.path.display()
                ));
            }
            replace(&change.path, bytes.as_deref())?;
        }
        replace(&pending, None)
    })();
    if let Err(error) = result {
        recover(root).map_err(|recovery| format!("{error}\n{recovery}"))?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "csp-apply-test-{}-{}",
                std::process::id(),
                TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn owned(path: &Path, bytes: &[u8]) -> PlannedFile {
        PlannedFile {
            path: path.into(),
            bytes: bytes.into(),
            preview: String::new(),
            shared_input: None,
        }
    }

    #[test]
    fn retains_ten_meaningful_transactions_and_restores_removed_integration() {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("state");
        let fragment = temporary.0.join("fragment.json");
        let _lock = lock(&root).unwrap();
        for number in 1..=12 {
            apply(&root, vec![owned(&fragment, &[number])], None).unwrap();
        }
        let latest = history(&root).unwrap()[0].id.clone();
        apply(&root, vec![owned(&fragment, &[12])], None).unwrap();
        let entries = history(&root).unwrap();
        assert_eq!(entries.len(), 10);
        assert_eq!(entries[0].id, latest);
        apply(&root, vec![], None).unwrap();
        assert!(!fragment.exists());
        let entries = history(&root).unwrap();
        assert_eq!(entries.len(), 10);
        assert_eq!(entries[0].file_count, 1);
        assert!(entries[0].can_restore);
        assert!(entries[1..].iter().all(|entry| !entry.can_restore));
        assert_ne!(entries[0].id, entries[1].id);
        restore_latest(&root).unwrap();
        assert_eq!(fs::read(&fragment).unwrap(), [12]);
        assert_eq!(history(&root).unwrap().len(), 9);
        restore_latest(&root).unwrap();
        assert_eq!(fs::read(&fragment).unwrap(), [11]);
        apply(&root, vec![], None).unwrap();
        assert!(!fragment.exists());
    }

    #[test]
    fn rejects_conflicts_before_restoring_integration_or_preferences() {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("state");
        let fragment = temporary.0.join("fragment.json");
        let preferences = temporary.0.join("personal.json");
        let _lock = lock(&root).unwrap();
        atomic_write(&preferences, b"old").unwrap();
        apply(
            &root,
            vec![owned(&fragment, b"generated")],
            Some((preferences.clone(), b"new".to_vec())),
        )
        .unwrap();
        atomic_write(&preferences, b"external").unwrap();
        assert!(!history(&root).unwrap()[0].can_restore);
        assert!(restore_latest(&root).is_err());
        assert_eq!(fs::read(&preferences).unwrap(), b"external");
        assert_eq!(fs::read(&fragment).unwrap(), b"generated");
        atomic_write(&preferences, b"new").unwrap();
        restore_latest(&root).unwrap();
        assert_eq!(fs::read(&preferences).unwrap(), b"old");
        assert!(!fragment.exists());
        assert!(history(&root).unwrap().is_empty());

        apply(
            &root,
            vec![owned(&fragment, b"other")],
            Some((preferences.clone(), b"old".to_vec())),
        )
        .unwrap();
        atomic_write(&preferences, b"unrelated").unwrap();
        restore_latest(&root).unwrap();
        assert_eq!(fs::read(&preferences).unwrap(), b"unrelated");
    }

    #[test]
    fn apply_twice_preserves_original_and_conflicts_block_removal() {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("state");
        let profile = temporary.0.join("profile.ps1");
        let fragment = temporary.0.join("fragment.json");
        atomic_write(&profile, b"user").unwrap();
        let _lock = lock(&root).unwrap();
        for _ in 0..2 {
            apply(
                &root,
                vec![
                    PlannedFile {
                        path: profile.clone(),
                        bytes: b"user + generated".to_vec(),
                        preview: String::new(),
                        shared_input: Some(read(&profile).unwrap()),
                    },
                    PlannedFile {
                        path: fragment.clone(),
                        bytes: b"fragment".to_vec(),
                        preview: String::new(),
                        shared_input: None,
                    },
                ],
                None,
            )
            .unwrap();
        }
        atomic_write(&profile, b"user change").unwrap();
        assert!(apply(&root, vec![], None).is_err());
        assert_eq!(fs::read(&fragment).unwrap(), b"fragment");
        assert_eq!(fs::read(&profile).unwrap(), b"user change");
        atomic_write(&profile, b"user + generated").unwrap();
        apply(&root, vec![], None).unwrap();
        assert_eq!(fs::read(&profile).unwrap(), b"user");
        assert!(!fragment.exists());
    }

    #[test]
    fn vanished_generated_file_is_recreated_or_forgotten() {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("state");
        let script = temporary.0.join("clink/cli-superpower.lua");
        let _lock = lock(&root).unwrap();
        apply(&root, vec![owned(&script, b"generated")], None).unwrap();

        fs::remove_dir_all(script.parent().unwrap()).unwrap();
        apply(&root, vec![owned(&script, b"generated")], None).unwrap();
        assert_eq!(fs::read(&script).unwrap(), b"generated");

        fs::remove_file(&script).unwrap();
        apply(&root, vec![], None).unwrap();
        assert!(!script.exists());
        assert!(
            parse_manifest(read(&root.join("manifest.json")).unwrap().as_deref())
                .unwrap()
                .is_empty()
        );
        assert!(history(&root).unwrap()[0].can_restore);
    }

    #[test]
    fn external_edits_and_missing_user_files_still_conflict() {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("state");
        let script = temporary.0.join("script.lua");
        let profile = temporary.0.join("profile.ps1");
        atomic_write(&profile, b"user").unwrap();
        let _lock = lock(&root).unwrap();
        let shared = |input| PlannedFile {
            path: profile.clone(),
            bytes: b"user + generated".to_vec(),
            preview: String::new(),
            shared_input: Some(input),
        };
        apply(
            &root,
            vec![owned(&script, b"generated"), shared(Some(b"user".to_vec()))],
            None,
        )
        .unwrap();

        atomic_write(&script, b"external").unwrap();
        assert!(apply(&root, vec![owned(&script, b"generated")], None).is_err());
        assert!(apply(&root, vec![], None).is_err());
        assert_eq!(fs::read(&script).unwrap(), b"external");
        atomic_write(&script, b"generated").unwrap();

        // A user file with a backup must not silently come back or be dropped.
        fs::remove_file(&profile).unwrap();
        assert!(apply(&root, vec![shared(None)], None).is_err());
        assert!(apply(&root, vec![], None).is_err());
        assert!(!profile.exists());
    }

    #[test]
    fn recovery_reverts_only_files_matching_the_interrupted_transaction() {
        let temporary = TestDirectory::new();
        let root = &temporary.0;
        let path = root.join("profile.ps1");
        let manifest = root.join("manifest.json");
        let journal = Journal {
            changes: vec![Change {
                path: path.clone(),
                before: Some(b"original".to_vec()),
                after: Some(fingerprint(b"planned")),
            }],
            previous_manifest: Some(b"{}".to_vec()),
        };
        atomic_write(
            &root.join("pending.json"),
            &serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        atomic_write(&manifest, b"new").unwrap();
        atomic_write(&path, b"external change").unwrap();
        assert!(recover(root).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"external change");
        atomic_write(&path, b"planned").unwrap();
        recover(root).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read(&manifest).unwrap(), b"{}");
        assert!(!root.join("pending.json").exists());
    }

    #[test]
    fn interrupted_restore_recovers_history_and_manifest_with_files() {
        let temporary = TestDirectory::new();
        let root = temporary.0.join("state");
        let fragment = temporary.0.join("fragment.json");
        let manifest = root.join("manifest.json");
        let history_path = root.join("history.json");
        let _lock = lock(&root).unwrap();
        apply(&root, vec![owned(&fragment, b"generated")], None).unwrap();
        let history_before = read(&history_path).unwrap().unwrap();
        let manifest_before = read(&manifest).unwrap().unwrap();

        let journal = Journal {
            changes: [
                (fragment.clone(), b"generated".to_vec()),
                (manifest.clone(), manifest_before.clone()),
                (history_path.clone(), history_before.clone()),
            ]
            .into_iter()
            .map(|(path, bytes)| Change {
                path,
                before: Some(bytes),
                after: None,
            })
            .collect(),
            previous_manifest: None,
        };
        atomic_write(
            &root.join("pending.json"),
            &serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        for change in &journal.changes {
            replace(&change.path, None).unwrap();
        }
        recover(&root).unwrap();
        assert_eq!(fs::read(&fragment).unwrap(), b"generated");
        assert_eq!(fs::read(&manifest).unwrap(), manifest_before);
        assert_eq!(fs::read(&history_path).unwrap(), history_before);
        assert!(history(&root).unwrap()[0].can_restore);
    }
}
