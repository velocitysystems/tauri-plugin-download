//
//  StoreLocation.swift
//  DownloadManagerKit
//

import Foundation
import os.log

/// Raised when the store directory cannot be applied.
public enum StoreLocationError: LocalizedError {
   /// The store had already opened its file, so there was nothing left to move.
   case alreadyResolved(URL)

   public var errorDescription: String? {
      switch self {
         case .alreadyResolved(let url):
            return "Store directory set to \(url.path) after the store was opened"
      }
   }
}

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

   /// The directory used when none is configured: the app's Application Support
   /// directory.
   ///
   /// May not exist on a first launch; `DownloadStore.save` creates it.
   static var defaultDirectory: URL {
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
   }

   /// Sets the directory holding the store.
   ///
   /// Creates the directory, and that is the point of doing it here rather than leaving
   /// it to the first write: [`DownloadStore.save`] can only log a failure, so a
   /// directory the app cannot create would cost every record with nothing reported to
   /// anyone. Failing on the call that configures it puts the error where the caller
   /// can still act on it — desktop refuses the same misconfiguration in
   /// `validate_store_dir`, Android through `AtomicFile`, and this is the iOS half.
   ///
   /// Only effective before [`savePath`] is first read: once the store has opened its
   /// file there is nothing left to move. A late call throws rather than reporting a
   /// success it did not deliver — an assertion would be compiled out under `-O`.
   ///
   /// - Parameter url: The directory to persist the store in.
   /// - Throws: [`StoreLocationError.alreadyResolved`] if the store has already opened
   ///   its file, or the underlying error if the directory cannot be created.
   static func set(_ url: URL) throws {
      guard !isResolved else {
         throw StoreLocationError.alreadyResolved(url)
      }

      // Before the assignment, so a directory that cannot be created leaves the store
      // where it was rather than pointing it somewhere nothing can be written.
      try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)

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
