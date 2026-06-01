import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import {
   DownloadAction,
   type DownloadActionResponse,
   type DownloadState,
   DownloadStatus,
   expectedStatusesForAction,
} from './types';

export type MockDownloadCommand =
   | 'list'
   | 'get'
   | 'create'
   | 'start'
   | 'resume'
   | 'pause'
   | 'cancel'
   | 'is_native';

export interface MockDownloadInvocation {
   cmd: `plugin:download|${MockDownloadCommand}`;
   args: Record<string, unknown>;
}

export interface MockDownloadPluginOptions {
   downloads?: DownloadState<DownloadStatus>[];
}

export interface MockDownloadPluginController {
   clearCommandError(command: MockDownloadCommand): void;
   deleteDownload(path: string): boolean;
   emitChange(download: DownloadState<DownloadStatus>): Promise<void>;
   getDownload(path: string): DownloadState<DownloadStatus>;
   getInvocations(): MockDownloadInvocation[];
   getLastInvocation(): MockDownloadInvocation | null;
   listDownloads(): DownloadState<DownloadStatus>[];
   setCommandError(command: MockDownloadCommand, error: Error | string): void;
   setDownload(download: DownloadState<DownloadStatus>): void;
}

type MockActionResponse<A extends DownloadAction> = Omit<DownloadActionResponse<A>, 'download'> & {
   download: DownloadState<DownloadStatus>;
};

const DOWNLOAD_EVENT_NAME = 'tauri-plugin-download:changed';

const DEFAULT_URL = 'https://example.com/file.zip';

function cloneDownload<S extends DownloadStatus>(download: DownloadState<S>): DownloadState<S> {
   return { ...download };
}

function cloneDownloads(downloadsByPath: Map<string, DownloadState<DownloadStatus>>): DownloadState<DownloadStatus>[] {
   return [ ...downloadsByPath.values() ].map((download) => {
      return cloneDownload(download);
   });
}

function createPendingDownload(path: string): DownloadState<DownloadStatus.Pending> {
   return {
      url: '',
      path,
      progress: 0,
      status: DownloadStatus.Pending,
   };
}

function normalizeError(error: Error | string): Error {
   return error instanceof Error ? error : new Error(error);
}

function getExpectedStatus<A extends DownloadAction>(action: A): MockActionResponse<A>['expectedStatus'] {
   return expectedStatusesForAction[action][0] as MockActionResponse<A>['expectedStatus'];
}

function createActionResponse<A extends DownloadAction>(
   action: A,
   download: DownloadState<DownloadStatus>,
   isExpectedStatus: boolean
): MockActionResponse<A> {
   return {
      download: cloneDownload(download),
      expectedStatus: getExpectedStatus(action),
      isExpectedStatus,
   };
}

function getDownloadForPath(
   downloadsByPath: Map<string, DownloadState<DownloadStatus>>,
   path: string
): DownloadState<DownloadStatus> {
   return cloneDownload(downloadsByPath.get(path) ?? createPendingDownload(path));
}

function setDownloadForPath(
   downloadsByPath: Map<string, DownloadState<DownloadStatus>>,
   download: DownloadState<DownloadStatus>
): void {
   downloadsByPath.set(download.path, cloneDownload(download));
}

function getPathArg(args: Record<string, unknown>): string {
   return String(args.path);
}

function getUrlArg(args: Record<string, unknown>): string {
   return String(args.url ?? DEFAULT_URL);
}

function createTransitionDownload(
   currentDownload: DownloadState<DownloadStatus>,
   nextStatus: DownloadStatus,
   url?: string
): DownloadState<DownloadStatus> {
   return {
      ...currentDownload,
      url: url ?? currentDownload.url,
      status: nextStatus,
   };
}

export function createMockDownloadState(
   status: DownloadStatus,
   overrides: Partial<DownloadState<DownloadStatus>> = {}
): DownloadState<DownloadStatus> {
   return {
      url: overrides.url ?? DEFAULT_URL,
      path: overrides.path ?? '/tmp/file.zip',
      progress: overrides.progress ?? 0,
      status,
   };
}

export function clearDownloadMocks(): void {
   return clearMocks();
}

