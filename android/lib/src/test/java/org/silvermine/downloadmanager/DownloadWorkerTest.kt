package org.silvermine.downloadmanager

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DownloadWorkerTest {

   // These pin the predicate, not the branches it drives: a CoroutineWorker cannot be
   // built without WorkManager's test artifact, so neither path in handleTransientError
   // is covered here. WorkManager counts the runs before the current one, so a
   // download's first run sees 0.

   @Test
   fun `a download has attempts left up to the cap`() {
      for (runAttemptCount in 0..4) {
         assertFalse(
            "run reporting $runAttemptCount should still have attempts",
            DownloadWorker.isOutOfAttempts(runAttemptCount),
         )
      }
   }

   @Test
   fun `a download is out of attempts once five are spent`() {
      // Above the cap is reachable, not merely defensive: a constraint interruption
      // increments the count without ever consulting the cap.
      assertTrue(DownloadWorker.isOutOfAttempts(5))
      assertTrue(DownloadWorker.isOutOfAttempts(6))
   }

   // -- Request construction --

   @Test
   fun `a configured user agent is set on the request`() {
      val request = DownloadWorker.requestFor("https://example.com/f.bin", "my-app/1.0", 0L)

      assertEquals("my-app/1.0", request.header("User-Agent"))
   }

   @Test
   fun `no user agent leaves the header unset`() {
      // Absent rather than empty: OkHttp then sends its own default.
      val request = DownloadWorker.requestFor("https://example.com/f.bin", null, 0L)

      assertNull(request.header("User-Agent"))
   }

   @Test
   fun `a fresh download sends no range header`() {
      // The common path, and the boundary of the resume condition: without this,
      // widening `downloadedSize > 0` to `>= 0` changes no test outcome.
      val request = DownloadWorker.requestFor("https://example.com/f.bin", "my-app/1.0", 0L)

      assertNull(request.header("Range"))
   }

   @Test
   fun `the user agent and range headers coexist on a resume`() {
      // Mirrors the Rust test_user_agent_and_range_header_are_both_sent_on_resume:
      // neither header may displace the other.
      val request = DownloadWorker.requestFor("https://example.com/f.bin", "my-app/1.0", 4L)

      assertEquals("my-app/1.0", request.header("User-Agent"))
      assertEquals("bytes=4-", request.header("Range"))
   }
}
