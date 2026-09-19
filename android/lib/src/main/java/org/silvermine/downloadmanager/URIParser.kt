package org.silvermine.downloadmanager

import java.net.URI

/**
 * Parses and validates a download path string.
 * Checks that the path is not empty, is an absolute path and contains a filename.
 * Returns it unchanged, as it is the download's identity.
 */
fun parsePath(pathString: String): String {
   if (pathString.isEmpty()) {
      throw IllegalArgumentException("Path cannot be empty")
   }

   if (!pathString.startsWith("/")) {
      throw IllegalArgumentException("Path must be absolute")
   }

   if (fileName(pathString) == null) {
      throw IllegalArgumentException("Path must have a filename")
   }

   return pathString
}

/**
 * The path's last component, or `null` for "/" or a trailing "..", as Rust's
 * `Path::file_name()` does. A string check: nothing is resolved.
 */
private fun fileName(path: String): String? {
   val components = path.split("/").filter { it.isNotEmpty() && it != "." }
   val last = components.lastOrNull() ?: return null

   return if (last == "..") null else last
}

/**
 * Parses and validates a download URL string.
 * Checks that the URL is valid, has a valid scheme (http or https) and has a valid host.
 */
fun parseURI(urlString: String): String {
   val uri = try {
      URI(urlString)
   } catch (e: Exception) {
      throw IllegalArgumentException("Invalid URL: $urlString")
   }

   val scheme = uri.scheme?.lowercase()
   if (scheme != "http" && scheme != "https") {
      throw IllegalArgumentException("Invalid URL scheme '${scheme ?: "none"}': must be http or https")
   }

   val host = uri.host
   if (host.isNullOrEmpty()) {
      throw IllegalArgumentException("URL must have a host")
   }

   if (uri.userInfo != null) {
      throw IllegalArgumentException("URL must not contain credentials")
   }

   return urlString
}
