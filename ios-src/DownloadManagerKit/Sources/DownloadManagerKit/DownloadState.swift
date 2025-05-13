//
//  DownloadState.swift
//  DownloadManagerKit
//

/// Represents the various states of a download item.
public enum DownloadState: String, Codable, Sendable {
   case created, inProgress, paused, cancelled, completed
}
