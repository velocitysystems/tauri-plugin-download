package com.plugin.downloadmanagerexample

sealed class DownloadError(message: String) : Exception(message) {
   class DuplicateKey(val key: String) : DownloadError("Download with key '$key' already exists")
   class InvalidKey(val key: String) : DownloadError("No download found with key '$key'")
   class InvalidState(val state: DownloadState) : DownloadError("Invalid state transition from $state")
}
