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
      throw DownloadError.invalidPath("path cannot be empty")
   }

   guard pathString.hasPrefix("/") else {
      throw DownloadError.invalidPath("path must be absolute")
   }

   guard fileName(pathString) != nil else {
      throw DownloadError.invalidPath("path must have a filename")
   }
   
   return pathString
}

/// The path's last component, or `nil` for "/" or a trailing "..", as Rust's
/// `Path::file_name()` does. A string check: nothing is resolved.
private func fileName(_ path: String) -> String? {
   let components = path.split(separator: "/").filter { $0 != "." }

   guard let last = components.last else {
      return nil
   }

   return last == ".." ? nil : String(last)
}

/// Parses and validates a download URL string.
/// Checks that the URL is valid, has a valid scheme (http or https) and has a valid host.
public func parseURL(_ urlString: String) throws -> URL {
   if urlString.isEmpty {
      throw DownloadError.invalidURL("URL cannot be empty")
   }

   guard let url = URL(string: urlString) else {
      throw DownloadError.invalidURL("Invalid URL: \(urlString)")
   }
   
   // Without a scheme the string is not an absolute URL, which is where Rust's parser
   // stops too, so it gets the same message rather than a scheme error.
   guard let scheme = url.scheme?.lowercased() else {
      throw DownloadError.invalidURL("Invalid URL: \(urlString)")
   }

   guard scheme == "http" || scheme == "https" else {
      throw DownloadError.invalidURL("Invalid URL scheme '\(scheme)': must be http or https")
   }
   
   guard let host = url.host, !host.isEmpty else {
      throw DownloadError.invalidURL("URL must have a host")
   }

   // Refused, not forwarded: credentials would be persisted in the store and logged
   // with every progress line. After the host, so all three platforms agree on which
   // error a URL failing both gets.
   guard url.user == nil, url.password == nil else {
      throw DownloadError.invalidURL("URL must not contain credentials")
   }

   return url
}
