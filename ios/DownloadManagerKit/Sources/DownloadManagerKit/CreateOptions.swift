//
//  CreateOptions.swift
//  DownloadManagerKit
//

import Foundation

/// Options fixed when a download is created: calling `create` again for an
/// existing path returns the stored record with its original options.
///
/// Mirrors the Rust `CreateOptions` in `crates/download-manager/src/models.rs`.
public struct CreateOptions: Codable, Sendable, Equatable {
   /// Whether the download may transfer on metered or constrained connections.
   ///
   /// When `false` the task's `URLRequest` refuses cellular, expensive, and
   /// constrained (Low Data Mode) paths, so the background session holds it until
   /// an eligible network appears rather than failing it.
   public var allowMetered: Bool

   /// The default is what `DownloadManager.create` applies when a caller states
   /// no policy. Decoding has no such fallback.
   public init(allowMetered: Bool = true) {
      self.allowMetered = allowMetered
   }
}
