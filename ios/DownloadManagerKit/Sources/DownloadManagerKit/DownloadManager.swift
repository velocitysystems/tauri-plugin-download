//
//  DownloadManager.swift
//  DownloadManagerKit
//

import Foundation

/// A manager class responsible for handling download operations.
/// Used to provide functionality for downloading files, tracking download progress and handling completion events.
public final class DownloadManager: NSObject {
   public static let shared = DownloadManager()

   public var changed: AsyncStream<DownloadItem> {
       AsyncStream { continuation in
           Task {
               let id = await self.downloadContinuation.add(continuation)
               continuation.onTermination = { @Sendable _ in
                  Task {
                     await self.downloadContinuation.remove(id)
                  }
               }
           }
       }
   }
   
   let downloadContinuation = DownloadContinuation()
   
   private var sessionDelegate: DownloadSessionDelegate!
   private var session: URLSession!
   private let store = DownloadStore()
   private let backgroundSessionHandler = BackgroundSessionHandler()

   override init() {
      super.init()
      sessionDelegate = DownloadSessionDelegate()
      sessionDelegate.manager = self
      
      // delegateQueue: nil creates a serial operation queue for delegate callbacks by default
      let config = URLSessionConfiguration.background(withIdentifier: Bundle.main.bundleIdentifier!)
      session = URLSession(configuration: config, delegate: sessionDelegate, delegateQueue: nil)
   }
   
   deinit {
      Task {
         await downloadContinuation.finish()
      }
   }
   
   public func setBackgroundCompletionHandler(_ handler: @escaping () -> Void) {
      Task {
         await backgroundSessionHandler.set(handler)
      }
   }
   
   /**
    Lists all download operations.

    - Returns: The list of download operations.
    */
   public func list() async -> [DownloadItem] {
      return await store.list().map { $0.toItem() }
   }
    
   /**
    Gets a download operation.

    If the download exists in the store, returns it. If not found, returns a download
    in `pending` state (not persisted to store). The caller can then call `create` to
    persist it and transition to `idle` state.

    - Parameter path: The download path.
    - Returns: The download operation.
    */
   public func get(path: URL) async -> DownloadItem {
      if let record = await store.findByPath(path) {
         return record.toItem()
      }

      return DownloadRecord(url: URL(fileURLWithPath: ""), path: path, status: .pending).toItem()
   }
   
   /**
    Creates a download operation.

    - Parameters:
      - path: The download path.
      - url: The download URL for the resource.
    - Returns: The download operation.
    */
   public func create(path: URL, url: URL) async -> DownloadActionResponse {
      if let existing = await store.findByPath(path) {
         return DownloadActionResponse(download: existing.toItem(), expectedStatus: .idle)
      }

      let record = DownloadRecord(url: url, path: path)
      await store.append(record)
      let item = await emitChanged(record)
      
      return DownloadActionResponse(download: item)
   }
   
   /**
    Starts a download operation.

    - Parameter path: The download path.
    - Returns: The download operation.
    */
   public func start(path: URL) async throws -> DownloadActionResponse {
      guard var record = await store.findByPath(path) else {
         throw DownloadError.notFound(path.path)
      }

      guard record.status == .idle else {
         return DownloadActionResponse(download: record.toItem(), expectedStatus: .inProgress)
      }
      
      // Commit InProgress before the task can call back: handleProgress ignores
      // records that are not InProgress, so a fast first callback would otherwise
      // be dropped. Rust persists the status before spawning too.
      record.setStatus(.inProgress)
      await store.update(record)

      let task = session.downloadTask(with: record.url)
      task.taskDescription = path.path
      task.resume()
      
      let item = await emitChanged(record)
      
      return DownloadActionResponse(download: item)
   }
   
   /**
    Resumes a download operation.

    - Parameter path: The download path.
    - Returns: The download operation.
    */
   public func resume(path: URL) async throws -> DownloadActionResponse {
      guard var record = await store.findByPath(path) else {
         throw DownloadError.notFound(path.path)
      }
      
      guard record.status == .paused,
            let data = loadResumeData(for: record) else {
         return DownloadActionResponse(download: record.toItem(), expectedStatus: .inProgress)
      }

      // Commit InProgress before starting the task, as in start(). The resume
      // data is already in memory, so its file can go now.
      deleteResumeData(for: record)
      record.setResumeDataPath(nil)
      record.setStatus(.inProgress)
      await store.update(record)
      
      let task = session.downloadTask(withResumeData: data)
      task.taskDescription = path.path
      task.resume()

      let item = await emitChanged(record)
      
      return DownloadActionResponse(download: item)
   }
   
