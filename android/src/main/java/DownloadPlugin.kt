package com.plugin.download

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke

@InvokeArg
class CreateArgs {
  var key: String = ""
  var url: String = ""
  var path: String = ""
}

@InvokeArg
class KeyArgs {
  var key: String = ""
}

@TauriPlugin
class DownloadPlugin(private val activity: Activity): Plugin(activity) {
    @Command
    fun create(invoke: Invoke) {
        val args = invoke.parseArgs(CreateArgs::class.java)
    }

    @Command
    fun get(invoke: Invoke) {
        val args = invoke.parseArgs(KeyArgs::class.java)
    }

    @Command
    fun list(invoke: Invoke) {
        val args = invoke.parseArgs(KeyArgs::class.java)
    }

    @Command
    fun start(invoke: Invoke) {
        val args = invoke.parseArgs(KeyArgs::class.java)
    }

    @Command
    fun cancel(invoke: Invoke) {
        val args = invoke.parseArgs(KeyArgs::class.java)
    }

    @Command
    fun pause(invoke: Invoke) {
        val args = invoke.parseArgs(KeyArgs::class.java)
    }

    @Command
    fun resume(invoke: Invoke) {
        val args = invoke.parseArgs(KeyArgs::class.java)
    }
}
