package org.silvermine.downloadmanager

import android.content.Context
import android.util.Log
import androidx.work.Constraints
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.workDataOf
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import java.io.File

/**
 * A manager class responsible for handling download operations.
 * Provides functionality for downloading files, tracking download progress and handling completion events.
 *
 * Mirrors the iOS DownloadManager and Rust Download<R> API surface.
 */
class DownloadManager private constructor(context: Context) {
   internal val store = DownloadStore(context)
   private val workManager = WorkManager.getInstance(context)

   private val _changed = MutableSharedFlow<DownloadItem>(
      extraBufferCapacity = 64,
      onBufferOverflow = BufferOverflow.DROP_OLDEST,
   )

   /**
    * A flow that emits download items whenever their state changes.
    * Mirrors the iOS `changed: AsyncStream<DownloadItem>`.
    */
   val changed: SharedFlow<DownloadItem> = _changed.asSharedFlow()

   init {
      reconcileStoreOnInit()
   }

   /**
    * Reconciles the store on initialization.
    * Updates the state of any download operations which are still marked as "In Progress".
    * This can occur if the application was terminated before a download was completed.
    * Mirrors the Rust Download.init() method.
    */
   private fun reconcileStoreOnInit() {
      val records = store.list()
      for (record in records) {
         if (record.status == DownloadStatus.InProgress) {
            // Progress updates avoid disk writes, so the persisted byte count is
            // stale by whatever was in flight. Only the temp file's length says
            // what survived. totalBytes is kept — headers persisted it.
            val tempFile = File("${record.path}${DownloadWorker.DOWNLOAD_SUFFIX}")
            val updated = if (tempFile.exists()) {
               record.withBytes(tempFile.length(), record.totalBytes).withStatus(DownloadStatus.Paused)
            } else {
               record.withBytes(0L, record.totalBytes).withStatus(DownloadStatus.Idle)
            }

            store.update(updated)
            Log.d(TAG, "[${File(record.path).name}] Reconciled to ${updated.status}")
         }
      }
   }

   /**
    * Lists all download operations.
    *
    * @return The list of download operations.
    */
   fun list(): List<DownloadItem> = store.list().map { it.toItem() }

   /**
    * Gets a download operation.
    *
    * If the download exists in the store, returns it. If not found, returns a download
    * in `Pending` state (not persisted to store). The caller can then call `create` to
    * persist it and transition to `Idle` state.
    *
    * @param path The download path.
    * @return The download operation.
    */
   fun get(path: String): DownloadItem {
      val existing = store.findByPath(path)
      if (existing != null) return existing.toItem()

      return DownloadRecord(
         url = "",
         path = path,
         status = DownloadStatus.Pending,
      ).toItem()
   }

   /**
    * Creates a download operation.
    *
    * @param path The download path.
    * @param url The download URL for the resource.
    * @return The download action response.
    */
   @Synchronized
   fun create(path: String, url: String): DownloadActionResponse {
      val existing = store.findByPath(path)
      if (existing != null) {
         return DownloadActionResponse.withExpectedStatus(existing.toItem(), DownloadStatus.Idle)
      }

      val record = DownloadRecord(url = url, path = path)
      store.append(record)

      return DownloadActionResponse.new(emitChanged(record))
   }

   /**
    * Starts a download operation.
    *
    * @param path The download path.
    * @return The download action response.
    * @throws DownloadException if the download is not found.
    */
   @Synchronized
   fun start(path: String): DownloadActionResponse {
      val record = store.findByPath(path)
         ?: throw DownloadException.NotFound(path)

      if (record.status != DownloadStatus.Idle) {
         return DownloadActionResponse.withExpectedStatus(record.toItem(), DownloadStatus.InProgress)
      }

      val updated = record.withStatus(DownloadStatus.InProgress)
      store.update(updated)
      val item = emitChanged(updated)
      enqueueDownload(record)

      return DownloadActionResponse.new(item)
   }

