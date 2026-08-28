import XCTest
@testable import DownloadManagerKit

/// Covers where the store decides to persist itself.
///
/// `StoreLocation` is process-wide, so each test resets it on both sides. `tearDown`
/// alone protects the tests that follow but not this class from what precedes it:
/// XCTest orders classes arbitrarily within one process, so anything that constructs a
/// `DownloadStore` with its default path first would leave `isResolved` set and trap
/// the whole bundle here rather than failing one test.
final class StoreLocationTests: XCTestCase {

   override func setUp() {
      super.setUp()
      StoreLocation.resetForTesting()
   }

   override func tearDown() {
      StoreLocation.resetForTesting()
      super.tearDown()
   }

   func testTheDefaultPathIsInTheDocumentsDirectory() {
      // The behaviour an app that configures nothing keeps. Asserting the filename
      // alongside the directory: both platforms and the Rust store agree on it, and a
      // change here would silently orphan every existing store.
      let path = StoreLocation.savePath()

      XCTAssertEqual(path.lastPathComponent, "downloads.json")
      XCTAssertEqual(path.deletingLastPathComponent(), StoreLocation.defaultDirectory)
   }

   func testAConfiguredDirectoryHoldsTheStore() {
      let directory = URL(fileURLWithPath: "/tmp/configured", isDirectory: true)

      StoreLocation.set(directory)

      XCTAssertEqual(StoreLocation.savePath(), directory.appendingPathComponent("downloads.json"))
   }

   func testAConfiguredDirectoryReplacesTheDefault() {
      // Pairs with the case above: without it, a `set` that appended to the default
      // rather than replacing it would still produce a path containing the directory.
      let directory = URL(fileURLWithPath: "/tmp/configured", isDirectory: true)

      StoreLocation.set(directory)

      XCTAssertNotEqual(
         StoreLocation.savePath().deletingLastPathComponent(),
         StoreLocation.defaultDirectory
      )
   }

   func testTheLastDirectorySetBeforeTheStoreOpensWins() {
      // The plugin sets once, but nothing in the type forbids two calls before the
      // store opens, and latching the first would be the wrong half of the contract:
      // it is setting *after* the open that is refused.
      StoreLocation.set(URL(fileURLWithPath: "/tmp/first", isDirectory: true))
      StoreLocation.set(URL(fileURLWithPath: "/tmp/second", isDirectory: true))

      XCTAssertEqual(StoreLocation.savePath().deletingLastPathComponent().path, "/tmp/second")
   }
}
