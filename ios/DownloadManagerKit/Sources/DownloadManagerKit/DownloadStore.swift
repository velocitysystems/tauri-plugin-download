//
//  DownloadStore.swift
//  DownloadManagerKit
//

import Foundation
import os.log

/// Thread-safe store for the download records array, persisted to an atomically
/// written JSON file.
actor DownloadStore {
   private var downloads: [DownloadRecord]
   private let savePath: URL

   /// - Parameter savePath: Where the store is persisted. Injectable so tests can
   ///   work against a temporary file rather than the app's Documents directory.
   ///   Defaults through [`StoreLocation`], which is what a configured directory
   ///   reaches this class by — the path has to be known here, since the file is read
   ///   below rather than on first use.
   init(savePath: URL = StoreLocation.savePath()) {
      self.savePath = savePath
      self.downloads = DownloadStore.load(from: savePath)
   }

   func list() -> [DownloadRecord] { downloads }
   
   func findByPath(_ path: URL) -> DownloadRecord? {
      downloads.first(where: { $0.path == path })
   }
   
   func findByUrl(_ url: URL) -> DownloadRecord? {
      downloads.first(where: { $0.url == url })
   }
   
   func append(_ item: DownloadRecord) {
      downloads.append(item)
      save()
   }
   
   func update(_ item: DownloadRecord, persist: Bool = true) {
      if let index = downloads.firstIndex(where: { $0.path == item.path }) {
         downloads[index] = item
      }
      if persist {
         save()
      }
   }
   
   /// Records a newly-learned total in a single actor hop, returning the updated
   /// record, or nil when the total was already known.
   ///
   /// Progress callbacks arrive as unordered tasks, so a compare-and-set composed
   /// from a separate `findByUrl` and `update` lets several callbacks each read an
   /// unknown total and each act on it. Deliberately does not persist: pause,
   /// cancel and completion all write the record, and this runs on the hottest
   /// callback in the system.
   func setTotalIfChanged(path: URL, total: UInt64) -> DownloadRecord? {
      guard let index = downloads.firstIndex(where: { $0.path == path }),
            downloads[index].totalBytes != total else {
         return nil
      }

      downloads[index].setBytes(received: downloads[index].receivedBytes, total: total)

      return downloads[index]
   }

   /// Applies `body` in one actor hop and returns the stored record, or nil when no
   /// record has that path. Composed from `findByPath` and `update` it would suspend
   /// between read and write and lose concurrent changes; `body` is synchronous for
   /// the same reason.
   func mutate(path: URL, persist: Bool, _ body: @Sendable (inout DownloadRecord) -> Void) -> DownloadRecord? {
      guard let index = downloads.firstIndex(where: { $0.path == path }) else {
         return nil
      }

      body(&downloads[index])

      if persist {
         save()
      }

      return downloads[index]
   }

   /// Applies several record updates with a single write.
   ///
   /// Reconciliation can revert many records at once; one write per record would
   /// re-encode and rewrite the whole file that many times. Unknown paths are
   /// ignored.
   func update(_ records: [DownloadRecord]) {
      guard !records.isEmpty else { return }

      for record in records {
         if let index = downloads.firstIndex(where: { $0.path == record.path }) {
            downloads[index] = record
         }
      }

      save()
   }

   func remove(_ item: DownloadRecord) {
      if let index = downloads.firstIndex(where: { $0.path == item.path }) {
         downloads.remove(at: index)
      }
      save()
   }
   
   /// Decodes the persisted store.
   ///
   /// Note that one malformed element fails the whole array, discarding every other
   /// download in the file. Making that per-record is tracked in #64.
   static func load(from savePath: URL) -> [DownloadRecord] {
      do {
         let data = try Data(contentsOf: savePath)
         return try JSONDecoder().decode([DownloadRecord].self, from: data)
      } catch {
         os_log(.error, log: Log.downloadStore, "Failed to load download store: %{public}@", error.localizedDescription)
         return []
      }
   }

   private func save() {
      let encoder = JSONEncoder()
      do {
         let data = try encoder.encode(downloads)

         // `write` does not create intermediate directories, and a configured store
         // directory normally does not exist on a first launch. Without this, every
         // save fails into the `catch` below and the store is never written — silently,
         // since the in-memory array keeps serving `list` for the rest of the session.
         // Desktop creates it in `save_inner`; Android gets it from `AtomicFile`.
         try FileManager.default.createDirectory(
            at: savePath.deletingLastPathComponent(),
            withIntermediateDirectories: true
         )

         try data.write(to: savePath, options: .atomic)
      } catch {
         os_log(.error, log: Log.downloadStore, "Failed to save download store: %{public}@", error.localizedDescription)
      }
   }
}
