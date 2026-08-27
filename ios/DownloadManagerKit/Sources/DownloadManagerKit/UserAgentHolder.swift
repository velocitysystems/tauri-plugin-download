//
//  UserAgentHolder.swift
//  DownloadManagerKit
//

import Foundation

/// Holds the user agent sent with every download request.
///
/// An actor rather than a stored property on [`DownloadManager`], which is a class
/// and not isolated: the value is written from the Tauri plugin's command and read
/// from the tasks that build requests.
actor UserAgentHolder {
   private(set) var value: String?

   func set(_ userAgent: String?) {
      value = userAgent
   }
}
