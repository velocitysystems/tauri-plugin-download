package org.silvermine.downloadmanager

import java.util.UUID

/**
 * Fences live commands against workers still unwinding asynchronous cancellation.
 * All access is under the manager monitor, together with store and file mutations.
 *
 * Startup registers the compatible unfinished request before exposing the manager.
 * Only that request, or a replacement registered by a live command, can own a path.
 * Finished and canceled paths need no retained entries.
 */
internal class DownloadWorkOwnership {
   private val requests = mutableMapOf<String, UUID>()

   /** Grants a replacement request exclusive ownership of [path]. */
   fun replace(path: String, request: UUID) {
      requests[path] = request
   }

   /** Prevents every existing request from modifying [path] after pause or cancel. */
   fun revoke(path: String) {
      requests.remove(path)
   }

   /** Retries keep their request ID; replacements and explicit stops invalidate it. */
   fun owns(path: String, request: UUID): Boolean =
      requests[path] == request
}