   /**
    * Resumes a download operation.
    *
    * @param path The download path.
    * @return The download action response.
    * @throws DownloadException if the download is not found.
    */
   @Synchronized
   fun resume(path: String): DownloadActionResponse {
      val record = store.findByPath(path)
         ?: throw DownloadException.NotFound(path)

      if (record.status != DownloadStatus.Paused) {
         return DownloadActionResponse.withExpectedStatus(record.toItem(), DownloadStatus.InProgress)
      }

      val updated = record.withStatus(DownloadStatus.InProgress)
      store.update(updated)
      val item = emitChanged(updated)
      enqueueDownload(record)

      return DownloadActionResponse.new(item)
   }

   /**
    * Pauses a download operation.
    *
    * @param path The download path.
    * @return The download action response.
    * @throws DownloadException if the download is not found.
    */
   @Synchronized
   fun pause(path: String): DownloadActionResponse {
      val record = store.findByPath(path)
         ?: throw DownloadException.NotFound(path)

      if (record.status != DownloadStatus.InProgress) {
         return DownloadActionResponse.withExpectedStatus(record.toItem(), DownloadStatus.Paused)
      }

      // Update status to paused — the DownloadWorker checks the store status
      // on each progress tick and will stop reading when it sees Paused. This
      // also persists the byte count from the last tick.
      val updated = record.withStatus(DownloadStatus.Paused)
      store.update(updated)
      val item = emitChanged(updated)

      // Also cancel the WorkManager work to stop the worker promptly.
      workManager.cancelUniqueWork(workName(path))

      return DownloadActionResponse.new(item)
   }

   /**
    * Cancels a download operation.
    *
    * @param path The download path.
    * @return The download action response.
    * @throws DownloadException if the download is not found.
    */
   @Synchronized
   fun cancel(path: String): DownloadActionResponse {
      val record = store.findByPath(path)
         ?: throw DownloadException.NotFound(path)

      if (record.status != DownloadStatus.Idle &&
         record.status != DownloadStatus.InProgress &&
         record.status != DownloadStatus.Paused
      ) {
         return DownloadActionResponse.withExpectedStatus(record.toItem(), DownloadStatus.Canceled)
      }

      // Cancel the WorkManager work if running.
      workManager.cancelUniqueWork(workName(path))

      // Clean up temp file.
      val tempFile = File("${path}${DownloadWorker.DOWNLOAD_SUFFIX}")
      if (tempFile.exists()) tempFile.delete()

      // Remove from store and emit change.
      val canceled = record.withStatus(DownloadStatus.Canceled)
      store.remove(record)

      return DownloadActionResponse.new(emitChanged(canceled))
   }

   /**
    * Emits a download change event, derived from the persisted record.
    * Called by DownloadWorker to report progress and completion.
    *
    * Returns the emitted payload for reuse in a [DownloadActionResponse].
    */
   internal fun emitChanged(record: DownloadRecord): DownloadItem {
      val item = record.toItem()
      _changed.tryEmit(item)
      return item
   }

   /**
    * Enqueues a WorkManager work request for the download.
    * Uses unique work keyed by path to prevent duplicate workers.
    */
   private fun enqueueDownload(record: DownloadRecord) {
      val constraints = Constraints.Builder()
         .setRequiredNetworkType(NetworkType.CONNECTED)
         .build()

      val workRequest = OneTimeWorkRequestBuilder<DownloadWorker>()
         .setConstraints(constraints)
         .setInputData(
            workDataOf(
               DownloadWorker.KEY_URL to record.url,
               DownloadWorker.KEY_PATH to record.path,
            )
         )
         .addTag(WORK_TAG)
         .build()

      workManager.enqueueUniqueWork(
         workName(record.path),
         ExistingWorkPolicy.REPLACE,
         workRequest,
      )
   }

   private fun workName(path: String): String = "$WORK_TAG:$path"

   companion object {
      private const val TAG = "DownloadManager"
      private const val WORK_TAG = "download_manager"

      @Volatile
      private var instance: DownloadManager? = null

      /**
       * Returns the singleton DownloadManager instance.
       * Must be called with an application context.
       */
      fun getInstance(context: Context): DownloadManager {
         return instance ?: synchronized(this) {
            instance ?: DownloadManager(context.applicationContext).also {
               instance = it
            }
         }
      }
   }
}
