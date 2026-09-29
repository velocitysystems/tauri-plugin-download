import { describe, expect, expectTypeOf, it } from 'vitest';
import { DownloadError, DownloadState, DownloadStatus, DownloadWithAnyStatus } from './types';
import { createMockDownloadState } from './mocks';
import { attachDownload } from './actions';

describe('failure contract', () => {
   it('requires an error only on failed downloads', () => {
      expectTypeOf<DownloadState<DownloadStatus.Failed>>().toMatchTypeOf<{ error: DownloadError }>();
      expectTypeOf<DownloadState<Exclude<DownloadStatus, DownloadStatus.Failed>>>().toMatchTypeOf<{ error?: never }>();
      expectTypeOf<Extract<DownloadWithAnyStatus, { status: DownloadStatus.Failed }>['error']>().toEqualTypeOf<DownloadError>();
      expectTypeOf<Extract<DownloadError, { code: 'http' }>>().toMatchTypeOf<{ httpStatus: number }>();
      expectTypeOf<Exclude<DownloadError, { code: 'http' }>>().toMatchTypeOf<{ httpStatus?: never }>();
   });

   it('creates failed mock states with a fallback error', () => {
      const failed = attachDownload(createMockDownloadState(DownloadStatus.Failed));

      expect(failed.error).toEqual({ code: 'unknown', message: 'Download failed' });
      const idle = attachDownload(createMockDownloadState(DownloadStatus.Idle));

      expect(idle).not.toHaveProperty('error');
   });
});
