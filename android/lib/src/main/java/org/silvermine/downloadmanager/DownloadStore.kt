package org.silvermine.downloadmanager

import android.content.Context
import android.util.AtomicFile
import android.util.Log
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Thread-safe store for download records backed by an atomic JSON file.
 *
 * All public methods are synchronized to ensure consistency when accessed
 * from multiple threads (e.g. WorkManager workers and the main thread).
 * Mirrors the iOS DownloadStore actor pattern.
 */
internal class DownloadStore(context: Context) {
   private val file = AtomicFile(File(context.filesDir, STORE_FILENAME))
   private val downloads = mutableMapOf<String, DownloadRecord>()

   init {
      load()
   }

   @Synchronized
   fun list(): List<DownloadRecord> = downloads.values.toList()

   @Synchronized
   fun findByPath(path: String): DownloadRecord? = downloads[path]

   @Synchronized
   fun append(record: DownloadRecord) {
      downloads[record.path] = record
      save()
   }

   @Synchronized
   fun update(record: DownloadRecord, persist: Boolean = true) {
      if (downloads.containsKey(record.path)) {
         downloads[record.path] = record
      }
      if (persist) {
         save()
      }
   }

   /**
    * Applies several record updates with a single write.
    *
    * Reconciliation can revert many records at once; one write per record would
    * rewrite the whole file that many times.
    *
    * @param records The records to update. Unknown paths are ignored.
    */
   @Synchronized
   fun update(records: List<DownloadRecord>) {
      if (records.isEmpty()) {
         return
      }

      for (record in records) {
         if (downloads.containsKey(record.path)) {
            downloads[record.path] = record
         }
      }

      save()
   }

   @Synchronized
   fun remove(record: DownloadRecord) {
      downloads.remove(record.path)
      save()
   }

   private fun load() {
      try {
         val records = decodeRecords(String(file.readFully()))
         downloads.clear()
         for (record in records) {
            downloads[record.path] = record
         }
      } catch (e: Exception) {
         Log.e(TAG, "Failed to load download store: ${e.message}")
      }
   }

   private fun save() {
      val bytes = encodeRecords(downloads.values.toList()).toByteArray()
      val stream = file.startWrite()
      try {
         stream.write(bytes)
         file.finishWrite(stream)
      } catch (e: Exception) {
         file.failWrite(stream)
         Log.e(TAG, "Failed to save download store: ${e.message}")
      }
   }

   companion object {
      private const val TAG = "DownloadStore"
      private const val STORE_FILENAME = "downloads.json"

      private val json = Json { ignoreUnknownKeys = true }

      /**
       * Decodes persisted records.
       *
       * Note that one malformed element fails the whole array, discarding every
       * other download in the file. Making that per-record is tracked in #64.
       *
       * Extracted from [load] so it can be unit-tested, and deliberately free of
       * logging to keep it so: `android.util.Log` is a throwing stub off-device.
       *
       * @param text The persisted store's contents.
       * @return The decoded records.
       */
      internal fun decodeRecords(text: String): List<DownloadRecord> = json.decodeFromString(text)

      /**
       * Encodes records for persistence.
       *
       * @param records The records to encode.
       * @return The JSON text to persist.
       */
      internal fun encodeRecords(records: List<DownloadRecord>): String = json.encodeToString(records)
   }
}
