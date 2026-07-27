package org.silvermine.downloadmanager

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.ServiceInfo
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.work.CoroutineWorker
import androidx.work.ForegroundInfo
import androidx.work.WorkerParameters
import kotlinx.coroutines.delay
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.InterruptedIOException
import java.net.UnknownHostException
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLException

/**
 * WorkManager CoroutineWorker that performs the actual HTTP download.
 *
 * Mirrors the Rust downloader.rs pattern:
 * - Supports resume via Range headers
 * - Writes to a temp file (.download suffix), renames on completion
 * - Throttles progress updates via [ProgressTracker]
 * - Checks store status each progress tick to detect pause/cancel
 * - Runs as a foreground service with a notification
 */
internal class DownloadWorker(
   context: Context,
   params: WorkerParameters,
) : CoroutineWorker(context, params) {

   override suspend fun doWork(): Result {
      val url = inputData.getString(KEY_URL) ?: return Result.failure()
      val path = inputData.getString(KEY_PATH) ?: return Result.failure()

      val manager = DownloadManager.getInstance(applicationContext)
      val store = manager.store
      val tempFile = File("$path$DOWNLOAD_SUFFIX")

      try {
         setForeground(createForegroundInfo(path))
      } catch (e: Exception) {
         Log.w(TAG, "Failed to set foreground info: ${e.message}")
      }

      // Byte counts outlive the response scope so the completion block below can
      // report what the progress loop tracked.
      var finalReceivedBytes = 0L
      var finalTotalBytes: Long? = null

      try {
         // Check the size of the already downloaded part, if any.
         var downloadedSize = if (tempFile.exists()) tempFile.length() else 0L

         // Build request with Range header for resuming.
         val requestBuilder = Request.Builder().url(url)
         if (downloadedSize > 0) {
            requestBuilder.header("Range", "bytes=$downloadedSize-")
         }

         val response = executeWithRetry(requestBuilder.build())

         response.use {
            // If we requested a Range but the server doesn't support partial downloads,
            // fall back to restarting from zero rather than failing.
            if (downloadedSize > 0 && response.code != 206) {
               if (response.isSuccessful) {
                  Log.w(TAG, "Server does not support Range; restarting download from zero")
                  if (tempFile.exists()) tempFile.delete()
                  downloadedSize = 0L
               } else {
                  return handleError(manager, store, path, tempFile, "HTTP ${response.code}: ${response.message}")
               }
            }

            if (!response.isSuccessful && response.code != 206) {
               return handleError(manager, store, path, tempFile, "HTTP ${response.code}: ${response.message}")
            }

            val body = response.body
               ?: return handleError(manager, store, path, tempFile, "Empty response body")

            // Get the total size of the file from headers (if available).
            // OkHttp reports -1 when unknown, which stays null rather than
            // collapsing to a bogus zero total.
            val contentLength = body.contentLength()
            val totalSize = if (contentLength > 0) contentLength + downloadedSize else null

            // Ensure the output folder exists.
            tempFile.parentFile?.let { parent ->
               if (!parent.exists()) parent.mkdirs()
            }

            // Open the temp file in append mode (or truncate if restarting from zero).
            val append = downloadedSize > 0
            val progress = ProgressTracker(downloadedSize, totalSize)

            // Update status to in-progress, persisting a known total. The total is
            // written only when newly known: it must survive an abrupt exit so
            // reconcileStoreOnInit() can pair it with the recovered temp-file
            // length, but a chunked resume reports none and must not erase it.
            //
            // The status is written unconditionally because WorkManager reruns this
            // worker after process death, by which point reconcileStoreOnInit() has
            // already moved the record off InProgress. Synchronized on manager so the
            // write cannot interleave with pause/cancel; a pause that still lands
            // first is undone by the isStopped check in the read loop.
            synchronized(manager) {
               store.findByPath(path)?.let { record ->
                  val updated = if (totalSize != null && record.totalBytes != totalSize) {
                     record.withBytes(downloadedSize, totalSize)
                  } else {
                     record
                  }

                  store.update(updated.withStatus(DownloadStatus.InProgress))
               }
            }

            FileOutputStream(tempFile, append).use { output ->
               val buffer = ByteArray(BUFFER_SIZE)
               val source = body.byteStream()

               while (true) {
                  // Check if the worker has been stopped (canceled externally).
                  if (isStopped) {
                     source.close()
                     revertToPaused(manager, store, path)
                     dismissNotification()
                     return Result.success()
                  }

                  val bytesRead = source.read(buffer)
                  if (bytesRead == -1) break

                  output.write(buffer, 0, bytesRead)
                  progress.advance(bytesRead.toLong())

                  if (!progress.shouldEmit()) continue

                  progress.markEmitted()
                  val currentRecord = store.findByPath(path) ?: break

                  when (currentRecord.status) {
                     DownloadStatus.InProgress -> {
                        if (!progress.isComplete()) {
                           // Download is not yet complete.
                           // Update record in store and emit change event.
                           val updated = currentRecord
                              .withBytes(progress.receivedBytes, totalSize)
                              .withStatus(DownloadStatus.InProgress)
                           store.update(updated, persist = false)
                           val item = manager.emitChanged(updated)
                           updateNotificationProgress(path, item.progress.toInt(), indeterminate = totalSize == null)
                        }
                        // Completion is handled after the loop exits naturally.
                     }
                     DownloadStatus.Paused -> {
                        // Download was paused — stop reading and exit gracefully.
                        source.close()
                        dismissNotification()
                        return Result.success()
                     }
                     else -> {
                        // Download item was removed or in unexpected state.
                        source.close()
                        dismissNotification()
                        return Result.success()
                     }
                  }
               }
            }

            finalReceivedBytes = progress.receivedBytes
            finalTotalBytes = totalSize
         }

         // Download completed — rename temp file to final path and update store.
         // Synchronized on manager to prevent interleaving with cancel/pause,
         // mirroring the iOS actor serialization pattern.
         var renameFailed = false
         synchronized(manager) {
            val currentRecord = store.findByPath(path)
            if (currentRecord != null && currentRecord.status == DownloadStatus.InProgress) {
               val finalFile = File(path)
               finalFile.parentFile?.let { parent ->
                  if (!parent.exists()) parent.mkdirs()
               }

               // Remove existing file (if found) and move downloaded file to destination.
               if (finalFile.exists()) finalFile.delete()
               if (!tempFile.renameTo(finalFile)) {
                  renameFailed = true
               } else {
                  val completed = currentRecord
                     .withBytes(finalReceivedBytes, finalTotalBytes)
                     .withStatus(DownloadStatus.Completed)
                  store.remove(currentRecord)
                  manager.emitChanged(completed)
               }
            } else {
               // Download item was removed from store during download — clean up orphaned temp file.
               Log.w(TAG, "Download item not found or not in expected state after download completed for $path")
               if (tempFile.exists()) tempFile.delete()
            }
         }

         // Error handling is deferred outside the synchronized block to avoid
         // reentrant lock acquisition (handleError also synchronizes on manager).
         if (renameFailed) {
            return handleError(manager, store, path, tempFile, "Failed to move download to $path")
         }

         dismissNotification()
         return Result.success()
      } catch (e: Exception) {
         // Transient failures (network drops mid-download) preserve the temp file
         // and transition to Paused so the download can be resumed later. This
         // mirrors iOS behavior where URLSession saves resume data for transient
         // errors. Permanent failures (DNS, TLS) delete the temp file and cancel.
         val isTransientFailure = e is IOException && isTransient(e)
         return if (isTransientFailure) {
            handleTransientError(manager, store, path, e.message ?: "Unknown error")
         } else {
            handleError(manager, store, path, tempFile, e.message ?: "Unknown error")
         }
      }
   }

   /**
    * Reverts a record still marked InProgress when the worker stops early.
    *
    * A pause cancels the WorkManager work and sets the record to Paused, but the
    * worker may already have written InProgress back before observing isStopped.
    * Without this the record would stay InProgress with no worker behind it until
    * the next reconcileStoreOnInit(). The temp file is left in place so the
    * download can resume.
    */
   private fun revertToPaused(manager: DownloadManager, store: DownloadStore, path: String) {
      // Synchronized on manager to prevent interleaving with cancel/pause.
      synchronized(manager) {
         val record = store.findByPath(path)
         if (record == null || record.status != DownloadStatus.InProgress) {
            return
         }

         // Recover the byte count from the temp file: only bytes flushed to
         // disk are resumable, whatever the last progress tick reported.
         val tempFile = File("$path$DOWNLOAD_SUFFIX")
         val receivedBytes = if (tempFile.exists()) tempFile.length() else 0L
         val paused = record.withBytes(receivedBytes, record.totalBytes).withStatus(DownloadStatus.Paused)

         store.update(paused)
         manager.emitChanged(paused)
      }
   }

   /**
    * Handles permanent failures (HTTP errors, rename failures, DNS/TLS errors).
    * Deletes the temp file, cancels the download, and removes it from the store.
    */
   private fun handleError(manager: DownloadManager, store: DownloadStore, path: String, tempFile: File, message: String): Result {
      Log.e(TAG, "Download failed (permanent) for $path: $message")

      // Synchronized on manager to prevent interleaving with cancel/pause.
      synchronized(manager) {
         if (tempFile.exists()) tempFile.delete()
         store.findByPath(path)?.let { record ->
            val canceled = record.withStatus(DownloadStatus.Canceled)
            store.remove(record)
            manager.emitChanged(canceled)
         }
      }

      dismissNotification()
      return Result.failure()
   }

   /**
    * Handles transient failures (network drops, timeouts that exhausted retries).
    * Preserves the temp file and transitions to Paused so the download can be
    * resumed later via Range headers. Mirrors iOS behavior where URLSession saves
    * resume data for transient errors.
    */
   private fun handleTransientError(manager: DownloadManager, store: DownloadStore, path: String, message: String): Result {
      Log.w(TAG, "Download failed (transient) for $path: $message")

      // Synchronized on manager to prevent interleaving with cancel/pause.
      synchronized(manager) {
         store.findByPath(path)?.let { record ->
            // Recover the byte count from the temp file: only bytes flushed to
            // disk are resumable, whatever the last progress tick reported.
            val tempFile = File("$path$DOWNLOAD_SUFFIX")
            val receivedBytes = if (tempFile.exists()) tempFile.length() else 0L
            val paused = record.withBytes(receivedBytes, record.totalBytes).withStatus(DownloadStatus.Paused)
            store.update(paused)
            manager.emitChanged(paused)
         }
      }

      dismissNotification()
      return Result.failure()
   }

   private fun notificationID(): Int = id.hashCode()

   private fun ensureNotificationChannel() {
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
         val notificationManager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
         val channel = NotificationChannel(
            NOTIFICATION_CHANNEL_ID,
            "Downloads",
            NotificationManager.IMPORTANCE_LOW,
         )
         notificationManager.createNotificationChannel(channel)
      }
   }

   private fun buildNotification(filename: String, progress: Int, indeterminate: Boolean): android.app.Notification {
      return NotificationCompat.Builder(applicationContext, NOTIFICATION_CHANNEL_ID)
         .setContentTitle("Downloading")
         .setContentText(filename)
         .setSmallIcon(android.R.drawable.stat_sys_download)
         .setOngoing(true)
         .setProgress(100, progress, indeterminate)
         .build()
   }

   private fun createForegroundInfo(path: String): ForegroundInfo {
      ensureNotificationChannel()
      val notification = buildNotification(File(path).name, 0, indeterminate = true)
      return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
         ForegroundInfo(notificationID(), notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
      } else {
         ForegroundInfo(notificationID(), notification)
      }
   }

   private fun updateNotificationProgress(path: String, progress: Int, indeterminate: Boolean) {
      val notification = buildNotification(File(path).name, progress, indeterminate)
      val notificationManager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
      notificationManager.notify(notificationID(), notification)
   }

   private fun dismissNotification() {
      val notificationManager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
      notificationManager.cancel(notificationID())
   }

   /**
    * Executes an OkHttp request with retries and exponential backoff.
    * Mirrors the Rust reqwest-retry middleware (3 retries, exponential backoff).
    * Only retries on transient errors; permanent failures (DNS, TLS) fail immediately.
    * Uses coroutine delay() instead of Thread.sleep() to avoid blocking the dispatcher.
    */
   private suspend fun executeWithRetry(request: Request): Response {
      var lastException: IOException? = null

      for (attempt in 0..MAX_RETRIES) {
         if (attempt > 0) {
            // Exponential backoff: 1s, 2s, 4s
            delay(1000L * (1 shl (attempt - 1)))
         }

         try {
            val response = client.newCall(request).execute()
            if (response.code in 500..599 && attempt < MAX_RETRIES) {
               response.close()
               Log.w(TAG, "Retrying after HTTP ${response.code} (attempt ${attempt + 1}/$MAX_RETRIES)")
               continue
            }
            return response
         } catch (e: IOException) {
            if (!isTransient(e)) throw e
            lastException = e
            Log.w(TAG, "Retrying after ${e.message} (attempt ${attempt + 1}/$MAX_RETRIES)")
         }
      }

      throw lastException ?: IOException("Retry failed")
   }

   companion object {
      const val KEY_URL = "download_url"
      const val KEY_PATH = "download_path"

      internal const val TAG = "DownloadWorker"
      internal const val DOWNLOAD_SUFFIX = ".download"
      private const val BUFFER_SIZE = 64 * 1024
      private const val MAX_RETRIES = 3
      private const val NOTIFICATION_CHANNEL_ID = "download_manager_channel"

      private fun isTransient(e: IOException): Boolean = when (e) {
         is UnknownHostException -> false  // DNS resolution failed
         is SSLException -> false          // TLS/certificate errors
         is InterruptedIOException -> e.message?.contains("timeout", ignoreCase = true) == true
         else -> true                      // Connection reset, broken pipe, etc.
      }

      private val client = OkHttpClient.Builder()
         .connectTimeout(30, TimeUnit.SECONDS)
         .readTimeout(30, TimeUnit.SECONDS)
         .followRedirects(true)
         .followSslRedirects(false)
         .build()
   }
}
