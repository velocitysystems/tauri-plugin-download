import { invoke } from '@tauri-apps/api/core'

/**
 * Creates a download operation.
 *
 * @param key - The key identifier.
 * @param url - The download URL  for the resource.
 * @param path - The download path on the filesystem.
 * @returns - The download operation.
 */
export async function create(key: string, url: string, path: string): Promise<Download> {
  return await invoke<Download>('plugin:download|create', {
    key, url, path,
  });
}

/**
 * Gets all download operations.
 *
 * @returns - The list of download operations.
 */
export async function list(): Promise<Download[]> {
  return await invoke<Download[]>('plugin:download|list');
}

/**
 * Gets a download operation.
 *
 * @param key - The key identifier.
 * @returns - The download operation.
 */
export async function get(key: string): Promise<Download> {
  return await invoke<Download>('plugin:download|get', {
    key,
  });
}

/**
 * Starts a download operation.
 *
 * @param key - The key identifier.
 * @returns - The download operation.
 */
export async function start(key: string): Promise<Download> {
  return await invoke<Download>('plugin:download|start', {
    key,
  });
}

/**
 * Cancels a download operation.
 *
 * @param key - The key identifier.
 * @returns - The download operation.
 */
export async function cancel(key: string): Promise<Download> {
  return await invoke<Download>('plugin:download|cancel', {
    key,
  });
}

/**
 * Pauses a download operation.
 *
 * @param key - The key identifier.
 * @returns - The download operation.
 */
export async function pause(key: string): Promise<Download> {
  return await invoke<Download>('plugin:download|pause', {
    key,
  });
}

/**
 * Resumes a download operation.
 *
 * @param key - The key identifier.
 * @returns - The download operation.
 */
export async function resume(key: string): Promise<Download> {
  return await invoke<Download>('plugin:download|resume', {
    key,
  });
}

/**
 * Represents a download operation.
 */
export interface Download {
  key: string;
  url: string;
  path: string;
  progress: number;
  state: DownloadState;
}

/**
 * Represents a download event payload.
 */
export interface DownloadEvent {
  key: string;
  progress?: number;
}

/**
* Represents the state of a download operation.
*/
export enum DownloadState {
  UNKNOWN = 'UNKNOWN',
  CREATED = 'CREATED',
  IN_PROGRESS = 'IN_PROGRESS',
  PAUSED = 'PAUSED',
  CANCELLED = 'CANCELLED'
}
