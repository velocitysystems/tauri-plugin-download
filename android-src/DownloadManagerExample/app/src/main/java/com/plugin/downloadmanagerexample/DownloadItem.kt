package com.plugin.downloadmanagerexample

data class DownloadItem(
   val key: String,
   val url: String,
   val path: String,
   var progress: Int = 0,
   var state: DownloadState = DownloadState.CREATED,
   var downloadedBytes: Long = 0
)
