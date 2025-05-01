//
//  DownloadItem.swift
//  DownloadManagerKit
//

import Foundation

/**
 A class that represents an item to be downloaded.
 Used to track the state and progress of a download operation.
 */
public class DownloadItem: ObservableObject, Identifiable, Codable {
   enum CodingKeys: CodingKey {
      case key, url, path, progress, state, resumeDataPath
   }
   
   public let key: String
   public let url: URL
   public let path: URL
   @Published public var progress: Double
   @Published public var state: DownloadState
   public var resumeDataPath: URL?
   
   init(key: String, url: URL, path: URL, progress: Double = 0.0, state: DownloadState = .created, resumeDataPath: URL? = nil) {
      self.key = key
      self.url = url
      self.path = path
      self.progress = progress
      self.state = state;
      self.resumeDataPath = resumeDataPath
   }
   
   public required init(from decoder: Decoder) throws {
      let container = try decoder.container(keyedBy: CodingKeys.self)
      key = try container.decode(String.self, forKey: .key)
      url = try container.decode(URL.self, forKey: .url)
      path = try container.decode(URL.self, forKey: .path)
      progress = try container.decode(Double.self, forKey: .progress)
      state = try container.decode(DownloadState.self, forKey: .state)
      resumeDataPath = try container.decodeIfPresent(URL.self, forKey: .resumeDataPath)
   }
   
   public func encode(to encoder: Encoder) throws {
      var container = encoder.container(keyedBy: CodingKeys.self)
      try container.encode(key, forKey: .key)
      try container.encode(url, forKey: .url)
      try container.encode(path, forKey: .path)
      try container.encode(progress, forKey: .progress)
      try container.encode(state, forKey: .state)
      try container.encode(resumeDataPath, forKey: .resumeDataPath)
   }
}
