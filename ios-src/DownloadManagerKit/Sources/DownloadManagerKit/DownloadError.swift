//
//  DownloadError.swift
//  DownloadManagerKit
//
//  Created by Matthew Richardson on 29/04/2025.
//

enum DownloadError: Error {
   case duplicateKey
   case invalidKey
   case invalidState
   case resumeDataNotFound
   case sessionNotFound
   case taskNotFound
}
