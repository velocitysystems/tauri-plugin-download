package org.silvermine.downloadmanager

import kotlinx.serialization.Required
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Options fixed when a download is created: calling [DownloadManager.create] again
 * for an existing path returns the stored record with its original options.
 *
 * Mirrors the Rust `CreateOptions` in `crates/download-manager/src/models.rs`.
 *
 * @property allowMetered Whether the download may transfer on metered connections.
 *    When `false` the work request requires [androidx.work.NetworkType.UNMETERED], so
 *    WorkManager holds the download rather than failing it. The default is what
 *    [DownloadManager.create] applies when a caller states none; [Required] settles it
 *    on the wire both ways — always encoded, never defaulted on decode.
 */
@Serializable
data class CreateOptions(
   @Required
   @SerialName("allowMetered")
   val allowMetered: Boolean = true,
)
