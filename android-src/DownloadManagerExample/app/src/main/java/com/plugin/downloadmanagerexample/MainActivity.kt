   package com.plugin.downloadmanagerexample

   import android.content.Context
   import android.os.Bundle
   import android.os.Environment
   import androidx.activity.ComponentActivity
   import androidx.activity.compose.setContent
   import androidx.activity.enableEdgeToEdge
   import androidx.compose.foundation.layout.Arrangement
   import androidx.compose.foundation.layout.Column
   import androidx.compose.foundation.layout.Row
   import androidx.compose.foundation.layout.Spacer
   import androidx.compose.foundation.layout.fillMaxSize
   import androidx.compose.foundation.layout.fillMaxWidth
   import androidx.compose.foundation.layout.height
   import androidx.compose.foundation.layout.padding
   import androidx.compose.material3.Button
   import androidx.compose.material3.Card
   import androidx.compose.material3.OutlinedTextField
   import androidx.compose.material3.Scaffold
   import androidx.compose.material3.Text
   import androidx.compose.runtime.Composable
   import androidx.compose.runtime.collectAsState
   import androidx.compose.runtime.getValue
   import androidx.compose.runtime.mutableStateOf
   import androidx.compose.runtime.remember
   import androidx.compose.runtime.setValue
   import androidx.compose.ui.Modifier
   import androidx.compose.ui.unit.dp
   import androidx.lifecycle.ViewModel
   import androidx.lifecycle.viewModelScope
   import com.plugin.downloadmanagerexample.ui.theme.DownloadManagerExampleTheme
   import kotlinx.coroutines.flow.StateFlow
   import kotlinx.coroutines.launch
   import java.io.File

   class MainActivity : ComponentActivity() {
       override fun onCreate(savedInstanceState: Bundle?) {
           super.onCreate(savedInstanceState)

          val viewModel: DownloadViewModel = DownloadViewModel(DownloadManager(this))
           enableEdgeToEdge()
           setContent {
               DownloadManagerExampleTheme {
                   Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                      DownloadsView(
                         viewModel = viewModel,
                         modifier = Modifier.padding(innerPadding)
                      )
                   }
               }
           }
       }
   }

   @Composable
   fun DownloadsView(viewModel: DownloadViewModel, modifier: Modifier = Modifier) {
      val downloads by viewModel.downloads.collectAsState()
      var downloadUrl by remember { mutableStateOf("") }

      Column(modifier = modifier.padding(16.dp)) {

         // Input and Create Button
         OutlinedTextField(
            value = downloadUrl,
            onValueChange = { downloadUrl = it },
            label = { Text("Download URL") },
            modifier = Modifier.fillMaxWidth()
         )

         Spacer(modifier = Modifier.height(8.dp))

         Button(
            onClick = {
               if (downloadUrl.isNotBlank()) {
                  viewModel.createDownload(downloadUrl.trim())
                  downloadUrl = ""
               }
            },
            modifier = Modifier.fillMaxWidth()
         ) {
            Text("Create Download")
         }

         Spacer(modifier = Modifier.height(16.dp))

         // List of downloads
         downloads.forEach { item ->
            DownloadItemRow(item, viewModel)
            Spacer(modifier = Modifier.height(8.dp))
         }
      }
   }

   @Composable
   fun DownloadItemRow(item: DownloadItem, viewModel: DownloadViewModel) {
      Card(modifier = Modifier.fillMaxWidth()) {
         Column(modifier = Modifier.padding(16.dp)) {
            Text(text = "Key: ${item.key}")
            Text(text = "Progress: ${item.progress}%")
            Text(text = "State: ${item.state}")

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
               when (item.state) {
                  DownloadState.CREATED -> {
                     Button(onClick = { viewModel.start(item.key) }) { Text("Start") }
                     Button(onClick = { viewModel.cancel(item.key) }) { Text("Cancel") }
                  }
                  DownloadState.IN_PROGRESS -> {
                     Button(onClick = { viewModel.pause(item.key) }) { Text("Pause") }
                     Button(onClick = { viewModel.cancel(item.key) }) { Text("Cancel") }
                  }
                  DownloadState.PAUSED -> {
                     Button(onClick = { viewModel.resume(item.key) }) { Text("Resume") }
                     Button(onClick = { viewModel.cancel(item.key) }) { Text("Cancel") }
                  }
                  else -> {
                     // COMPLETED or CANCELLED – no actions
                  }
               }
            }
         }
      }
   }

   class DownloadViewModel(private val manager: DownloadManager) : ViewModel() {
      val downloads: StateFlow<List<DownloadItem>> = manager.downloads

      fun createDownload(url: String) = viewModelScope.launch {
         val key = getFileNameFromUrl(url)
         val path = ""
         manager.create(DownloadItem(key, url, path))
      }

      fun start(key: String) = viewModelScope.launch {
         manager.start(key)
      }

      fun pause(key: String) = viewModelScope.launch {
         manager.pause(key)
      }

      fun resume(key: String) = viewModelScope.launch {
         manager.resume(key)
      }

      fun cancel(key: String) = viewModelScope.launch {
         manager.cancel(key)
      }

      fun getFileNameFromUrl(url: String): String {
         return try {
            val uri = java.net.URI(url)
            val path = uri.path
            path.substringAfterLast('/').ifEmpty { "downloaded_file" }
         } catch (e: Exception) {
            "downloaded_file"
         }
      }
   }

