//
//  DownloadManager.swift
//  DownloadManagerKit
//
//  Created by Matthew Richardson on 24/04/2025.
//

import Combine
import Foundation

public final class DownloadManager: NSObject, ObservableObject, URLSessionDownloadDelegate, @unchecked Sendable {
   public static let shared = DownloadManager()
   @Published public var downloads: [DownloadItem] = []

   public var downloadItemChanged: AnyPublisher<DownloadItem, Never> {
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
      if downloads.contains(where: { $0.key == key })
      {
         throw DownloadError.duplicateKey
      }
      
      let item = DownloadItem(key: key, url: url, path: path)
      downloads.append(item)
      saveState()
      emitChanged(item)
      
      return item
   }
   
   public func start(key: String) throws -> DownloadItem {
      guard let session = session else { throw DownloadError.sessionNotFound }
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey }
      guard item.state == .created else { throw DownloadError.invalidState }
      
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
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey }
      guard item.state == .created || item.state == .inProgress || item.state == .paused else { throw DownloadError.invalidState }
      
      if let task = activeTasks[key] {
         task?.cancel()
      }
      
      if let data = loadResumeData(for: item) {
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
      guard let task = activeTasks[key] else { throw DownloadError.taskNotFound }
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey }
      guard item.state == .inProgress else { throw DownloadError.invalidState }
      
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
      guard let item = downloads.first(where: { $0.key == key }) else { throw DownloadError.invalidKey }
      guard let data = loadResumeData(for: item) else { throw DownloadError.resumeDataNotFound }
      guard item.state == .paused else { throw DownloadError.invalidState }
      
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
   
   public func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
      guard let url = downloadTask.originalRequest?.url,
            let item = downloads.first(where: { $0.url == url }) else { return }

      item.progress = Double(totalBytesWritten) / Double(totalBytesExpectedToWrite) * 100
      if let index = self.downloads.firstIndex(where: {$0.key == item.key}) {
         downloads[index] = item
         emitChanged(item)
      }
   }

   public func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
      guard let url = downloadTask.originalRequest?.url,
            let item = downloads.first(where: { $0.url == url }) else { return }

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