   /**
    Pauses a download operation.

    - Parameter path: The download path.
    - Returns: The download operation.
    */
   public func pause(path: URL) async throws -> DownloadActionResponse {
      guard let record = await store.findByPath(path) else {
         throw DownloadError.notFound(path.path)
      }

      guard record.status == .inProgress,
            let task = await getDownloadTask(path.path) else {
         return DownloadActionResponse(download: record.toItem(), expectedStatus: .paused)
      }
      
      // Cancel task and collect resume data via callback. The callback and
      // handleError may both try to persist resume data; mutateRecord serializes
      // access through the actor so only one wins, and the duplicate is cleaned up.
      task.cancel(byProducingResumeData: { [weak self] data in
         guard let self, let data else { return }
         let savedURL = self.saveResumeData(data)
         Task {
            let updated = await self.mutateRecord(path: path) { current in
               if current.resumeDataPath == nil {
                  current.setResumeDataPath(savedURL)
               }
            }
            if let updated, updated.resumeDataPath != savedURL {
               try? FileManager.default.removeItem(at: savedURL)
            }
         }
      })

      // Persisting here is what carries the last progress tick's byte count
      // across the pause.
      let updated = await mutateRecord(path: path) { $0.setStatus(.paused) }
      let result = updated ?? record
      let item = await emitChanged(result)

      return DownloadActionResponse(download: item)
   }
   
   /**
    Cancels a download operation.

    - Parameter path: The download path.
    - Returns: The download operation.
    */
   public func cancel(path: URL) async throws -> DownloadActionResponse {
      guard var record = await store.findByPath(path) else {
         throw DownloadError.notFound(path.path)
      }

      guard record.status == .idle || record.status == .inProgress || record.status == .paused else {
         return DownloadActionResponse(download: record.toItem(), expectedStatus: .canceled)
      }
      
      if let task = await getDownloadTask(path.path) {
         task.cancel()
      }
      
      if let _ = loadResumeData(for: record) {
         deleteResumeData(for: record)
         record.setResumeDataPath(nil)
      }
      
      record.setStatus(.canceled)
      await store.remove(record)
      let item = await emitChanged(record)
      
      return DownloadActionResponse(download: item)
   }

   /**
    Handler for download progress updates. Called by DownloadSessionDelegate.

    - Parameters:
      - url: The URL of the download.
      - totalBytesWritten: The total number of bytes transferred so far.
      - totalBytesExpectedToWrite: The expected length of the file, or a negative
        value when the server did not supply a content length.
    */
   func handleProgress(url: URL, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) async {
      guard var record = await store.findByUrl(url),
            record.status == .inProgress else { return }

      let receivedBytes = UInt64(max(totalBytesWritten, 0))
      let totalBytes: UInt64? = totalBytesExpectedToWrite > 0 ? UInt64(totalBytesExpectedToWrite) : nil

      // Record a known total independently of the throttle below. handleFinished
      // reads the total back off the record, so a download that completes without
      // ever passing the throttle — a small file whose first callback is already
      // at 100% — would otherwise report no total. Mirrors the header-time
      // persist in the Rust downloader.
      if let totalBytes, record.totalBytes != totalBytes {
         record.setBytes(received: record.receivedBytes, total: totalBytes)
         await store.update(record, persist: true)
      }
      
      // Every emission writes the byte count back to the store, so the stored
      // record is itself the last-emitted baseline. That keeps throttle state off
      // this class, which would otherwise need synchronizing across callbacks.
      let tracker = ProgressTracker(
         lastEmittedBytes: record.receivedBytes,
         receivedBytes: receivedBytes,
         totalBytes: totalBytes
      )

      // Completion is reported by handleFinished, which sizes the landed file.
      guard tracker.shouldEmit, !tracker.isComplete else { return }

      record.setBytes(received: receivedBytes, total: totalBytes)
      await store.update(record, persist: false)
      await emitChanged(record)
   }

