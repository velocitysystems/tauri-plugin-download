//
//  AppDelegate.swift
//  DownloadManagerExample
//
//  Created by Matthew Richardson on 25/04/2025.
//

import UIKit

class AppDelegate: NSObject, UIApplicationDelegate {
   var backgroundSessionCompletionHandler: (() -> Void)?

   func application(_ application: UIApplication, handleEventsForBackgroundURLSession identifier: String, completionHandler: @escaping () -> Void) {
      backgroundSessionCompletionHandler = completionHandler
   }
}
