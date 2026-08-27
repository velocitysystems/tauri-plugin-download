package org.silvermine.plugin.download

import android.app.Activity
import android.util.Log
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.silvermine.downloadmanager.CreateOptions
import org.silvermine.downloadmanager.DownloadManager
import org.silvermine.downloadmanager.parsePath
import org.silvermine.downloadmanager.parseURI
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.json.JSONArray
import java.io.File

@InvokeArg
class PathArgs {
   var path: String? = null
}

/**
 * Bridge-local mirror of [CreateOptions], kept separate so Tauri's `@InvokeArg`
 * reflection and kotlinx.serialization never share a type.
 *
 * Nullable so an absent value is rejected rather than defaulted. The Rust bridge
 * resolves the API default before invoking, so the policy always arrives stated.
 */
@InvokeArg
class CreateOptionsArgs {
   var allowMetered: Boolean? = null
}

/**
 * Settings pushed by the Rust plugin at startup.
 *
 * Named for the payload rather than the setting so a second builder option is a new
 * field here, not a second command.
 */
@InvokeArg
class ConfigArgs {
   var userAgent: String? = null
}

@InvokeArg
class CreateArgs {
   var path: String? = null
   var url: String? = null
   var options: CreateOptionsArgs? = null
}

@TauriPlugin
class DownloadPlugin(activity: Activity) : Plugin(activity) {
   private val json = Json { encodeDefaults = true }
   private val downloadManager by lazy { DownloadManager.getInstance(activity.applicationContext) }
   private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

   override fun load(webView: WebView) {
      scope.launch {
         downloadManager.changed.collect { item ->
            try {
               trigger("changed", JSObject(json.encodeToString(item)))
               Log.d(
                  TAG,
                  "[${File(item.path).name}] ${item.status} - ${"%.0f".format(item.progress)}%" +
                     " (${item.receivedBytes}/${item.totalBytes ?: "unknown"} bytes)",
               )
            } catch (e: Exception) {
               Log.e(TAG, "Failed to emit changed event: ${e.message}")
            }
         }
      }
   }

   override fun onDestroy() {
      super.onDestroy()
      scope.cancel()
   }

   /**
    * Applies the settings from the Rust plugin's builder.
    *
    * Invoked by the Rust plugin during setup rather than from the webview: Tauri
    * fills the config it hands to [load] from `tauri.conf.json`, so a value set on
    * the Rust builder can only arrive as a command.
    */
   @Command
   fun configure(invoke: Invoke) {
      val args = invoke.parseArgs(ConfigArgs::class.java)
      val userAgent = args.userAgent
         ?: return invoke.reject("Missing required argument: userAgent")

      downloadManager.userAgent = userAgent
      invoke.resolve()
   }

   @Command
   fun list(invoke: Invoke) {
      scope.launch {
         try {
            val items = withContext(Dispatchers.IO) { downloadManager.list() }
            val result = JSObject().apply {
               put("value", JSONArray(json.encodeToString(items)))
            }
            invoke.resolve(result)
         } catch (e: Exception) {
            invoke.reject(e.message)
         }
      }
   }

   @Command
   fun get(invoke: Invoke) {
      val args = invoke.parseArgs(PathArgs::class.java)
      val path = try {
         parsePath(args.path ?: throw IllegalArgumentException("Missing required argument: path"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      scope.launch {
         try {
            val response = withContext(Dispatchers.IO) { downloadManager.get(path) }
            invoke.resolve(JSObject(json.encodeToString(response)))
         } catch (e: Exception) {
            invoke.reject(e.message)
         }
      }
   }

   @Command
   fun create(invoke: Invoke) {
      val args = invoke.parseArgs(CreateArgs::class.java)
      val path = try {
         parsePath(args.path ?: throw IllegalArgumentException("Missing required argument: path"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      val url = try {
         parseURI(args.url ?: throw IllegalArgumentException("Missing required argument: url"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      val options = try {
         CreateOptions(
            allowMetered = args.options?.allowMetered
               ?: throw IllegalArgumentException("Missing required argument: options.allowMetered"),
         )
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }

      scope.launch {
         val response = withContext(Dispatchers.IO) { downloadManager.create(path, url, options) }
         invoke.resolve(JSObject(json.encodeToString(response)))
      }
   }

   @Command
   fun start(invoke: Invoke) {
      val args = invoke.parseArgs(PathArgs::class.java)
      val path = try {
         parsePath(args.path ?: throw IllegalArgumentException("Missing required argument: path"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      scope.launch {
         try {
            val response = withContext(Dispatchers.IO) { downloadManager.start(path) }
            invoke.resolve(JSObject(json.encodeToString(response)))
         } catch (e: Exception) {
            invoke.reject(e.message)
         }
      }
   }

   @Command
   fun cancel(invoke: Invoke) {
      val args = invoke.parseArgs(PathArgs::class.java)
      val path = try {
         parsePath(args.path ?: throw IllegalArgumentException("Missing required argument: path"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      scope.launch {
         try {
            val response = withContext(Dispatchers.IO) { downloadManager.cancel(path) }
            invoke.resolve(JSObject(json.encodeToString(response)))
         } catch (e: Exception) {
            invoke.reject(e.message)
         }
      }
   }

   @Command
   fun pause(invoke: Invoke) {
      val args = invoke.parseArgs(PathArgs::class.java)
      val path = try {
         parsePath(args.path ?: throw IllegalArgumentException("Missing required argument: path"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      scope.launch {
         try {
            val response = withContext(Dispatchers.IO) { downloadManager.pause(path) }
            invoke.resolve(JSObject(json.encodeToString(response)))
         } catch (e: Exception) {
            invoke.reject(e.message)
         }
      }
   }

   @Command
   fun resume(invoke: Invoke) {
      val args = invoke.parseArgs(PathArgs::class.java)
      val path = try {
         parsePath(args.path ?: throw IllegalArgumentException("Missing required argument: path"))
      } catch (e: Exception) {
         return invoke.reject(e.message)
      }
      scope.launch {
         try {
            val response = withContext(Dispatchers.IO) { downloadManager.resume(path) }
            invoke.resolve(JSObject(json.encodeToString(response)))
         } catch (e: Exception) {
            invoke.reject(e.message)
         }
      }
   }

   companion object {
      private const val TAG = "DownloadPlugin"
   }
}
