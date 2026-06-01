import { afterEach, describe, expect, it, vi } from 'vitest';
import { get, list } from './index';
import { clearDownloadMocks, createMockDownloadState, mockDownloadPlugin } from './mocks';
import { DownloadAction, DownloadStatus, hasAction } from './types';

afterEach(() => {
   clearDownloadMocks();
});

describe('mockDownloadPlugin', () => {
   it('seeds downloads for list and records invocations', async () => {
      const controller = mockDownloadPlugin({
         downloads: [
            createMockDownloadState(DownloadStatus.Idle, {
               path: '/tmp/seeded.zip',
               progress: 12,
            }),
         ],
      });

      const downloads = await list();

      expect(downloads).toHaveLength(1);
      expect(downloads[0].path).toBe('/tmp/seeded.zip');
      expect(downloads[0].status).toBe(DownloadStatus.Idle);
      expect(hasAction(downloads[0], DownloadAction.Start)).toBe(true);
      expect(controller.getLastInvocation()).toEqual({
         cmd: 'plugin:download|list',
         args: {},
      });
   });

   it('transitions Pending to Idle on create and updates stored state', async () => {
      const controller = mockDownloadPlugin();

      const download = await get('/tmp/new.zip');

      expect(download.status).toBe(DownloadStatus.Pending);
      expect(hasAction(download, DownloadAction.Create)).toBe(true);
      if (!hasAction(download, DownloadAction.Create)) {
         throw new Error('expected create action');
      }

      const response = await download.create('https://example.com/new.zip');

      expect(response.isExpectedStatus).toBe(true);
      expect(response.download.status).toBe(DownloadStatus.Idle);
      expect(controller.getDownload('/tmp/new.zip')).toEqual({
         url: 'https://example.com/new.zip',
         path: '/tmp/new.zip',
         progress: 0,
         status: DownloadStatus.Idle,
      });
   });

   it('emits mocked desktop events to download listeners', async () => {
      const controller = mockDownloadPlugin({
         downloads: [
            createMockDownloadState(DownloadStatus.Idle, {
               path: '/tmp/listener.zip',
            }),
         ],
      });

      const download = await get('/tmp/listener.zip');

      const listener = vi.fn();

      if (!hasAction(download, DownloadAction.Listen)) {
         throw new Error('expected listen action');
      }

      const unlisten = await download.listen(listener);

      await controller.emitChange(createMockDownloadState(DownloadStatus.InProgress, {
         path: '/tmp/listener.zip',
         progress: 42,
      }));

      expect(listener).toHaveBeenCalledTimes(1);
      expect(listener).toHaveBeenCalledWith(expect.objectContaining({
         path: '/tmp/listener.zip',
         progress: 42,
         status: DownloadStatus.InProgress,
      }));
      expect(hasAction(listener.mock.calls[0][0], DownloadAction.Pause)).toBe(true);

      unlisten();
   });

   it('allows command errors to be injected per action', async () => {
      const controller = mockDownloadPlugin({
         downloads: [
            createMockDownloadState(DownloadStatus.Idle, {
               path: '/tmp/error.zip',
            }),
         ],
      });

      const download = await get('/tmp/error.zip');

      controller.setCommandError('start', 'start failed');

      if (!hasAction(download, DownloadAction.Start)) {
         throw new Error('expected start action');
      }

      await expect(download.start()).rejects.toThrow('start failed');

      controller.clearCommandError('start');

      const response = await download.start();

      expect(response.isExpectedStatus).toBe(true);
      expect(response.download.status).toBe(DownloadStatus.InProgress);
   });
});
