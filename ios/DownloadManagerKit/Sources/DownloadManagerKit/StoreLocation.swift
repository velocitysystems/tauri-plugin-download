//
//  StoreLocation.swift
//  DownloadManagerKit
//

import Foundation
import os.log

/// Where the download store is persisted.
///
/// A type-level setting rather than a parameter on [`DownloadManager`], which is a
/// singleton built by whichever caller reaches `shared` first — and whose store opens
/// its file in its own initializer. The directory therefore has to be settled *before*
/// construction, and cannot be handed to it.
///
/// Not synchronized, because the two accesses are ordered rather than concurrent: the
/// Tauri plugin's `configure` command sets the directory before it first touches the
/// manager, and every later reader gets the already-constructed `shared`. The ordering
/// comes from the happens-before edge of enqueuing that work, not from the two accesses
/// sharing a thread — `set` runs on the plugin's IPC queue and the manager is built on
/// the cooperative pool. A lock would imply a concurrency this design forbids.
enum StoreLocation {
   private static let filename = "downloads.json"

   private static var directory = defaultDirectory
   private static var isResolved = false

   /// The directory used when none is configured: the app's Documents directory.
   static var defaultDirectory: URL {
      FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
   }

   /// Sets the directory holding the store.
   ///
   /// Only effective before [`savePath`] is first read: once the store has opened its
   /// file there is nothing left to move. A late call traps under `-Onone`, so a debug
   /// build surfaces the mistake; under `-O` `assertionFailure` is compiled out and the
   /// call degrades to the log line above and no change of directory. The guard catches
   /// the misuse during development rather than in the build that ships.
   ///
   /// - Parameter url: The directory to persist the store in.
   static func set(_ url: URL) {
      guard !isResolved else {
         os_log(
            .error,
            log: Log.downloadStore,
            "Store directory set after the store was opened; ignoring %{public}@",
            url.path
         )
         assertionFailure("StoreLocation.set called after the store was opened")
         return
      }

      directory = url
   }

   /// The store file's full path, fixing the directory as a side effect.
   ///
   /// - Returns: The path the store is persisted at.
   static func savePath() -> URL {
      isResolved = true

      return directory.appendingPathComponent(filename)
   }

   /// Restores the unconfigured state.
   ///
   /// Test-only: the state is process-wide, so without this one test's directory would
   /// leak into the next and the ordering assertion would fire on the second `set`.
   static func resetForTesting() {
      directory = defaultDirectory
      isResolved = false
   }
}
