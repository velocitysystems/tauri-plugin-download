//
//  ProgressTracker.swift
//  DownloadManagerKit
//

import Foundation

/// Decides when a progress update should be emitted: on every change of integer
/// percent when `totalBytes` is known and positive, otherwise every
/// `bytesThreshold` bytes.
///
/// Mirrors the emission rule of the Rust `ProgressTracker` in
/// `crates/download-manager/src/downloader.rs`. Rust accumulates chunk sizes as
/// it streams; URLSession reports a cumulative count per callback, so this
/// compares against the last count emitted instead of holding state.
struct ProgressTracker {
   static let bytesThreshold: UInt64 = 1024 * 1024

   private let receivedBytes: UInt64
   private let totalBytes: UInt64?
   private let lastEmittedBytes: UInt64

   init(lastEmittedBytes: UInt64, receivedBytes: UInt64, totalBytes: UInt64?) {
      self.receivedBytes = receivedBytes
      self.totalBytes = totalBytes
      self.lastEmittedBytes = lastEmittedBytes
   }

   var shouldEmit: Bool {
      // A restarted download (rejected resume data, or a server that stops
      // honouring Range) reports fewer bytes than the store last saw. Emit so
      // the baseline resets, rather than going quiet until it climbs back past
      // the old count. Also guards the unsigned subtraction below. Rust owns its
      // accumulator, so it never sees this.
      guard receivedBytes >= lastEmittedBytes else {
         return true
      }

      guard let total = totalBytes, total > 0 else {
         return (receivedBytes - lastEmittedBytes) >= Self.bytesThreshold
      }

      let percent = ((receivedBytes * 100) / total)
      let lastEmittedPercent = ((lastEmittedBytes * 100) / total)

      // `percent >= 100` never emits — the call site guards on it. Inert here, where
      // no store poll follows; kept so the three trackers read alike.
      return (percent >= 100 || percent > lastEmittedPercent)
   }

   var isComplete: Bool {
      // Positive, not merely non-nil — a zero total would otherwise read as
      // complete from the first byte. Mirrors the Android guard.
      guard let total = totalBytes, total > 0 else {
         return false
      }
      return receivedBytes >= total
   }
}
