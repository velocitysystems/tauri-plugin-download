import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { DOWNLOAD_EVENT_NAME, resetDownloadEventManager } from './actions';
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

/**
 * Controls the mocked download plugin state and invocation history during tests.
 */
export interface MockDownloadPluginController {

   /**
    * Clears a previously configured error for a mocked command.
    *
    * @param command - The command whose mocked error should be removed.
    */
   clearCommandError(command: MockDownloadCommand): void;

   /**
    * Removes a mocked download from the in-memory store.
    *
    * @param path - The download path to remove.
    * @returns `true` if a download existed for the path and was removed.
    */
   deleteDownload(path: string): boolean;

   /**
    * Updates a mocked download and emits the corresponding change event.
    *
    * @param download - The download state to store and broadcast.
    * @returns A promise that resolves after the change event is emitted.
    */
   emitChange(download: DownloadState<DownloadStatus>): Promise<void>;

   /**
    * Gets the mocked download state for a path.
    *
    * @param path - The download path to look up.
    * @returns The stored download, or a pending download when none exists yet.
    */
   getDownload(path: string): DownloadState<DownloadStatus>;

   /**
    * Gets a snapshot of all mocked plugin invocations.
    *
    * @returns The recorded invocations in call order.
    */
   getInvocations(): MockDownloadInvocation[];

   /**
    * Gets the most recent mocked plugin invocation.
    *
    * @returns The last recorded invocation, or `null` if none have occurred.
    */
   getLastInvocation(): MockDownloadInvocation | null;

   /**
    * Lists all mocked downloads currently stored in memory.
    *
    * @returns The mocked downloads.
    */
   listDownloads(): DownloadState<DownloadStatus>[];

   /**
    * Configures a mocked command to throw the provided error when invoked.
    *
    * @param command - The command that should fail.
    * @param error - The error instance or message to throw.
    */
   setCommandError(command: MockDownloadCommand, error: Error | string): void;

   /**
    * Stores or replaces a mocked download without emitting a change event.
    *
    * @param download - The download state to store.
    */
   setDownload(download: DownloadState<DownloadStatus>): void;
}

type MockActionResponse<A extends DownloadAction> = Omit<DownloadActionResponse<A>, 'download'> & {
   download: DownloadState<DownloadStatus>;
};

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
      receivedBytes: 0,
      totalBytes: null,
      progress: 0,
      status: DownloadStatus.Pending,
   };
}

function normalizeError(error: Error | string): Error {
   return error instanceof Error ? error : new Error(error);
}

function getExpectedStatus<A extends DownloadAction>(action: A): MockActionResponse<A>['expectedStatus'] {
   const expectedStatus = expectedStatusesForAction[action][0];

   if (!expectedStatus) {
      throw new Error(`Mocked download action ${action} must define at least one expected status`);
   }

   return expectedStatus as MockActionResponse<A>['expectedStatus'];
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

function createNoOpActionResponse<A extends DownloadAction>(
   action: A,
   download: DownloadState<DownloadStatus>
): MockActionResponse<A> {
   const expectedStatus = getExpectedStatus(action);

   return {
      download: cloneDownload(download),
      expectedStatus,
      isExpectedStatus: download.status === expectedStatus,
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

/**
 * Creates a mock download state object for tests.
 *
 * Unless `progress` is explicitly overridden, progress is derived from
 * `receivedBytes` and `totalBytes`. Unknown-size downloads use
 * `totalBytes: null`, and completed downloads default to `progress: 100`.
 *
 * @param status - The download status to assign to the mock state.
 * @param overrides - Optional properties to override on the generated state.
 * @returns A mock download state with defaults for unspecified fields.
 */
export function createMockDownloadState(
   status: DownloadStatus,
   overrides: Partial<DownloadState<DownloadStatus>> = {}
): DownloadState<DownloadStatus> {
   const receivedBytes = overrides.receivedBytes ?? 0;

   const totalBytes = overrides.totalBytes ?? null;

   let computedProgress = 0;

   if (overrides.progress !== undefined) {
      computedProgress = overrides.progress;
   } else if (status === DownloadStatus.Completed) {
      computedProgress = 100;
   } else if (totalBytes !== null && totalBytes > 0) {
      computedProgress = Math.min((receivedBytes / totalBytes) * 100, 100);
   }

   return {
      url: overrides.url ?? DEFAULT_URL,
      path: overrides.path ?? '/tmp/file.zip',
      receivedBytes,
      totalBytes,
      progress: computedProgress,
      status,
   };
}

/**
 * Clears all configured Tauri mocks used by the download guest API tests.
 */
export function clearDownloadMocks(): void {
   resetDownloadEventManager();
   clearMocks();
}

/**
 * Configures Tauri IPC mocks for unit tests that exercise the guest JS download API.
 *
 * This helper approximates backend/native state transitions for common test flows.
 * It is not a backend contract and does not transition downloads to `Completed`.
 * Create options are recorded in invocation history, but network-policy enforcement
 * is not simulated.
 * It only simulates the desktop event path and always returns `false` for `is_native`,
 * so tests that need the native/mobile listener branch require a separate approach.
 * Use `emitChange()` or `setDownload()` to simulate progress updates or terminal states.
 *
 * @param options Initial mocked download state.
 * @return Controller for inspecting invocations and mutating mocked download state.
 */
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
               return createNoOpActionResponse(action, currentDownload);
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
               return createNoOpActionResponse(action, currentDownload);
            }

            const nextDownload = createTransitionDownload(currentDownload, DownloadStatus.InProgress);

            setDownloadForPath(downloadsByPath, nextDownload);

            return createActionResponse(action, nextDownload, true);
         }
         case DownloadAction.Resume: {
            if (currentDownload.status !== DownloadStatus.Paused) {
               return createNoOpActionResponse(action, currentDownload);
            }

            const nextDownload = createTransitionDownload(currentDownload, DownloadStatus.InProgress);

            setDownloadForPath(downloadsByPath, nextDownload);

            return createActionResponse(action, nextDownload, true);
         }
         case DownloadAction.Pause: {
            if (currentDownload.status !== DownloadStatus.InProgress) {
               return createNoOpActionResponse(action, currentDownload);
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
               return createNoOpActionResponse(action, currentDownload);
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

      /**
       * Emits a mocked desktop download change event.
       */
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
