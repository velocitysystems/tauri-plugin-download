package org.silvermine.downloadmanager

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import kotlinx.serialization.SerializationException
import org.junit.Test

/**
 * Covers the store's decode and encode, which is where a malformed file decides
 * whether one record or every record is lost.
 *
 * The `AtomicFile` wiring around them needs an Android runtime, so it is not
 * exercised here — `android.util.AtomicFile` and `android.util.Log` are throwing
 * stubs in a JVM unit test.
 */
class DownloadStoreTest {

   private fun sampleRecord(path: String, receivedBytes: Long = 0L): DownloadRecord = DownloadRecord(
      url = "http://example.com/$path",
      path = "/tmp/$path",
      receivedBytes = receivedBytes,
      totalBytes = 1000L,
      status = DownloadStatus.Paused,
   )

   // -- Decoding --

   @Test
   fun `decodes persisted records`() {
      val decoded = DownloadStore.decodeRecords(
         """[{"url":"http://example.com/a.mp4","path":"/tmp/a.mp4","receivedBytes":500,"totalBytes":1000,"status":"paused"}]"""
      )

      assertEquals(1, decoded.size)
      assertEquals(500L, decoded.first().receivedBytes)
      assertEquals(1000L, decoded.first().totalBytes)
      assertEquals(DownloadStatus.Paused, decoded.first().status)
   }

   @Test
   fun `one unreadable record discards the whole store`() {
      // Pins today's behaviour, which is not the behaviour we want: the array is
      // decoded in one call, so the middle element takes both good records with it
      // and load() falls back to an empty store. Invert this test when per-record
      // decoding lands (#64).
      //
      // Scoped to the decode call so the inverted test fails for a known reason.
      // Type only: the message wording is kotlinx's, not a contract.
      assertThrows(SerializationException::class.java) {
         DownloadStore.decodeRecords(
            """
            [{"url":"http://example.com/a.mp4","path":"/tmp/a.mp4","receivedBytes":1,"status":"paused"},
             {"url":"http://example.com/b.mp4","status":"paused"},
             {"url":"http://example.com/c.mp4","path":"/tmp/c.mp4","receivedBytes":3,"status":"idle"}]
            """.trimIndent()
         )
      }
   }

   @Test
   fun `a record of the wrong shape fails the decode`() {
      assertThrows(SerializationException::class.java) {
         DownloadStore.decodeRecords("""["not an object"]""")
      }
   }

   @Test
   fun `unknown keys are ignored`() {
      // The store's Json is configured with ignoreUnknownKeys, so a field added by
      // a later version does not cost the record.
      val decoded = DownloadStore.decodeRecords(
         """[{"url":"http://example.com/a.mp4","path":"/tmp/a.mp4","receivedBytes":7,"status":"idle","somethingNew":42}]"""
      )

      assertEquals(7L, decoded.first().receivedBytes)
   }

   @Test
   fun `absent byte fields fall back to their defaults`() {
      val decoded = DownloadStore.decodeRecords(
         """[{"url":"http://example.com/a.mp4","path":"/tmp/a.mp4","status":"idle"}]"""
      )

      assertEquals(0L, decoded.first().receivedBytes)
      assertNull(decoded.first().totalBytes)
   }

   @Test
   fun `an empty array decodes to nothing`() {
      val decoded = DownloadStore.decodeRecords("[]")

      assertEquals(0, decoded.size)
   }

   @Test
   fun `text that is not a json array is rejected`() {
      // load() catches these and leaves the store empty rather than half-built.
      for (text in listOf("this is not json", """{"url":"http://example.com/a.mp4"}""", "")) {
         assertThrows(SerializationException::class.java) { DownloadStore.decodeRecords(text) }
      }
   }

   // -- Round trip --

   @Test
   fun `encoded records decode back unchanged`() {
      val records = listOf(sampleRecord("a.mp4", 10L), sampleRecord("b.mp4", 20L))

      val decoded = DownloadStore.decodeRecords(DownloadStore.encodeRecords(records))

      assertEquals(records, decoded)
   }

   @Test
   fun `a record of every default survives the round trip`() {
      // The store's Json leaves encodeDefaults off, so a default-valued record
      // persists as url and path alone — the shape production actually writes.
      val record = DownloadRecord(url = "http://example.com/a.mp4", path = "/tmp/a.mp4")

      val encoded = DownloadStore.encodeRecords(listOf(record))
      val decoded = DownloadStore.decodeRecords(encoded)

      assertFalse(encoded.contains("receivedBytes"))
      assertEquals(listOf(record), decoded)
      assertEquals(0L, decoded.first().receivedBytes)
      assertNull(decoded.first().totalBytes)
   }
}
