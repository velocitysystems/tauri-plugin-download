import XCTest
@testable import DownloadManagerKit

final class ProgressTrackerTests: XCTestCase {

   // MARK: - Known size

   func testEmitsOnIntegerPercentChange() {
      // 0.9% — still rounds to percent 0, which is what was last emitted.
      XCTAssertFalse(ProgressTracker(lastEmittedBytes: 0, receivedBytes: 9, totalBytes: 1000).shouldEmit)

      // 1.0% — crosses into percent 1.
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 0, receivedBytes: 10, totalBytes: 1000).shouldEmit)
   }

   func testDoesNotEmitWithinTheSamePercent() {
      // Baseline at 50%: anything still rounding to 50 is nothing new to report.
      XCTAssertFalse(ProgressTracker(lastEmittedBytes: 500, receivedBytes: 505, totalBytes: 1000).shouldEmit)
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 500, receivedBytes: 510, totalBytes: 1000).shouldEmit)
   }

   func testAlwaysEmitsAtOneHundredPercent() {
      // Emits at 100%, and past it if the body overruns the advertised length.
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 990, receivedBytes: 1000, totalBytes: 1000).shouldEmit)
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 1000, receivedBytes: 1000, totalBytes: 1000).shouldEmit)
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 1000, receivedBytes: 1500, totalBytes: 1000).shouldEmit)
   }

   func testResumingTreatsInitialBytesAsAlreadyEmitted() {
      // A resumed download must not re-emit the count it started from.
      XCTAssertFalse(ProgressTracker(lastEmittedBytes: 500, receivedBytes: 500, totalBytes: 1000).shouldEmit)
   }

   func testIsCompleteWithKnownSize() {
      XCTAssertFalse(ProgressTracker(lastEmittedBytes: 0, receivedBytes: 999, totalBytes: 1000).isComplete)
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 0, receivedBytes: 1000, totalBytes: 1000).isComplete)
      XCTAssertTrue(ProgressTracker(lastEmittedBytes: 0, receivedBytes: 1500, totalBytes: 1000).isComplete)
   }

   func testZeroTotalFallsBackToByteThreshold() {
      // A zero content length must not divide by zero; treat it as unknown.
      XCTAssertFalse(ProgressTracker(lastEmittedBytes: 0, receivedBytes: 1, totalBytes: 0).shouldEmit)
      XCTAssertTrue(
         ProgressTracker(
            lastEmittedBytes: 0,
            receivedBytes: ProgressTracker.bytesThreshold,
            totalBytes: 0
         ).shouldEmit
      )

      // A zero total would otherwise read as complete from the first byte.
      XCTAssertFalse(
         ProgressTracker(
            lastEmittedBytes: 0,
            receivedBytes: ProgressTracker.bytesThreshold,
            totalBytes: 0
         ).isComplete
      )
   }

   // MARK: - Unknown size

   func testEmitsEveryThresholdWhenSizeIsUnknown() {
      XCTAssertFalse(
         ProgressTracker(
            lastEmittedBytes: 0,
            receivedBytes: ProgressTracker.bytesThreshold - 1,
            totalBytes: nil
         ).shouldEmit
      )
      XCTAssertTrue(
         ProgressTracker(
            lastEmittedBytes: 0,
            receivedBytes: ProgressTracker.bytesThreshold,
            totalBytes: nil
         ).shouldEmit
      )
   }

   func testUnknownSizeThresholdIsRelativeToTheLastEmission() {
      XCTAssertFalse(
         ProgressTracker(
            lastEmittedBytes: 100,
            receivedBytes: 100 + ProgressTracker.bytesThreshold - 1,
            totalBytes: nil
         ).shouldEmit
      )
      XCTAssertTrue(
         ProgressTracker(
            lastEmittedBytes: 100,
            receivedBytes: 100 + ProgressTracker.bytesThreshold,
            totalBytes: nil
         ).shouldEmit
      )
   }

   func testIsNeverCompleteWhenSizeIsUnknown() {
      // Unknown-size completion is signalled by the stream ending.
      XCTAssertFalse(
         ProgressTracker(
            lastEmittedBytes: 0,
            receivedBytes: 10 * ProgressTracker.bytesThreshold,
            totalBytes: nil
         ).isComplete
      )
   }

   // MARK: - Restarted downloads

   func testRestartFromZeroEmitsImmediately() {
      // URLSession restarts from zero when resume data is rejected, so the
      // cumulative count can come back below the store's baseline. Emitting
      // resets it; staying quiet would strand the frontend on a stale count.
      // The unknown-size branch also subtracts, so this guards underflow.
      XCTAssertTrue(
         ProgressTracker(lastEmittedBytes: 5_000_000, receivedBytes: 65_536, totalBytes: nil).shouldEmit
      )
      XCTAssertTrue(
         ProgressTracker(lastEmittedBytes: 900, receivedBytes: 10, totalBytes: 1000).shouldEmit
      )
   }

   func testNormalThrottlingResumesOnceTheBaselineHasReset() {
      // Once the lower count is emitted it becomes the baseline.
      XCTAssertFalse(
         ProgressTracker(lastEmittedBytes: 65_536, receivedBytes: 65_536, totalBytes: nil).shouldEmit
      )
      XCTAssertTrue(
         ProgressTracker(
            lastEmittedBytes: 65_536,
            receivedBytes: 65_536 + ProgressTracker.bytesThreshold,
            totalBytes: nil
         ).shouldEmit
      )
   }
}
