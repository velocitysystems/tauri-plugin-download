import XCTest
@testable import DownloadManagerKit

final class DownloadRecordTests: XCTestCase {

   private func sampleRecord() -> DownloadRecord {
      return DownloadRecord(
         url: URL(string: "http://example.com/file.mp4")!,
         path: URL(fileURLWithPath: "/tmp/file.mp4"),
         receivedBytes: 0,
         totalBytes: nil,
         status: .idle
      )
   }

   // MARK: - Mutation

   func testSetBytes() {
      var record = sampleRecord()
      record.setBytes(received: 500, total: 1000)

      XCTAssertEqual(record.receivedBytes, 500)
      XCTAssertEqual(record.totalBytes, 1000)
      XCTAssertEqual(record.status, .idle)
      XCTAssertEqual(record.url, sampleRecord().url)
      XCTAssertEqual(record.path, sampleRecord().path)
   }

   func testSetStatusPreservesBytes() {
      var record = sampleRecord()
      record.setBytes(received: 500, total: 1000)

      var paused = record
      paused.setStatus(.paused)
      XCTAssertEqual(paused.receivedBytes, 500)
      XCTAssertEqual(paused.totalBytes, 1000)
      XCTAssertEqual(paused.status, .paused)

      // Status transitions preserve the factual byte counts, including completion.
      var completed = record
      completed.setStatus(.completed)
      XCTAssertEqual(completed.receivedBytes, 500)
      XCTAssertEqual(completed.totalBytes, 1000)
      XCTAssertEqual(completed.status, .completed)
      XCTAssertEqual(completed.toItem().progress, 100.0)
   }

   func testSetStatusCompletedWithUnknownSize() {
      // Completed with unknown size preserves receivedBytes.
      var record = sampleRecord()
      record.setBytes(received: 5000, total: nil)
      record.setStatus(.completed)

      XCTAssertEqual(record.receivedBytes, 5000)
      XCTAssertNil(record.totalBytes)
   }

   // MARK: - Progress derivation

   func testToItemWithKnownSize() {
      var record = sampleRecord()
      record.setBytes(received: 500, total: 1000)

      let item = record.toItem()
      XCTAssertEqual(item.progress, 50.0)
      XCTAssertEqual(item.receivedBytes, 500)
      XCTAssertEqual(item.totalBytes, 1000)
   }

   func testToItemClampsProgressTo100Percent() {
      var record = sampleRecord()
      record.setBytes(received: 1500, total: 1000)

      XCTAssertEqual(record.toItem().progress, 100.0)
   }

   func testToItemWithUnknownSize() {
      let record = sampleRecord()
      XCTAssertEqual(record.toItem().progress, 0.0)

      // Completed with unknown size still reports 100%.
      var completed = record
      completed.setStatus(.completed)
      XCTAssertEqual(completed.toItem().progress, 100.0)
   }

   func testToItemWithZeroTotal() {
      // A zero content length must not divide by zero.
      var record = sampleRecord()
      record.setBytes(received: 0, total: 0)

      XCTAssertEqual(record.toItem().progress, 0.0)
   }

   // MARK: - Encoding

   func testItemEncodesTotalBytesAsExplicitNull() throws {
      // The key is always present, so the payload matches the frontend contract
      // without relying on the TypeScript layer to coalesce a missing key.
      let record = sampleRecord()
      let json = try JSONSerialization.jsonObject(
         with: try JSONEncoder().encode(record.toItem())
      ) as? [String: Any]

      XCTAssertTrue(try XCTUnwrap(json).keys.contains("totalBytes"))
      XCTAssertTrue(try XCTUnwrap(json)["totalBytes"] is NSNull)
   }

   func testItemEncodesByteFields() throws {
      var record = sampleRecord()
      record.setBytes(received: 500, total: 1000)

      let json = try XCTUnwrap(
         try JSONSerialization.jsonObject(
            with: try JSONEncoder().encode(record.toItem())
         ) as? [String: Any]
      )

      XCTAssertEqual(json["receivedBytes"] as? UInt64, 500)
      XCTAssertEqual(json["totalBytes"] as? UInt64, 1000)
      XCTAssertEqual(json["progress"] as? Double, 50.0)
      XCTAssertEqual(json["status"] as? String, "idle")
   }

   func testItemDoesNotEncodeResumeDataPath() throws {
      // resumeDataPath is internal to URLSession and must not reach the frontend.
      var record = sampleRecord()
      record.setResumeDataPath(URL(fileURLWithPath: "/tmp/abc.resumedata"))

      let json = try XCTUnwrap(
         try JSONSerialization.jsonObject(
            with: try JSONEncoder().encode(record.toItem())
         ) as? [String: Any]
      )

      XCTAssertFalse(json.keys.contains("resumeDataPath"))
   }

   func testRecordDecodesPersistedWireFormat() throws {
      // A literal payload, not a round trip, which would pass even if the
      // field names drifted on both sides together.
      let json = """
      {"url":"http://example.com/f.mp4","path":"file:///tmp/f.mp4",\
      "receivedBytes":500,"totalBytes":1000,"status":"paused"}
      """

      let record = try JSONDecoder().decode(DownloadRecord.self, from: Data(json.utf8))

      XCTAssertEqual(record.receivedBytes, 500)
      XCTAssertEqual(record.totalBytes, 1000)
      XCTAssertEqual(record.status, .paused)
      // progress is derived via toItem(), not stored.
      XCTAssertEqual(record.toItem().progress, 50.0)
   }

   func testRecordRoundTripsThroughJSON() throws {
      var record = sampleRecord()
      record.setBytes(received: 500, total: 1000)
      record.setStatus(.paused)

      let decoded = try JSONDecoder().decode(
         DownloadRecord.self,
         from: try JSONEncoder().encode(record)
      )

      XCTAssertEqual(decoded.receivedBytes, 500)
      XCTAssertEqual(decoded.totalBytes, 1000)
      XCTAssertEqual(decoded.status, .paused)
      // progress is derived via toItem(), not stored.
      XCTAssertEqual(decoded.toItem().progress, 50.0)
   }

   // MARK: - Action response

   func testActionResponse() {
      let item = sampleRecord().toItem()

      // The single-argument initializer reports the status as expected.
      let response = DownloadActionResponse(download: item)
      XCTAssertTrue(response.isExpectedStatus)
      XCTAssertEqual(response.expectedStatus, .idle)

      // A matching expected status is still expected.
      let matching = DownloadActionResponse(download: item, expectedStatus: .idle)
      XCTAssertTrue(matching.isExpectedStatus)

      // A mismatched expected status is not.
      let mismatched = DownloadActionResponse(download: item, expectedStatus: .inProgress)
      XCTAssertFalse(mismatched.isExpectedStatus)
      XCTAssertEqual(mismatched.expectedStatus, .inProgress)
   }
}
