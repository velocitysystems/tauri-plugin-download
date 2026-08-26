use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tempfile::NamedTempFile;

use crate::Error;
use crate::models::{DownloadRecord, DownloadStatus};

pub(crate) enum UpdateIfStatusResult {
   Updated(DownloadRecord),
   Unchanged(DownloadRecord),
   NotFound,
}

/// Thread-safe JSON file store for download records, mirroring iOS `DownloadStore`.
#[derive(Clone, Debug)]
pub struct DownloadStore {
   inner: Arc<Mutex<StoreInner>>,
}

#[derive(Debug)]
struct StoreInner {
   downloads: Vec<DownloadRecord>,
   path: PathBuf,
}

impl DownloadStore {
   /// Creates a new store backed by the given file path.
   pub fn new(path: PathBuf) -> Self {
      Self {
         inner: Arc::new(Mutex::new(StoreInner {
            downloads: Vec::new(),
            path,
         })),
      }
   }

   pub fn list(&self) -> crate::Result<Vec<DownloadRecord>> {
      let inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;
      Ok(inner.downloads.clone())
   }

   pub fn find_by_path(&self, path: &str) -> crate::Result<Option<DownloadRecord>> {
      let inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;
      Ok(inner.downloads.iter().find(|i| i.path == path).cloned())
   }

   pub fn create(&self, item: DownloadRecord) -> crate::Result<DownloadRecord> {
      let mut inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;

      if inner.downloads.iter().any(|i| i.path == item.path) {
         return Err(Error::Store(format!(
            "Item already exists for path: {}",
            &item.path
         )));
      }

      inner.downloads.push(item.clone());
      save_inner(&inner)?;
      Ok(item)
   }

   pub fn update(&self, item: DownloadRecord) -> crate::Result<()> {
      let mut inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;

      if let Some(existing) = inner.downloads.iter_mut().find(|i| i.path == item.path) {
         *existing = item;
      }
      save_inner(&inner)?;
      Ok(())
   }

   pub fn update_if_status(
      &self,
      path: &str,
      expected_status: DownloadStatus,
      new_status: DownloadStatus,
   ) -> crate::Result<UpdateIfStatusResult> {
      let mut inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;

      let Some(existing) = inner.downloads.iter_mut().find(|item| item.path == path) else {
         return Ok(UpdateIfStatusResult::NotFound);
      };

      if existing.status != expected_status {
         return Ok(UpdateIfStatusResult::Unchanged(existing.clone()));
      }

      existing.status = new_status;
      let updated = existing.clone();
      save_inner(&inner)?;
      Ok(UpdateIfStatusResult::Updated(updated))
   }

   pub fn update_no_persist(&self, item: DownloadRecord) -> crate::Result<()> {
      let mut inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;

      if let Some(existing) = inner.downloads.iter_mut().find(|i| i.path == item.path) {
         *existing = item;
      }
      Ok(())
   }

   pub fn delete(&self, path: &str) -> crate::Result<()> {
      let mut inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;

      inner.downloads.retain(|i| i.path != path);
      save_inner(&inner)?;
      Ok(())
   }

   /// Loads the store from disk. Should be called once at startup.
   pub fn load(&self) -> crate::Result<()> {
      let mut inner = self
         .inner
         .lock()
         .map_err(|e| Error::Store(format!("Lock poisoned: {}", e)))?;

      if !inner.path.exists() {
         return Ok(());
      }

      let data =
         fs::read(&inner.path).map_err(|e| Error::Store(format!("Failed to read store: {}", e)))?;
      inner.downloads = serde_json::from_slice(&data)
         .map_err(|e| Error::Store(format!("Failed to parse store: {}", e)))?;

      Ok(())
   }
}

