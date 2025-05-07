import { invoke, addPluginListener } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

/**
 * Manages subscriptions to download events from Rust and
 * mobile plugins (iOS/Android), and dispatching these events
 * to registered listeners.
 */
export class DownloadEventManager {
   public static shared: DownloadEventManager = new DownloadEventManager();
   private _listeners: Map<string, Set<(download: Download) => void>> = new Map();
   private _eventUnlistenFn: UnlistenFn | null = null;
   private _pluginListener: { unregister: () => void } | null = null;

   private constructor() { }

   /**
    * Adds a listener for download events.
    * @param key - The key of the download item to listen for.
    * @param listener - The callback function to invoke when the download changes.
    * @returns A promise with a function to remove this specific listener.
    */
   public async addListener(key: string, listener: (download: Download) => void): Promise<() => void> {
      await this.ensureGlobalListeners();

      if (!this._listeners.has(key)) {
         this._listeners.set(key, new Set());
      }

      const listenersForKey = this._listeners.get(key);

      if (listenersForKey) {
         listenersForKey.add(listener);
      }

      // Return a function to remove this specific listener
      return () => {
         const listeners = this._listeners.get(key);

         if (listeners) {
            listeners.delete(listener);

            // If no more listeners for this key, remove the key from the map.
            if (listeners.size === 0) {
               this._listeners.delete(key);
            }
         }

         this.cleanupGlobalListeners();
      };
   }

   private async ensureGlobalListeners(): Promise<void> {
      if (this._eventUnlistenFn && this._pluginListener) {
         return;
      }

      // Check if the plugin is running in a native environment (iOS)
      // or is the shared Rust implementation (desktop/Android).
      const isNative = await invoke<boolean>('plugin:download|is_native');

      if (isNative) {
         this._pluginListener = await addPluginListener('download', 'changed', (event: DownloadItem) => {
            this.notifyListeners(event.key, event);
         });
      } else {
         this._eventUnlistenFn = await listen<DownloadItem>('tauri-plugin-download:changed', (event) => {
            this.notifyListeners(event.payload.key, event.payload);
         });
      }
   }

   private notifyListeners(key: string, event: DownloadItem): void {
      const listeners = this._listeners.get(key);

      if (listeners) {
         // eslint-disable-next-line @typescript-eslint/no-use-before-define
         listeners.forEach((listener) => { return listener(new Download(event)); });
      }
   }

   private cleanupGlobalListeners(): void {
      if (this._listeners.size === 0) {
         if (this._eventUnlistenFn) {
            this._eventUnlistenFn();
            this._eventUnlistenFn = null;
         }

         if (this._pluginListener) {
            this._pluginListener.unregister();
            this._pluginListener = null;
         }
      }
   }
}

/**
 * Represents a download item with methods to control its lifecycle.
 * This class wraps a download item and provides methods to start, cancel, pause, resume,
 * and listen for changes to the download.
 */
export class Download implements DownloadItem {
   public key: string;
   public url: string;
   public path: string;
   public progress: number;
   public state: DownloadState;

   public constructor(item: DownloadItem) {
      this.key = item.key;
      this.url = item.url;
      this.path = item.path;
      this.progress = item.progress;
      this.state = item.state;
   }

   /**
   * Starts the download.
   * @returns A promise with the updated download.
   */
   public async start(): Promise<Download> {
      return new Download(await invoke('plugin:download|start', { key: this.key }));
   }

   /**
   * Cancels the download.
   * @returns A promise with the updated download.
   */
   public async cancel(): Promise<Download> {
      return new Download(await invoke('plugin:download|cancel', { key: this.key }));
   }

   /**
   * Pauses the download.
   * @returns A promise with the updated download.
   */
   public async pause(): Promise<Download> {
      return new Download(await invoke('plugin:download|pause', { key: this.key }));
   }

   /**
   * Resumes the download.
   * @returns A promise with the updated download.
   */
   public async resume(): Promise<Download> {
      return new Download(await invoke('plugin:download|resume', { key: this.key }));
   }

   /**
   * Listen for changes to the download.
   * To avoid memory leaks, the `unlisten` function returned by the promise
   * should be called when no longer required.
   * @param onChanged - Callback function invoked when the download has changed.
   * @returns A promise with a function to remove the download listener.
   *
   * @example
   * ```ts
   * const unlisten = await download.listen((updatedDownload) => {
   *   console.log('Download:', updatedDownload);
   * });
   *
   * // To stop listening
   * unlisten();
   * ```
   */
   public async listen(listener: (download: Download) => void): Promise<UnlistenFn> {
      return DownloadEventManager.shared.addListener(this.key, listener);
   }
}

/**
 * Represents a download item.
 */
export interface DownloadItem {
  key: string;
  url: string;
  path: string;
  progress: number;
  state: DownloadState;
}

/**
* Represents the state of a download operation.
*/
export enum DownloadState {
  UNKNOWN = 'unknown',
  CREATED = 'created',
  IN_PROGRESS = 'inProgress',
  PAUSED = 'paused',
  CANCELLED = 'cancelled',
  COMPLETED = 'completed'
}

/**
 * Creates a download operation.
 *
 * @param key - The key identifier.
 * @param url - The download URL for the resource.
 * @param path - The download path on the filesystem.
 * @returns - The download operation.
 */
export async function create(key: string, url: string, path: string): Promise<Download> {
   return new Download(await invoke<DownloadItem>('plugin:download|create', { key, url, path }));
}

/**
 * Gets all download operations.
 *
 * @returns - The list of download operations.
 */
export async function list(): Promise<Download[]> {
   return (await invoke<DownloadItem[]>('plugin:download|list'))
      .map((item) => { return new Download(item); });
}

/**
 * Gets a download operation.
 *
 * @param key - The key identifier.
 * @returns - The download operation.
 */
export async function get(key: string): Promise<Download> {
   return new Download(await invoke<DownloadItem>('plugin:download|get', { key }));
}
