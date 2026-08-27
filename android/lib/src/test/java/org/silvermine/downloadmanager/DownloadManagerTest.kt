package org.silvermine.downloadmanager

import androidx.work.NetworkType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DownloadManagerTest {

   private fun inProgressRecord(): DownloadRecord = DownloadRecord(
      url = "http://example.com/file.mp4",
      path = "/tmp/file.mp4",
      receivedBytes = 500L,
      totalBytes = 1000L,
      status = DownloadStatus.InProgress,
   )

   // -- Reconciliation --

   @Test
   fun `record with a temp file reverts to paused at the flushed length`() {
      val reverted = DownloadManager.revertInProgress(inProgressRecord(), tempFileLength = 480L)

      assertEquals(DownloadStatus.Paused, reverted?.status)
      // Only bytes flushed to disk are resumable, so the temp file's length wins
      // over the 500 the last progress tick reported.
      assertEquals(480L, reverted?.receivedBytes)
      assertEquals(1000L, reverted?.totalBytes)
   }

   @Test
   fun `record without a temp file reverts to idle at zero`() {
      val reverted = DownloadManager.revertInProgress(inProgressRecord(), tempFileLength = null)

      assertEquals(DownloadStatus.Idle, reverted?.status)
      // Nothing to resume from, so the download restarts from scratch.
      assertEquals(0L, reverted?.receivedBytes)
      // The total came from headers and is still true of the remote file.
      assertEquals(1000L, reverted?.totalBytes)
   }

   @Test
   fun `an empty temp file still reverts to paused`() {
      val reverted = DownloadManager.revertInProgress(inProgressRecord(), tempFileLength = 0L)

      assertEquals(DownloadStatus.Paused, reverted?.status)
      assertEquals(0L, reverted?.receivedBytes)
   }

   @Test
   fun `records that are not in progress are left alone`() {
      // Derived from the enum so a status added later is covered automatically.
      val statuses = DownloadStatus.entries.filter { it != DownloadStatus.InProgress }

      for (status in statuses) {
         val record = inProgressRecord().withStatus(status)

         assertNull("$status should not be reverted", DownloadManager.revertInProgress(record, 480L))
         assertNull("$status should not be reverted", DownloadManager.revertInProgress(record, null))
      }
   }

   // -- Network policy --

   // Building the Constraints outside enqueueDownload() shrinks the untestable surface
   // to one delegating call. That delegation stays uncovered — enqueueDownload() needs a
   // Context — so these pin the policy the constraint carries, not that the enqueue path
   // uses it.

   @Test
   fun `an unrestricted download only requires a connection`() {
      assertEquals(
         NetworkType.CONNECTED,
         DownloadManager.constraintsFor(CreateOptions(allowMetered = true)).requiredNetworkType,
      )
   }

   @Test
   fun `a restricted download requires an unmetered network`() {
      assertEquals(
         NetworkType.UNMETERED,
         DownloadManager.constraintsFor(CreateOptions(allowMetered = false)).requiredNetworkType,
      )
   }

   // -- Work request input data --

   @Test
   fun `a configured user agent is captured in the work request`() {
      val data = DownloadManager.inputDataFor(inProgressRecord(), userAgent = "my-app/1.0")

      assertEquals("my-app/1.0", data.getString(DownloadWorker.KEY_USER_AGENT))
      assertEquals("http://example.com/file.mp4", data.getString(DownloadWorker.KEY_URL))
      assertEquals("/tmp/file.mp4", data.getString(DownloadWorker.KEY_PATH))
   }

   @Test
   fun `no user agent stores a null value`() {
      // The key is present with a null value, not absent: measured on work-runtime
      // 2.9.1, `workDataOf(k to null)` gives size()==3 and containsKey()==true. Either
      // way `getString` returns null, which the worker treats as "send no User-Agent",
      // leaving OkHttp's default.
      val data = DownloadManager.inputDataFor(inProgressRecord(), userAgent = null)

      assertNull(data.getString(DownloadWorker.KEY_USER_AGENT))
   }
}