export function mockDownloadPlugin(
   options: MockDownloadPluginOptions = {}
): MockDownloadPluginController {
   const downloadsByPath = new Map<string, DownloadState<DownloadStatus>>();

   const invocations: MockDownloadInvocation[] = [];

   const commandErrors = new Map<MockDownloadCommand, Error>();

   for (const download of options.downloads ?? []) {
      setDownloadForPath(downloadsByPath, download);
   }

   function applyAction<A extends DownloadAction>(
      action: A,
      path: string,
      args: Record<string, unknown>
   ): MockActionResponse<A> {
      const currentDownload = getDownloadForPath(downloadsByPath, path);

      switch (action) {
         case DownloadAction.Create: {
            if (currentDownload.status !== DownloadStatus.Pending) {
               return createActionResponse(action, currentDownload, false);
            }

            const createdDownload = {
               ...currentDownload,
               url: getUrlArg(args),
               status: DownloadStatus.Idle,
            };

            setDownloadForPath(downloadsByPath, createdDownload);

            return createActionResponse(action, createdDownload, true);
         }
         case DownloadAction.Start: {
            if (currentDownload.status !== DownloadStatus.Idle) {
               return createActionResponse(action, currentDownload, false);
            }

            const nextDownload = createTransitionDownload(currentDownload, DownloadStatus.InProgress);

            setDownloadForPath(downloadsByPath, nextDownload);

            return createActionResponse(action, nextDownload, true);
         }
         case DownloadAction.Resume: {
            if (currentDownload.status !== DownloadStatus.Paused) {
               return createActionResponse(action, currentDownload, false);
            }

            const nextDownload = createTransitionDownload(currentDownload, DownloadStatus.InProgress);

            setDownloadForPath(downloadsByPath, nextDownload);

            return createActionResponse(action, nextDownload, true);
         }
         case DownloadAction.Pause: {
            if (currentDownload.status !== DownloadStatus.InProgress) {
               return createActionResponse(action, currentDownload, false);
            }

            const nextDownload = createTransitionDownload(currentDownload, DownloadStatus.Paused);

            setDownloadForPath(downloadsByPath, nextDownload);

            return createActionResponse(action, nextDownload, true);
         }
         case DownloadAction.Cancel: {
            const canCancel = currentDownload.status === DownloadStatus.Idle
               || currentDownload.status === DownloadStatus.InProgress
               || currentDownload.status === DownloadStatus.Paused;

            if (!canCancel) {
               return createActionResponse(action, currentDownload, false);
            }

            const nextDownload = createTransitionDownload(currentDownload, DownloadStatus.Canceled);

            setDownloadForPath(downloadsByPath, nextDownload);

            return createActionResponse(action, nextDownload, true);
         }
         default: {
            throw new Error(`Unsupported download action: ${action}`);
         }
      }
   }

   mockIPC((cmd, args) => {
      if (!cmd.startsWith('plugin:download|')) {
         return undefined;
      }

      const command = cmd.replace('plugin:download|', '') as MockDownloadCommand;

      const commandArgs = (args as Record<string, unknown> | undefined) ?? {};

      const invocation = {
         cmd: cmd as MockDownloadInvocation['cmd'],
         args: { ...commandArgs },
      };

      invocations.push(invocation);

      const commandError = commandErrors.get(command);

      if (commandError) {
         throw commandError;
      }

      switch (command) {
         case 'list': {
            return cloneDownloads(downloadsByPath);
         }
         case 'get': {
            return getDownloadForPath(downloadsByPath, getPathArg(invocation.args));
         }
         case 'create': {
            return applyAction(DownloadAction.Create, getPathArg(invocation.args), invocation.args);
         }
         case 'start': {
            return applyAction(DownloadAction.Start, getPathArg(invocation.args), invocation.args);
         }
         case 'resume': {
            return applyAction(DownloadAction.Resume, getPathArg(invocation.args), invocation.args);
         }
         case 'pause': {
            return applyAction(DownloadAction.Pause, getPathArg(invocation.args), invocation.args);
         }
         case 'cancel': {
            return applyAction(DownloadAction.Cancel, getPathArg(invocation.args), invocation.args);
         }
         case 'is_native': {
            return false;
         }
         default: {
            throw new Error(`Unsupported download command: ${command}`);
         }
      }
   }, { shouldMockEvents: true });

   return {
      clearCommandError(command: MockDownloadCommand): void {
         commandErrors.delete(command);
      },

      deleteDownload(path: string): boolean {
         return downloadsByPath.delete(path);
      },

      async emitChange(download: DownloadState<DownloadStatus>): Promise<void> {
         setDownloadForPath(downloadsByPath, download);
         await emit(DOWNLOAD_EVENT_NAME, cloneDownload(download));
      },

      getDownload(path: string): DownloadState<DownloadStatus> {
         return getDownloadForPath(downloadsByPath, path);
      },

      getInvocations(): MockDownloadInvocation[] {
         return invocations.map((invocation) => {
            return {
               cmd: invocation.cmd,
               args: { ...invocation.args },
            };
         });
      },

      getLastInvocation(): MockDownloadInvocation | null {
         const lastInvocation = invocations[invocations.length - 1];

         return lastInvocation ? {
            cmd: lastInvocation.cmd,
            args: { ...lastInvocation.args },
         } : null;
      },

      listDownloads(): DownloadState<DownloadStatus>[] {
         return cloneDownloads(downloadsByPath);
      },

      setCommandError(command: MockDownloadCommand, error: Error | string): void {
         commandErrors.set(command, normalizeError(error));
      },

      setDownload(download: DownloadState<DownloadStatus>): void {
         setDownloadForPath(downloadsByPath, download);
      },
   };
}
