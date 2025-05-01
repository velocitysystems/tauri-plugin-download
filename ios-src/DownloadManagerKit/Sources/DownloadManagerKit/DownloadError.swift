//
//  DownloadError.swift
//  DownloadManagerKit
//

/// Represents possible errors that can occur during download operations.
enum DownloadError: Error {
   case duplicateKey(String)
   case invalidKey(String)
   case invalidState(DownloadState)
   case resumeDataNotFound(String)
   case sessionNotFound
   case sessionDownloadTaskNotFound(String)

   var localizedDescription: String {
      switch self {
         case .duplicateKey(let key):
            return "Download with key '\(key)' already exists"
        case .invalidKey(let key):
            return "No download found with key '\(key)'"
         case .invalidState(let state):
            return "Invalid state transition from \(state)"
         case .resumeDataNotFound(let key):
            return "Resume data not found for key '\(key)'"
         case .sessionNotFound:
            return "URLSession not found"
         case .sessionDownloadTaskNotFound(let key):
            return "URLSessionDownloadTask not found for key '\(key)'"
      }
   }
}
