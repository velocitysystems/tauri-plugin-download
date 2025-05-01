//
//  DownloadManager.swift
//  DownloadManagerKit
//

import Combine
import Foundation

/**
 A manager class responsible for handling download operations.

 This class provides functionality for downloading files from URLs, tracking download progress,
 and handling completion events. It adopts the `ObservableObject` protocol to support SwiftUI data binding and
 the `URLSessionDownloadDelegate` to manage download tasks.
 */
public final class DownloadManager: NSObject, ObservableObject, URLSessionDownloadDelegate, @unchecked Sendable {
   public static let shared = DownloadManager()
   @Published public var downloads: [DownloadItem] = []

   public var changed: AnyPublisher<DownloadItem, Never> {
      downloadItemSubject.eraseToAnyPublisher()
   }

   let savePath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("downloads.json")
   let queue = DispatchQueue(label: Bundle.main.bundleIdentifier!, attributes: .concurrent)
   let downloadItemSubject = PassthroughSubject<DownloadItem, Never>()
   var session: URLSession?
   var activeTasks: [String: URLSessionDownloadTask?] = [:]

   override init() {
      super.init()
      let config = URLSessionConfiguration.background(withIdentifier: Bundle.main.bundleIdentifier!)
      session = URLSession(configuration: config, delegate: self, delegateQueue: .main)
      loadState()
   }

   public func create(key: String, url: URL, path: URL) throws -> DownloadItem {
      guard !downloads.contains(where: { $0.key == key }) else { throw DownloadError.duplicateKey(key) }
      let item = DownloadItem(key: key, url: url, path: path)
      downloads.append(item)
      saveState()
      emitChanged(item)
      
      return item
   }
    
   public func get(key: String) throws -> DownloadItem {
       guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey(key) }
       return item
   }
    
   public func list() -> [DownloadItem] {
       return downloads
   }
   
   public func start(key: String) throws -> DownloadItem {
      guard let session = session else { throw DownloadError.sessionNotFound }
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey(key) }
      guard item.state == .created else { throw DownloadError.invalidState(item.state) }
      
      let task = session.downloadTask(with: item.url)
      task.resume()
      activeTasks[key] = task
      
      item.state = .inProgress
      if let index = downloads.firstIndex(where: {$0.key == key}) {
         downloads[index] = item
         saveState()
         emitChanged(item)
      }
      
      return item
   }
   
   public func cancel(key: String) throws -> DownloadItem {
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey(key) }
      guard item.state == .created || item.state == .inProgress || item.state == .paused else { throw DownloadError.invalidState(item.state) }
      
      if let task = activeTasks[key] {
         task?.cancel()
      }
      
      if let _ = loadResumeData(for: item) {
         deleteResumeData(for: item)
      }
      
      item.state = .cancelled
      if let index = self.downloads.firstIndex(where: {$0.key == key}) {
         downloads.remove(at: index)
         saveState()
         emitChanged(item)
      }
      
      return item
   }
   
   public func pause(key: String) throws -> DownloadItem {
      guard let task = activeTasks[key] else { throw DownloadError.sessionDownloadTaskNotFound(key) }
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey(key) }
      guard item.state == .inProgress else { throw DownloadError.invalidState(item.state) }
      
      task?.cancel(byProducingResumeData: { data in
         if let data = data {
            self.saveResumeData(data, for: item)
            self.activeTasks[key] = nil
         }
      })
      
      item.state = .paused
      if let index = self.downloads.firstIndex(where: {$0.key == key}) {
         downloads[index] = item
         saveState()
         emitChanged(item)
      }
      
      return item
   }
   
   public func resume(key: String) throws -> DownloadItem {
      guard let session = session else { throw DownloadError.sessionNotFound }
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey(key) }
      guard let data = loadResumeData(for: item) else { throw DownloadError.resumeDataNotFound(key) }
      guard item.state == .paused else { throw DownloadError.invalidState(item.state) }
      
      let task = session.downloadTask(withResumeData: data)
      task.resume()
      deleteResumeData(for: item)
      activeTasks[key] = task
      
      item.state = .inProgress
      if let index = self.downloads.firstIndex(where: {$0.key == key}) {
         downloads[index] = item
         saveState()
         emitChanged(item)
      }
      
      return item
   }

   /**
    URLSession delegate method called periodically to inform about download progress.
    This method is called periodically during a download operation to provide information about the amount of data that has been downloaded.

    - Parameters:
      - session: The URL session containing the download task.
      - downloadTask: The download task that provided data.
      - bytesWritten: The number of bytes that were written in the latest write operation.
      - totalBytesWritten: The total number of bytes transferred so far.
      - totalBytesExpectedToWrite: The expected length of the file, as provided by the Content-Length header. If this header was not provided, the value is NSURLSessionTransferSizeUnknown.
    */
   public func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
      guard let url = downloadTask.originalRequest?.url,
            let item = downloads.first(where: { $0.url == url }) else { return }

      item.progress = Double(totalBytesWritten) / Double(totalBytesExpectedToWrite) * 100
      if let index = self.downloads.firstIndex(where: {$0.key == item.key}) {
         downloads[index] = item
         emitChanged(item)
      }
   }

   /**
    URLSession delegate method called when the download task has finished downloading.
    This method is called when the download task has completed successfully and the downloaded file is available at the specified location.

    - Parameters:
      - session: The URL session containing the download task.
      - downloadTask: The download task that finished downloading.
      - location: The temporary location of the downloaded file.
    */
   public func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
      guard let url = downloadTask.originalRequest?.url,
            let item = downloads.first(where: { $0.url == url }) else { return }

      // Ensure parent directory exists.
      let parentDirectory = item.path.deletingLastPathComponent()
      if !FileManager.default.fileExists(atPath: parentDirectory.path) {
         try? FileManager.default.createDirectory(at: parentDirectory, withIntermediateDirectories: true)
      }

      // Remove existing item (if found) and move downloaded item to destination path.
      try? FileManager.default.removeItem(at: item.path)
      try? FileManager.default.moveItem(at: location, to: item.path)
      activeTasks[item.key] = nil
      
      item.state = .completed
      if let index = self.downloads.firstIndex(where: {$0.key == item.key}) {
         downloads.remove(at: index)
         saveState()
         emitChanged(item)
      }
   }

   func loadResumeData(for item: DownloadItem) -> Data? {
      guard let url = item.resumeDataPath else { return nil }
      return try? Data(contentsOf: url)
   }
   
   func saveResumeData(_ data: Data, for item: DownloadItem) {
      let filename = UUID().uuidString + ".resumedata"
      let url = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent(filename)
      try? data.write(to: url)
      item.resumeDataPath = url
      saveState()
   }
   
   func deleteResumeData(for item: DownloadItem) {
      guard let url = item.resumeDataPath else { return }
      try? FileManager.default.removeItem(at: url)
      item.resumeDataPath = nil
   }
   
   func loadState() {
      queue.sync {
         let decoder = JSONDecoder()
         if let data = try? Data(contentsOf: savePath),
            let saved = try? decoder.decode([DownloadItem].self, from: data) {
            downloads = saved
         }
      }
   }

   func saveState() {
      queue.sync {
         let encoder = JSONEncoder()
         if let data = try? encoder.encode(downloads) {
            try? data.write(to: savePath)
         }
      }
   }
   
   func emitChanged(_ item: DownloadItem) {
      downloadItemSubject.send(item)
   }
}
