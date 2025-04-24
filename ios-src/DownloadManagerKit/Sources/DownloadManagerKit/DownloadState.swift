//
//  DownloadState.swift
//  DownloadManagerKit
//
//  Created by Matthew Richardson on 26/04/2025.
//

public enum DownloadState: String, Codable {
   case created, inProgress, paused, cancelled, completed
}
