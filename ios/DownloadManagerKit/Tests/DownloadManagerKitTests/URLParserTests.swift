import XCTest
@testable import DownloadManagerKit

final class URLParserTests: XCTestCase {

   // MARK: - parsePath tests

   func testValidPath() throws {
      XCTAssertNoThrow(try parsePath("/downloads/file.mp4"))
      XCTAssertNoThrow(try parsePath("/file.txt"))
   }

   func testEmptyPath() {
      XCTAssertThrowsError(try parsePath(""))
   }

   func testRelativePath() {
      XCTAssertThrowsError(try parsePath("relative/path.txt"))
      XCTAssertThrowsError(try parsePath("file.txt"))
   }

   func testPathWithoutFilename() {
      XCTAssertThrowsError(try parsePath("/"))
   }

   func testFileURLIsRejected() {
      XCTAssertThrowsError(try parsePath("file:///file.txt"))
   }

   func testPathIsReturnedAsGiven() throws {
      // A URL would percent-encode the space, handing back a different string from
      // the one JS keys its listeners by.
      XCTAssertEqual(try parsePath("/tmp/a b/x.zip"), "/tmp/a b/x.zip")
      XCTAssertEqual(try parsePath("/tmp//../x.zip"), "/tmp//../x.zip")
   }

   func testPathEndingInAParentSegmentThrows() {
      // Mirrors Rust's `Path::file_name()`, which returns `None` only when the path
      // terminates in `..` — an occurrence earlier in the path does not count.
      XCTAssertThrowsError(try parsePath("/a/b/.."))
   }

   func testPathsWithoutAFilenameThrow() {
      // Rust's `Path::file_name()` returns `None` for all of these.
      XCTAssertThrowsError(try parsePath("/.."))
      XCTAssertThrowsError(try parsePath("/."))
      XCTAssertThrowsError(try parsePath("/a/b/..//"))
      XCTAssertThrowsError(try parsePath("//"))
   }

   func testPathsEndingInCurrentOrParentSegmentsSucceed() throws {
      // A "." segment is skipped, and a ".." that is not the last segment does not
      // count, so Rust's `Path::file_name()` still returns the filename.
      XCTAssertEqual(try parsePath("/a/b/."), "/a/b/.")
      XCTAssertEqual(try parsePath("/a/../b"), "/a/../b")
      XCTAssertEqual(try parsePath("///a"), "///a")
      XCTAssertEqual(try parsePath("/a/b/"), "/a/b/")
   }

   // MARK: - parseURL tests

   func testValidUrls() throws {
      XCTAssertNoThrow(try parseURL("https://example.com/file.mp4"))
      XCTAssertNoThrow(try parseURL("http://example.com/file.mp4"))
      XCTAssertNoThrow(try parseURL("https://example.com:8080/file.mp4"))
      XCTAssertNoThrow(try parseURL("https://example.com/file.mp4?token=abc"))
   }

   func testEmptyUrl() {
      XCTAssertThrowsError(try parseURL(""))
   }

   func testInvalidScheme() {
      XCTAssertThrowsError(try parseURL("ftp://example.com/file.mp4"))
      XCTAssertThrowsError(try parseURL("file:///path/to/file.mp4"))
   }

   func testMissingHost() {
      XCTAssertThrowsError(try parseURL("https://:8080/file.mp4"))
   }

   func testInvalidUrlFormat() {
      XCTAssertThrowsError(try parseURL("not a valid url"))
   }

   func testURLWithCredentialsThrows() {
      XCTAssertThrowsError(try parseURL("https://user:pass@example.com/file.mp4"))
      XCTAssertThrowsError(try parseURL("https://user@example.com/file.mp4"))
      // A password with no username is still credentials.
      XCTAssertThrowsError(try parseURL("https://:pass@example.com/file.mp4"))
   }

   // MARK: - Messages

   func testPathMessagesMatchOtherPlatforms() {
      // Rust and Android send these exact strings for the same inputs.
      XCTAssertEqual(message { try parsePath("") }, "Path Error: path cannot be empty")
      XCTAssertEqual(message { try parsePath("file.txt") }, "Path Error: path must be absolute")
      XCTAssertEqual(message { try parsePath("/") }, "Path Error: path must have a filename")
      XCTAssertEqual(message { try parsePath("/a/b/..") }, "Path Error: path must have a filename")
   }

   func testURLMessagesMatchOtherPlatforms() {
      XCTAssertEqual(message { try parseURL("") }, "URL Error: URL cannot be empty")
      XCTAssertEqual(message { try parseURL("not a valid url") }, "URL Error: Invalid URL: not a valid url")
      XCTAssertEqual(message { try parseURL("example.com/file.mp4") }, "URL Error: Invalid URL: example.com/file.mp4")
      XCTAssertEqual(
         message { try parseURL("ftp://example.com/file.mp4") },
         "URL Error: Invalid URL scheme 'ftp': must be http or https"
      )
      XCTAssertEqual(
         message { try parseURL("https://user:pass@example.com/file.mp4") },
         "URL Error: URL must not contain credentials"
      )
   }

   /// The message the plugin rejects with: `localizedDescription` of the thrown error.
   private func message(_ body: () throws -> Any) -> String? {
      do {
         _ = try body()
         return nil
      } catch {
         return error.localizedDescription
      }
   }
}
