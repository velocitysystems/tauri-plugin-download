//
//  DownloadManagerExampleApp.swift
//  DownloadManagerExample
//
//  Created by Matthew Richardson on 25/04/2025.
//

import SwiftUI

@main
struct DownloadManagerExampleApp: App {
   @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
   var body: some Scene {
      WindowGroup {
         DownloadsView()
      }
   }
}
