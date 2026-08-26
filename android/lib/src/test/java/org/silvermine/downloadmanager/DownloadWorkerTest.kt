package org.silvermine.downloadmanager

import org.junit.Assert.assertFalse
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
}
