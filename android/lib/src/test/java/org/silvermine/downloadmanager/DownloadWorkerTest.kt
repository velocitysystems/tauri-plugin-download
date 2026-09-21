package org.silvermine.downloadmanager

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.IOException
import java.io.InterruptedIOException
import java.net.ProtocolException
import java.net.SocketException
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import java.security.cert.CertificateException
import javax.net.ssl.SSLException
import javax.net.ssl.SSLHandshakeException
import javax.net.ssl.SSLPeerUnverifiedException

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

   // -- Resume failure outcome --

   @Test
   fun `a 416 stating a total equal to the partial completes it`() {
      assertEquals(
         DownloadWorker.PartialFileOutcome.Complete,
         DownloadWorker.partialFileOutcomeFor(416, "bytes */1000", 1000L),
      )
   }

   @Test
   fun `any other 416 discards the partial`() {
      for (contentRange in listOf(null, "bytes */999", "bytes 0-499/1000", "1000", "bytes */abc")) {
         assertEquals(
            "Content-Range $contentRange",
            DownloadWorker.PartialFileOutcome.Discard,
            DownloadWorker.partialFileOutcomeFor(416, contentRange, 1000L),
         )
      }
   }

   @Test
   fun `other failures on a resume keep the partial`() {
      for (responseCode in listOf(503, 500, 404, 403)) {
         assertEquals(
            "HTTP $responseCode",
            DownloadWorker.PartialFileOutcome.KeepPartial,
            DownloadWorker.partialFileOutcomeFor(responseCode, "bytes */1000", 1000L),
         )
      }
   }

   // -- Failure classification --
   //
   // Transient means the partial survives and the work is retried.

   @Test
   fun `a connect timeout is transient whatever the platform calls it`() {
      // The JVM's wording and Android libcore's. Neither says "timeout".
      assertTrue(DownloadWorker.isTransient(SocketTimeoutException("Connect timed out")))
      assertTrue(
         DownloadWorker.isTransient(
            SocketTimeoutException(
               "failed to connect to example.com/93.184.216.34 (port 443) from /10.0.2.15 (port 41234) after 30000ms",
            ),
         ),
      )
   }

   @Test
   fun `a read timeout is transient from either racing source`() {
      // OkHttp sets the socket timeout to the same interval as Okio's watchdog, so
      // which message arrives is a race. Both must classify alike.
      assertTrue(DownloadWorker.isTransient(SocketTimeoutException("timeout")))
      assertTrue(DownloadWorker.isTransient(SocketTimeoutException("Read timed out")))
   }

   @Test
   fun `an interrupt that is not a timeout is permanent`() {
      // This process tearing the read down, not the network failing.
      assertFalse(DownloadWorker.isTransient(InterruptedIOException("thread interrupted")))
   }

   @Test
   fun `a DNS failure is permanent`() {
      assertFalse(DownloadWorker.isTransient(UnknownHostException("example.invalid")))
   }

   @Test
   fun `a mid-stream TLS failure is transient`() {
      // Conscrypt reports a reset inside an established TLS session this way.
      assertTrue(DownloadWorker.isTransient(SSLException("Read error: ssl=0x0: I/O error during system call, Connection reset by peer")))
   }

   @Test
   fun `a certificate failure is permanent`() {
      // Retrying cannot make an untrusted certificate trusted.
      val handshake = SSLHandshakeException("Trust anchor for certification path not found")

      handshake.initCause(CertificateException("untrusted root"))

      assertFalse(DownloadWorker.isTransient(handshake))
      assertFalse(DownloadWorker.isTransient(SSLPeerUnverifiedException("Hostname example.com not verified")))
   }

   @Test
   fun `a handshake failure with no certificate cause is transient`() {
      // Matches OkHttp's isRecoverable, which refuses only the certificate case.
      assertTrue(DownloadWorker.isTransient(SSLHandshakeException("Connection closed by peer")))
   }

   @Test
   fun `an ordinary network failure is transient`() {
      assertTrue(DownloadWorker.isTransient(SocketException("Connection reset")))
      assertTrue(DownloadWorker.isTransient(IOException("unexpected end of stream")))
      assertTrue(DownloadWorker.isTransient(ProtocolException("unexpected status line")))
   }
}
