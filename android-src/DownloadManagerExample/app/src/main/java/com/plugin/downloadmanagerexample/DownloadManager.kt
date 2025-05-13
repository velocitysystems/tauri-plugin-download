package com.plugin.downloadmanagerexample

import android.content.Context
import androidx.work.WorkManager
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow

class DownloadManager(private val context: Context) {
   private val store = DownloadStore(context)
   private val workManager = WorkManager.getInstance(context)

   private val _changed = MutableSharedFlow<DownloadItem>(0)
   val changed: SharedFlow<DownloadItem> = _changed

   private val _downloads = MutableStateFlow<List<DownloadItem>>(emptyList())
   val downloads: StateFlow<List<DownloadItem>> = _downloads

   suspend fun create(item: DownloadItem): DownloadItem {
      if (store.get(item.key) != null) {
         throw DownloadError.DuplicateKey(item.key)
      }

      store.save(item)
      _changed.emit(item)
      _downloads.value = store.getAll().values.toList()
      return item
   }

   suspend fun get(key: String): DownloadItem {
      return store.get(key)?: throw DownloadError.InvalidKey(key)
   }

   suspend fun list(): List<DownloadItem> {
      return store.getAll().values.toList()
   }

   suspend fun start(key: String): DownloadItem {
      val item = store.get(key)?: throw DownloadError.InvalidKey(key)
      if (item.state != DownloadState.CREATED) {
         throw DownloadError.InvalidState(item.state)
      }

      val request = DownloadWorker.createWorkRequest(key)
      workManager.enqueue(request)

      item.state = DownloadState.IN_PROGRESS
      store.save(item)
      _changed.emit(item)
      _downloads.value = store.getAll().values.toList()
      return item
   }

   suspend fun cancel(key: String): DownloadItem {
      val item = store.get(key)?: throw DownloadError.InvalidKey(key)
      if (!(item.state == DownloadState.CREATED || item.state == DownloadState.IN_PROGRESS || item.state == DownloadState.PAUSED)) {
         throw DownloadError.InvalidState(item.state)
      }

      workManager.cancelAllWorkByTag(key);

      item.state = DownloadState.CANCELLED
      store.save(item)
      _changed.emit(item)
      _downloads.value = store.getAll().values.toList()
      return item
   }

   suspend fun pause(key: String): DownloadItem {
      val item = store.get(key)?: throw DownloadError.InvalidKey(key)
      if (item.state != DownloadState.IN_PROGRESS) {
         throw DownloadError.InvalidState(item.state)
      }

      workManager.cancelAllWorkByTag(key);

      item.state = DownloadState.PAUSED
      store.save(item)
      _changed.emit(item)
      _downloads.value = store.getAll().values.toList()
      return item
   }

   suspend fun resume(key: String): DownloadItem {
      val item = store.get(key)?: throw DownloadError.InvalidKey(key)
      if (item.state != DownloadState.PAUSED) {
         throw DownloadError.InvalidState(item.state)
      }

      val request = DownloadWorker.createWorkRequest(key)
      workManager.enqueue(request)

      item.state = DownloadState.IN_PROGRESS
      store.save(item)
      _changed.emit(item)
      _downloads.value = store.getAll().values.toList()
      return item
   }
}