/// Serializes and writes the store to disk.
///
/// Accepts `&StoreInner` directly rather than `&self` because callers already hold the
/// `MutexGuard` when they call this. Taking `&self` would attempt to re-acquire the lock
/// on the same thread, causing a deadlock since `Mutex` is not re-entrant.
///
/// Writes atomically, as iOS and Android do, so a crash mid-write leaves the previous store
/// intact rather than truncated JSON. Two consequences: the store takes the temp file's
/// `0600` on Unix, leaving it app-private as it is on mobile, and the directory is not
/// fsynced, so a power loss can cost the last update but never the file.
fn save_inner(inner: &StoreInner) -> crate::Result<()> {
   let parent = inner
      .path
      .parent()
      .ok_or_else(|| Error::Store("Store path has no parent directory".to_string()))?;

   fs::create_dir_all(parent)
      .map_err(|e| Error::Store(format!("Failed to create store directory: {}", e)))?;

   // Serialize before touching the disk so a failure here cannot leave a temp file behind.
   let data = serde_json::to_vec(&inner.downloads)
      .map_err(|e| Error::Store(format!("Failed to serialize store: {}", e)))?;

   // The temp file has to be a sibling of the store: `rename` cannot cross a mount point.
   // `NamedTempFile` removes itself on drop, so the early returns below leave no debris.
   let mut temp = NamedTempFile::new_in(parent)
      .map_err(|e| Error::Store(format!("Failed to create temp store file: {}", e)))?;

   temp
      .write_all(&data)
      .map_err(|e| Error::Store(format!("Failed to write store: {}", e)))?;

   // Sync before the rename, as Android's `AtomicFile.finishWrite` does. `flush` would be
   // a no-op — the writes are unbuffered — and the rename only publishes a directory entry,
   // so without this a crash can leave a correctly named file whose contents never landed.
   temp.as_file().sync_all().map_err(|e| {
      Error::Store(format!(
         "Failed to sync store to disk: {} at path {:?}",
         e,
         temp.path()
      ))
   })?;

   temp.persist(&inner.path).map_err(|e| {
      Error::Store(format!(
         "Failed to replace store: {} at path {:?}",
         e.error, inner.path
      ))
   })?;

   Ok(())
}

#[cfg(test)]
mod tests {
   use super::*;
   use crate::models::DownloadStatus;
   use std::fs;
   use tempfile::TempDir;

   fn temp_store() -> (DownloadStore, TempDir) {
      let dir = TempDir::new().unwrap();
      let store = DownloadStore::new(dir.path().join("downloads.json"));
      (store, dir)
   }

   fn sample_record(path: &str) -> DownloadRecord {
      DownloadRecord {
         url: "https://example.com/file.mp4".to_string(),
         path: path.to_string(),
         options: Default::default(),
         received_bytes: 0,
         total_bytes: None,
         status: DownloadStatus::Idle,
      }
   }

   #[test]
   fn test_list_empty() {
      let (store, _dir) = temp_store();
      assert!(store.list().unwrap().is_empty());
   }

   #[test]
   fn test_list_after_create() {
      let (store, _dir) = temp_store();
      store.create(sample_record("/tmp/a.mp4")).unwrap();
      store.create(sample_record("/tmp/b.mp4")).unwrap();
      assert_eq!(store.list().unwrap().len(), 2);
   }

