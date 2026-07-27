package org.silvermine.downloadmanager

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Persisted download record, stored in `downloads.json`. Mirrors the Rust
 * `DownloadRecord` in `crates/download-manager/src/models.rs`.
 *
 * Carries no `progress` — that is derived, and lives only on [DownloadItem],
 * the type sent to the frontend.
 */
@Serializable
internal data class DownloadRecord(
   @SerialName("url")
   val url: String,

   @SerialName("path")
   val path: String,

   @SerialName("receivedBytes")
   val receivedBytes: Long = 0L,

   /** `null` when the server did not supply a content length. */
   @SerialName("totalBytes")
   val totalBytes: Long? = null,

   @SerialName("status")
   val status: DownloadStatus = DownloadStatus.Idle,
) {
   fun withBytes(receivedBytes: Long, totalBytes: Long?): DownloadRecord =
      copy(receivedBytes = receivedBytes, totalBytes = totalBytes)

   fun withStatus(newStatus: DownloadStatus): DownloadRecord =
      copy(status = newStatus)

   /**
    * Builds the public payload, computing `progress` from the byte counts.
    *
    * A completed download always reports 100%, even without a content length.
    * In flight without one it reports 0%, since [receivedBytes] is then the
    * only meaningful signal.
    */
   fun toItem(): DownloadItem {
      val progress = when {
         status == DownloadStatus.Completed -> 100.0
         totalBytes != null && totalBytes > 0L ->
            ((receivedBytes.toDouble() / totalBytes.toDouble()) * 100.0).coerceIn(0.0, 100.0)
         else -> 0.0
      }

      return DownloadItem(
         url = url,
         path = path,
         receivedBytes = receivedBytes,
         totalBytes = totalBytes,
         progress = progress,
         status = status,
      )
   }
}
