package org.silvermine.downloadmanager

/**
 * Decides when a progress update should be emitted: on every change of integer
 * percent when [totalBytes] is known and positive, otherwise every
 * [BYTES_THRESHOLD] bytes.
 *
 * Tracks [receivedBytes] internally, so the download loop only calls [advance]
 * with each chunk size and checks [shouldEmit]. Mirrors the Rust
 * `ProgressTracker` in `crates/download-manager/src/downloader.rs`.
 */
internal class ProgressTracker(receivedBytes: Long, val totalBytes: Long?) {
   var receivedBytes: Long = receivedBytes
      private set

   private var lastEmittedPercent: Long = percent(receivedBytes, totalBytes)
   private var lastEmittedBytes: Long = receivedBytes

   fun advance(bytesWritten: Long) {
      receivedBytes += bytesWritten
   }

   fun shouldEmit(): Boolean {
      if (totalBytes == null || totalBytes <= 0L) {
         return (receivedBytes - lastEmittedBytes) >= BYTES_THRESHOLD
      }

      val percent = percent(receivedBytes, totalBytes)
      return (percent >= 100L || percent > lastEmittedPercent)
   }

   fun isComplete(): Boolean = (totalBytes != null && receivedBytes >= totalBytes)

   fun markEmitted() {
      lastEmittedPercent = percent(receivedBytes, totalBytes)
      lastEmittedBytes = receivedBytes
   }

   companion object {
      const val BYTES_THRESHOLD = 1024L * 1024L

      private fun percent(bytes: Long, total: Long?): Long {
         if (total == null || total <= 0L) {
            return 0L
         }
         return ((bytes * 100L) / total)
      }
   }
}
