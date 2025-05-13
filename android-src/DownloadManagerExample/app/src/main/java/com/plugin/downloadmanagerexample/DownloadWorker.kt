package com.plugin.downloadmanagerexample

import android.content.Context
import androidx.work.*
import kotlinx.coroutines.coroutineScope
import java.io.BufferedInputStream
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL

class DownloadWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
   override suspend fun doWork(): Result = coroutineScope {
      val key = inputData.getString("key") ?: return@coroutineScope Result.failure()
      val store = DownloadStore(applicationContext)
      val item = store.get(key) ?: return@coroutineScope Result.failure()

      try {
         item.state = DownloadState.IN_PROGRESS
         store.save(item)

         val file = File(item.path)
         val connection = URL(item.url).openConnection().apply {
            if (file.exists()) {
               val resumeFrom = file.length()
               setRequestProperty("Range", "bytes=$resumeFrom-")
               item.downloadedBytes = resumeFrom
            }
         }

         val responseCode = (connection as? HttpURLConnection)?.responseCode ?: 200
         if (responseCode != 200 && responseCode != 206) {
            return@coroutineScope Result.retry()
         }

         val totalSize = connection.contentLength + item.downloadedBytes
         val input = BufferedInputStream(connection.getInputStream())
         val output = FileOutputStream(file, true) // append = true

         val buffer = ByteArray(1024)
         var bytesRead: Int
         var downloaded = item.downloadedBytes

         while (input.read(buffer).also { bytesRead = it } != -1) {
            val latestItem = store.get(key) ?: break

            if (latestItem.state == DownloadState.PAUSED) {
               input.close()
               output.close()
               return@coroutineScope Result.retry()
            }

            if (latestItem.state == DownloadState.CANCELLED) {
               input.close()
               output.close()
               file.delete()
               return@coroutineScope Result.failure()
            }

            output.write(buffer, 0, bytesRead)
            downloaded += bytesRead
            item.downloadedBytes = downloaded
            item.progress = (downloaded * 100 / totalSize).toInt()
            store.save(item)
         }

         output.flush()
         output.close()
         input.close()

         item.state = DownloadState.COMPLETED
         item.progress = 100
         store.save(item)

         Result.success()
      } catch (e: Exception) {
         item.state = DownloadState.PAUSED
         store.save(item)
         Result.retry()
      }
   }

   companion object {
      fun createWorkRequest(key: String): OneTimeWorkRequest {
         return OneTimeWorkRequestBuilder<DownloadWorker>()
            .setInputData(workDataOf("key" to key))
            .addTag(key)
            .build()
      }
   }
}
