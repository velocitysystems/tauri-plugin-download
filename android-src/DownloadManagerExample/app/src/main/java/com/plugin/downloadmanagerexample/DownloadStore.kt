package com.plugin.downloadmanagerexample

import android.content.Context
import android.content.SharedPreferences
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import androidx.core.content.edit

class DownloadStore(context: Context) {
   private val prefs: SharedPreferences = context.getSharedPreferences("downloads", Context.MODE_PRIVATE)
   private val gson = Gson()
   private val mutex = Mutex()

   suspend fun save(downloadItem: DownloadItem) = mutex.withLock {
      val all = getAll().toMutableMap()
      all[downloadItem.key] = downloadItem
      val json = gson.toJson(all)
      prefs.edit { putString("items", json) }
   }

   suspend fun get(key: String): DownloadItem? = mutex.withLock {
      getAll()[key]
   }

   suspend fun getAll(): Map<String, DownloadItem> = mutex.withLock {
      prefs.getString("items", null)?.let {
         val type = object : TypeToken<Map<String, DownloadItem>>() {}.type
         gson.fromJson(it, type)
      } ?: emptyMap()
   }
}