   /**
    Handler for download completion. Called by DownloadSessionDelegate.
    The file has already been moved to a temp location by the delegate.

    - Parameters:
      - url: The URL of the download.
      - location: The temporary location of the downloaded file.
    */
   func handleFinished(url: URL, location: URL) async {
      guard var record = await store.findByUrl(url) else {
         try? FileManager.default.removeItem(at: location)
         return
      }

      // Ensure parent directory exists.
      let parentDirectory = record.path.deletingLastPathComponent()
      if !FileManager.default.fileExists(atPath: parentDirectory.path) {
         try? FileManager.default.createDirectory(at: parentDirectory, withIntermediateDirectories: true)
      }

      // Remove existing item (if found) and move downloaded item to destination path.
      try? FileManager.default.removeItem(at: record.path)
      try? FileManager.default.moveItem(at: location, to: record.path)

      // Size the file that landed. URLSession's counters already include the
      // resume offset, so this agrees with them; it is here because the file on
      // disk is the authority on what was actually written, and the last progress
      // callback may have been throttled away before completion.
      let receivedBytes = DownloadManager.fileSize(at: record.path) ?? record.receivedBytes

      record.setBytes(received: receivedBytes, total: record.totalBytes)
      record.setStatus(.completed)
      await store.remove(record)
      await emitChanged(record)
   }
   
   /**
    Handler for download errors. Called by DownloadSessionDelegate.

    - Parameters:
      - url: The URL of the download.
      - error: An error object indicating how the transfer failed, or nil if successful.
    */
   func handleError(url: URL, error: Error?) async {
      guard let error = error,
            let record = await store.findByUrl(url) else { return }
      
      // Cancellation with resume data. For user-invoked pauses, pause() may have
      // already persisted resume data. The atomic mutate ensures only one path wins.
      if let data = (error as NSError).userInfo[NSURLSessionDownloadTaskResumeData] as? Data {
         let savedURL = saveResumeData(data)
         
         let updated = await mutateRecord(path: record.path) { current in
            current.setStatus(.paused)
            if current.resumeDataPath == nil {
               current.setResumeDataPath(savedURL)
            }
         }
         
         // If the store already had resumeDataPath, our file is a duplicate.
         if let updated, updated.resumeDataPath != savedURL {
            try? FileManager.default.removeItem(at: savedURL)
         }
         
         if let updated {
            await emitChanged(updated)
         }
         return
      }
      
      // Download failed - update status and clean up
      deleteResumeData(for: record)
      if let updated = await mutateRecord(path: record.path, { $0.setStatus(.canceled) }) {
         await store.remove(updated)
         await emitChanged(updated)
      }
   }
   
   /**
    Handler for background session completion. Called by DownloadSessionDelegate.
    The completion handler must be called to let the system know we're done processing.
    If the handler hasn't been set yet (race condition), defers until it is set.
    */
   func handleBackgroundSessionComplete() {
      Task {
         await backgroundSessionHandler.handleComplete()
      }
   }

   func loadResumeData(for record: DownloadRecord) -> Data? {
      guard let url = record.resumeDataPath else { return nil }
      return try? Data(contentsOf: url)
   }
   
   func saveResumeData(_ data: Data) -> URL {
      let filename = UUID().uuidString + ".resumedata"
      let url = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent(filename)
      try? data.write(to: url)
      return url
   }
   
   func deleteResumeData(for record: DownloadRecord) {
      guard let url = record.resumeDataPath else { return }
      try? FileManager.default.removeItem(at: url)
   }
   
   private func mutateRecord(path: URL, _ body: (inout DownloadRecord) -> Void) async -> DownloadRecord? {
      guard var record = await store.findByPath(path) else { return nil }
      body(&record)
      await store.update(record, persist: true)
      return record
   }
   
   func getDownloadTask(_ path: String) async -> URLSessionDownloadTask? {
      let tasks = await session.allTasks
      return tasks.compactMap { $0 as? URLSessionDownloadTask }
         .first { $0.taskDescription == path }
   }

   /// Emits the public payload derived from `record` and returns it for reuse in
   /// a `DownloadActionResponse`.
   @discardableResult
   func emitChanged(_ record: DownloadRecord) async -> DownloadItem {
      let item = record.toItem()
      await downloadContinuation.yield(item)
      return item
   }

   static func fileSize(at url: URL) -> UInt64? {
      guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
            let size = attributes[.size] as? NSNumber else {
         return nil
      }
      return size.uint64Value
   }
}
