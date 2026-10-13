import { describe, expect, expectTypeOf, it } from 'vitest';
import {
   DOWNLOAD_ERROR_CODES,
   DownloadError,
   DownloadErrorCode,
   DownloadHttpError,
   DownloadNonHttpError,
   isDownloadError,
} from './index';

describe('download error exports', () => {
   it('derives the code type from the exported values', () => {
      expectTypeOf<DownloadErrorCode>().toEqualTypeOf<(typeof DOWNLOAD_ERROR_CODES)[number]>();
   });

   it.each(DOWNLOAD_ERROR_CODES)('accepts the %s code', (code) => {
      const error = code === 'http'
         ? { code, message: 'HTTP 429', httpStatus: 429 }
         : { code, message: 'Download failed' };

      expect(isDownloadError(error)).toBe(true);
   });

   it('accepts extra fields and empty messages', () => {
      expect(isDownloadError({ code: 'file', message: '', detail: 'extra' })).toBe(true);
      expect(isDownloadError({ code: 'http', message: '', httpStatus: 503, detail: 'extra' })).toBe(true);
   });

   it('accepts an undefined HTTP status on a non-HTTP error', () => {
      expect(isDownloadError({ code: 'unknown', message: 'Download failed', httpStatus: undefined })).toBe(true);
   });

   it.each([
      { code: 'http', message: 'HTTP failure' },
      { code: 'http', message: 'HTTP failure', httpStatus: undefined },
      { code: 'http', message: 'HTTP failure', httpStatus: null },
      { code: 'http', message: 'HTTP failure', httpStatus: '429' },
      { code: 'http', message: 'HTTP failure', httpStatus: true },
      { code: 'file', message: 'File failure', httpStatus: 429 },
      { code: 'file', message: 'File failure', httpStatus: null },
      { code: 'file', message: 'File failure', httpStatus: '429' },
   ])(
      'rejects an invalid HTTP status field: %j',
      (value) => {
         expect(isDownloadError(value)).toBe(false);
      }
   );

   it.each([
      null,
      undefined,
      'Download failed',
      429,
      true,
      [],
      [ { code: 'file', message: 'File failure' } ],
      Object.assign([], { code: 'file', message: 'File failure' }),
      new Error('Download failed'),
      {},
      { message: 'Missing code' },
      { code: 'file' },
      { code: 'future error', message: 'New native failure' },
      { code: 429, message: 'Invalid code' },
      { code: null, message: 'Invalid code' },
      { code: 'file', message: 42 },
      { code: 'file', message: null },
      { code: 'file', message: undefined },
   ])(
      'rejects a value outside the error contract: %j',
      (value) => {
         expect(isDownloadError(value)).toBe(false);
      }
   );

   it('narrows unknown values and discriminates HTTP errors', () => {
      const value: unknown = { code: 'http', message: 'HTTP 429', httpStatus: 429 };

      expect(isDownloadError(value)).toBe(true);
      if (isDownloadError(value)) {
         expectTypeOf(value).toEqualTypeOf<DownloadError>();
         if (value.code === 'http') {
            expectTypeOf(value).toEqualTypeOf<DownloadHttpError>();
            expectTypeOf(value.httpStatus).toEqualTypeOf<number>();
            expect(value.httpStatus).toBe(429);
         } else {
            expectTypeOf(value).toEqualTypeOf<DownloadNonHttpError>();
            expectTypeOf(value.httpStatus).toEqualTypeOf<undefined>();
         }
      }
   });
});