   #[test]
   fn test_find_by_path_found() {
      let (store, _dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      let result = store.find_by_path("/tmp/file.mp4").unwrap();
      assert_eq!(result.unwrap().path, "/tmp/file.mp4");
   }

   #[test]
   fn test_find_by_path_not_found() {
      let (store, _dir) = temp_store();
      assert!(store.find_by_path("/tmp/missing.mp4").unwrap().is_none());
   }

   #[test]
   fn test_create_success() {
      let (store, _dir) = temp_store();
      let item = store.create(sample_record("/tmp/file.mp4")).unwrap();
      assert_eq!(item.path, "/tmp/file.mp4");
   }

   #[test]
   fn test_create_persists_to_disk() {
      let (store, dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      assert!(dir.path().join("downloads.json").exists());
   }

   #[test]
   fn test_create_options_persist_across_reload() {
      let (store, dir) = temp_store();
      let mut item = sample_record("/tmp/file.mp4");
      item.options.allow_metered = false;
      store.create(item).unwrap();

      let reloaded = DownloadStore::new(dir.path().join("downloads.json"));
      reloaded.load().unwrap();

      let found = reloaded.find_by_path("/tmp/file.mp4").unwrap().unwrap();
      assert!(!found.options.allow_metered);
   }

   #[test]
   fn test_create_duplicate_returns_error() {
      let (store, _dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      let result = store.create(sample_record("/tmp/file.mp4"));
      assert!(result.is_err());
   }

   #[test]
   fn test_update_persists_to_disk() {
      let (store, dir) = temp_store();
      let item = store.create(sample_record("/tmp/file.mp4")).unwrap();
      let updated = DownloadRecord {
         received_bytes: 500,
         total_bytes: Some(1000),
         status: DownloadStatus::InProgress,
         ..item
      };
      store.update(updated).unwrap();

      let reloaded = DownloadStore::new(dir.path().join("downloads.json"));
      reloaded.load().unwrap();
      let found = reloaded.find_by_path("/tmp/file.mp4").unwrap().unwrap();
      assert_eq!(found.received_bytes, 500);
      assert_eq!(found.total_bytes, Some(1000));
   }

   #[test]
   fn test_update_no_op_on_unknown_path() {
      let (store, _dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      let unknown = sample_record("/tmp/unknown.mp4");
      assert!(store.update(unknown).is_ok());
      assert_eq!(store.list().unwrap().len(), 1);
   }

   #[test]
   fn test_update_if_status_updates_matching_record() {
      let (store, dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();

      let result = store
         .update_if_status(
            "/tmp/file.mp4",
            DownloadStatus::Idle,
            DownloadStatus::InProgress,
         )
         .unwrap();

      assert!(matches!(
         result,
         UpdateIfStatusResult::Updated(item)
            if item.status == DownloadStatus::InProgress
      ));

      let reloaded = DownloadStore::new(dir.path().join("downloads.json"));
      reloaded.load().unwrap();
      assert_eq!(
         reloaded
            .find_by_path("/tmp/file.mp4")
            .unwrap()
            .unwrap()
            .status,
         DownloadStatus::InProgress
      );
   }

   #[test]
   fn test_update_if_status_returns_current_record_on_mismatch() {
      let (store, _dir) = temp_store();
      let mut item = sample_record("/tmp/file.mp4");
      item.status = DownloadStatus::Paused;
      store.create(item).unwrap();

      let result = store
         .update_if_status(
            "/tmp/file.mp4",
            DownloadStatus::Idle,
            DownloadStatus::InProgress,
         )
         .unwrap();

      assert!(matches!(
         result,
         UpdateIfStatusResult::Unchanged(item) if item.status == DownloadStatus::Paused
      ));
      assert_eq!(
         store.find_by_path("/tmp/file.mp4").unwrap().unwrap().status,
         DownloadStatus::Paused
      );
   }

   #[test]
   fn test_update_if_status_returns_not_found_for_unknown_path() {
      let (store, _dir) = temp_store();

      assert!(matches!(
         store
            .update_if_status(
               "/tmp/missing.mp4",
               DownloadStatus::Idle,
               DownloadStatus::InProgress,
            )
            .unwrap(),
         UpdateIfStatusResult::NotFound
      ));
   }

   #[test]
   fn test_update_no_persist_does_not_write_disk() {
      let (store, dir) = temp_store();
      let item = store.create(sample_record("/tmp/file.mp4")).unwrap();
      let updated = DownloadRecord {
         received_bytes: 750,
         total_bytes: Some(1000),
         ..item
      };
      store.update_no_persist(updated).unwrap();

      // In-memory reflects the change.
      let in_memory = store.find_by_path("/tmp/file.mp4").unwrap().unwrap();
      assert_eq!(in_memory.received_bytes, 750);

      // Disk still has the original value.
      let reloaded = DownloadStore::new(dir.path().join("downloads.json"));
      reloaded.load().unwrap();
      let on_disk = reloaded.find_by_path("/tmp/file.mp4").unwrap().unwrap();
      assert_eq!(on_disk.received_bytes, 0);
   }

   #[test]
   fn test_delete_removes_item_and_persists() {
      let (store, dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      store.delete("/tmp/file.mp4").unwrap();

      assert!(store.list().unwrap().is_empty());

      let reloaded = DownloadStore::new(dir.path().join("downloads.json"));
      reloaded.load().unwrap();
      assert!(reloaded.list().unwrap().is_empty());
   }

   #[test]
   fn test_delete_unknown_path_is_ok() {
      let (store, _dir) = temp_store();
      assert!(store.delete("/tmp/nonexistent.mp4").is_ok());
   }

   #[test]
   fn test_load_missing_file_is_ok() {
      let (store, _dir) = temp_store();
      assert!(store.load().is_ok());
      assert!(store.list().unwrap().is_empty());
   }

   #[test]
   fn test_load_from_valid_json() {
      let dir = TempDir::new().unwrap();
      let path = dir.path().join("downloads.json");
      let items = vec![sample_record("/tmp/file.mp4")];
      fs::write(&path, serde_json::to_vec(&items).unwrap()).unwrap();

      let store = DownloadStore::new(path);
      store.load().unwrap();
      assert_eq!(store.list().unwrap().len(), 1);
   }

   #[test]
   fn test_load_invalid_json_returns_error() {
      let dir = TempDir::new().unwrap();
      let path = dir.path().join("downloads.json");
      fs::write(&path, b"not valid json").unwrap();

      let store = DownloadStore::new(path);
      assert!(store.load().is_err());
   }

   #[test]
   fn test_save_creates_parent_directory() {
      let dir = TempDir::new().unwrap();
      let store = DownloadStore::new(dir.path().join("nested/dir/downloads.json"));
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      assert!(dir.path().join("nested/dir/downloads.json").exists());
   }

   /// The temp file is removed on drop, so an early return between creating it and
   /// renaming it into place leaves no debris. A directory at the destination makes the
   /// rename fail, which is the only publish error reachable without fault injection.
   #[test]
   fn test_failed_save_leaves_no_temp_files() {
      let dir = TempDir::new().unwrap();
      let path = dir.path().join("downloads.json");
      fs::create_dir(&path).unwrap();
      let store = DownloadStore::new(path);

      assert!(store.create(sample_record("/tmp/file.mp4")).is_err());

      let entries: Vec<_> = fs::read_dir(dir.path())
         .unwrap()
         .map(|entry| entry.unwrap().file_name())
         .collect();
      assert_eq!(entries, vec!["downloads.json"]);
   }

   #[test]
   fn test_save_leaves_no_temp_files() {
      let (store, dir) = temp_store();
      store.create(sample_record("/tmp/file.mp4")).unwrap();
      store.delete("/tmp/file.mp4").unwrap();

      let entries: Vec<_> = fs::read_dir(dir.path())
         .unwrap()
         .map(|entry| entry.unwrap().file_name())
         .collect();
      assert_eq!(entries, vec!["downloads.json"]);
   }

   /// The store is published by renaming a temp file over it, so each save replaces the
   /// file rather than rewriting it. An in-place write would keep the same inode.
   #[cfg(unix)]
   #[test]
   fn test_save_replaces_rather_than_rewriting_in_place() {
      use std::os::unix::fs::MetadataExt;

      let (store, dir) = temp_store();
      let path = dir.path().join("downloads.json");

      store.create(sample_record("/tmp/file.mp4")).unwrap();
      let first = fs::metadata(&path).unwrap().ino();

      store.create(sample_record("/tmp/other.mp4")).unwrap();
      let second = fs::metadata(&path).unwrap().ino();

      assert_ne!(first, second);
   }
}
