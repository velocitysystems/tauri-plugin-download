package org.silvermine.downloadmanager

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Emit-only payload sent to the frontend. Built from a [DownloadRecord] with
 * `progress` derived from [receivedBytes] / [totalBytes]; never persisted, so
 * nothing internal to the download machinery leaks into the frontend contract.
 * [totalBytes] is `null` when the server supplied no content length, and
 * serializes as an explicit JSON `null`.
 *
 * No property carries a default, so every key is written whatever `encodeDefaults`
 * the caller's `Json` uses — the payload spells out the full contract even though
 * the TypeScript layer coalesces missing keys (attachDownload in
 * `guest-js/actions.ts`).
 */
@Serializable
data class DownloadItem(
   @SerialName("url")
   val url: String,

   @SerialName("path")
   val path: String,

   @SerialName("receivedBytes")
   val receivedBytes: Long,

   @SerialName("totalBytes")
   val totalBytes: Long?,

   @SerialName("progress")
   val progress: Double,

   @SerialName("status")
   val status: DownloadStatus,
)
