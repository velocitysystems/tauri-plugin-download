//
//  AppDelegate.swift
//  DownloadManagerExample
//

import UIKit

class AppDelegate: NSObject, UIApplicationDelegate {
   var backgroundSessionCompletionHandler: (() -> Void)?

   func application(_ application: UIApplication, handleEventsForBackgroundURLSession identifier: String, completionHandler: @escaping () -> Void) {
      backgroundSessionCompletionHandler = completionHandler
   }
}
