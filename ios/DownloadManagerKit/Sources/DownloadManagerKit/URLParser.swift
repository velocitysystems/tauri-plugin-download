//
//  URLParser.swift
//  DownloadManagerKit
//

import Foundation

/// Parses and validates a download path string.
/// Checks that the path is not empty, is an absolute path and contains a filename.
/// Returns it unchanged, as it is the download's identity.
public func parsePath(_ pathString: String) throws -> String {
   if pathString.isEmpty {
       throw DownloadError.invalidPath("Path cannot be empty")
   }

   guard pathString.hasPrefix("/") else {
       throw DownloadError.invalidPath("Path must be absolute")
   }

   let filename = URL(fileURLWithPath: pathString).lastPathComponent
   if filename.isEmpty || filename == "/" {
       throw DownloadError.invalidPath("Path must have a filename")
   }
   
   return pathString
}

/// Parses and validates a download URL string.
/// Checks that the URL is valid, has a valid scheme (http or https) and has a valid host.
public func parseURL(_ urlString: String) throws -> URL {
   guard let url = URL(string: urlString) else {
      throw DownloadError.invalidURL("Invalid URL: \(urlString)")
   }
   
   let scheme = url.scheme?.lowercased()
   guard scheme == "http" || scheme == "https" else {
      throw DownloadError.invalidURL("Invalid URL scheme '\(scheme ?? "none")': must be http or https")
   }
   
   guard let host = url.host, !host.isEmpty else {
      throw DownloadError.invalidURL("URL must have a host")
   }
   
   return url
}
