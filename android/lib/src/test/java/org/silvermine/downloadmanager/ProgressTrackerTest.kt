package org.silvermine.downloadmanager

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ProgressTrackerTest {

   // -- Known size --

   @Test
   fun `emits on integer percent change`() {
      val tracker = ProgressTracker(0L, 1000L)

      // 0.9% — still rounds to percent 0, which is what was last emitted.
      tracker.advance(9L)
      assertFalse(tracker.shouldEmit())

      // 1.0% — crosses into percent 1.
      tracker.advance(1L)
      assertTrue(tracker.shouldEmit())
   }

   @Test
   fun `markEmitted advances the baseline`() {
      val tracker = ProgressTracker(0L, 1000L)

      tracker.advance(10L)
      assertTrue(tracker.shouldEmit())
      tracker.markEmitted()

      // Same percent as the last emission, so nothing new to report.
      assertFalse(tracker.shouldEmit())

      tracker.advance(10L)
      assertTrue(tracker.shouldEmit())
   }

   @Test
   fun `always emits at one hundred percent`() {
      val tracker = ProgressTracker(0L, 1000L)

      tracker.advance(1000L)
      assertTrue(tracker.shouldEmit())
      tracker.markEmitted()

      // Still emits past 100% rather than going quiet.
      assertTrue(tracker.shouldEmit())
   }

   @Test
   fun `resuming treats initial bytes as already emitted`() {
      // A resumed download must not re-emit the count it started from.
      val tracker = ProgressTracker(500L, 1000L)
      assertFalse(tracker.shouldEmit())

      tracker.advance(9L)
      assertFalse(tracker.shouldEmit())

      tracker.advance(1L)
      assertTrue(tracker.shouldEmit())
   }

   @Test
   fun `isComplete with known size`() {
      val tracker = ProgressTracker(0L, 1000L)
      assertFalse(tracker.isComplete())

      tracker.advance(999L)
      assertFalse(tracker.isComplete())

      tracker.advance(1L)
      assertTrue(tracker.isComplete())
   }

   @Test
   fun `zero total falls back to the byte threshold`() {
      // A zero content length must not divide by zero; treat it as unknown.
      val tracker = ProgressTracker(0L, 0L)
      assertFalse(tracker.shouldEmit())

      tracker.advance(ProgressTracker.BYTES_THRESHOLD)
      assertTrue(tracker.shouldEmit())
   }

   // -- Unknown size --

   @Test
   fun `emits every threshold when size is unknown`() {
      val tracker = ProgressTracker(0L, null)

      tracker.advance(ProgressTracker.BYTES_THRESHOLD - 1L)
      assertFalse(tracker.shouldEmit())

      tracker.advance(1L)
      assertTrue(tracker.shouldEmit())
      tracker.markEmitted()

      assertFalse(tracker.shouldEmit())

      tracker.advance(ProgressTracker.BYTES_THRESHOLD)
      assertTrue(tracker.shouldEmit())
   }

   @Test
   fun `is never complete when size is unknown`() {
      // Unknown-size completion is signalled by the stream ending.
      val tracker = ProgressTracker(0L, null)
      tracker.advance(10L * ProgressTracker.BYTES_THRESHOLD)

      assertFalse(tracker.isComplete())
   }
}
