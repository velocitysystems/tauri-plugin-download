# Tauri Plugin Download

[![CI][ci-badge]][ci-url]

State-driven, resumable download API for Tauri 2.x apps.

This plugin provides a cross-platform download interface with resumable downloads,
byte-count progress tracking, and proper resource management.

[ci-badge]: https://github.com/silvermine/tauri-plugin-download/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/silvermine/tauri-plugin-download/actions/workflows/ci.yml

## Features

   * Parallel, resumable download support
   * Persistable, thread-safe store
   * State, byte count, and progress notifications
   * Cross-platform support (Linux, Windows, macOS, Android, iOS)

| Platform  | Supported |
| --------- | --------- |
| Linux     | ✓         |
| Windows   | ✓         |
| macOS     | ✓         |
| Android¹  | ✓         |
| iOS²      | ✓         |

¹ Uses [WorkManager][workmanager] with foreground service notifications for
reliable background downloads with resumable support via HTTP `Range` headers.
See [Android Support](#android-support) for details.

² Supports fully interruptible and resumable background downloads, even when
the app is suspended or terminated using
[`URLSession`](https://developer.apple.com/documentation/foundation/urlsession)
with a background configuration. See
[iOS Support](#ios-support) for details.

## Getting Started

### Installation

1. Install NPM dependencies:

   ```bash
   npm install
   ```

2. Build the TypeScript bindings:

   ```bash
   npm run build
   ```

3. Build the Rust plugin:

   ```bash
   cargo build
   ```

### Tests

Run Rust tests:

```bash
cargo test
```

Run Swift tests (iOS download manager library):

```bash
swift test --package-path ios/DownloadManagerKit
```

Run Kotlin tests (Android download manager library):

```bash
cd android && ./gradlew :lib:test
```

## Install

_This plugin requires a Rust version of at least **1.94.0**_

### Rust

Add the plugin to your `Cargo.toml`:

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-download = { git = "https://github.com/silvermine/tauri-plugin-download" }
```

### JavaScript/TypeScript

Install the JavaScript bindings:

```sh
npm install @silvermine/tauri-plugin-download
```

## Usage

### Prerequisites

Initialize the plugin in your `tauri::Builder`:

```rust
fn main() {
   tauri::Builder::default()
      .plugin(tauri_plugin_download::init())
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
}
```

### API

#### List downloads

```ts
import { list } from 'tauri-plugin-download';

async function listDownloads() {
   const downloads = await list();

   for (const download of downloads) {
      const totalBytes = download.totalBytes ?? 'unknown';

      console.debug(
         `Found '${download.path}': [${download.status}, ${download.receivedBytes}/${totalBytes} bytes, ${download.progress}%]`
      );
   }
}
```

#### Get a download

```ts
import { get, DownloadStatus } from 'tauri-plugin-download';

async function getDownload() {
   const download = await get('/path/to/file.zip');

   if (download.status === DownloadStatus.Pending) {
      console.debug(`Download '${download.path}' not found in store`);
   } else {
      const totalBytes = download.totalBytes ?? 'unknown';

      console.debug(
         `Found '${download.path}': [${download.status}, ${download.receivedBytes}/${totalBytes} bytes, ${download.progress}%]`
      );
   }
}
```

#### Create, start, pause, resume or cancel a download

The API uses discriminated unions with type guards for compile-time safety.
Only valid methods are available based on the download's status.

```ts
import { get, DownloadStatus, hasAction, DownloadAction } from 'tauri-plugin-download';

async function createAndStartDownload() {
   const download = await get('/path/to/file.zip');

   if (download.status === DownloadStatus.Pending) {
      // Download not in store - create it first
      const { download: created } = await download.create('https://example.com/file.zip');
      await created.start();
   }
}

async function manageDownload() {
   const download = await get('/path/to/file.zip');

   if (hasAction(download, DownloadAction.Start)) {
      await download.start(); // TypeScript knows start() is available
   } else if (hasAction(download, DownloadAction.Pause)) {
      await download.pause(); // TypeScript knows pause() is available
   } else if (hasAction(download, DownloadAction.Resume)) {
      await download.resume(); // TypeScript knows resume() is available
   }
}
```

On desktop, a download can be restricted to unmetered, unconstrained networks when it
is created:

```ts
const download = await get('/path/to/large-file.zip');

if (download.status === DownloadStatus.Pending) {
   const { download: created } = await download.create(
      'https://example.com/large-file.zip',
      { allowMetered: false }
   );

   await created.start();
}
```

`allowMetered` defaults to `true`. When it is `false`, both `start()` and `resume()`
reject if there is no active connection, connectivity cannot be determined, or the
current connection is reported as metered or constrained. The stored download remains
idle or paused so the action can be retried later. A connection change does not stop a
download that is already in progress. Android and iOS currently accept this option but
do not enforce it.

The network policy is fixed when the download is first created. Every download state
exposes its resolved policy through `download.options.allowMetered`. Calling `create()`
again for an existing path returns the existing record without changing its URL or
options.

#### Listen for progress notifications

Listeners can be attached to downloads in any status, including `Pending`.
This allows you to set up listeners before creating the download.
Each download state includes `receivedBytes`, `totalBytes`, and `progress`.
When the server does not provide a content length, `totalBytes` is `null`;
`progress` remains `0` until the terminal `Completed` event, where it is `100`.

```ts
import { get, DownloadStatus } from 'tauri-plugin-download';

async function setupAndStartDownload() {
   const download = await get('/path/to/file.zip');

   // Attach listener (works for Pending downloads too)
   const unlisten = await download.listen((updated) => {
      console.debug(
         `'${updated.path}': ${updated.receivedBytes}/${updated.totalBytes ?? 'unknown'} bytes (${updated.progress}%)`
      );
   });

   // Create and start if pending
   if (download.status === DownloadStatus.Pending) {
      const { download: created } = await download.create('https://example.com/file.zip');
      await created.start();
   }

   // To stop listening
   unlisten();
}
```

Alternatively, pass `{ autoUnlisten: true }` to automatically remove the listener
when the download reaches a terminal state (`Completed` or `Canceled`):

```ts
await download.listen((updated) => {
   console.debug(`'${updated.path}': ${updated.progress}%`);
}, { autoUnlisten: true });
```

### Examples

Check out the [examples/tauri-app](examples/tauri-app) directory for a working example of
how to use this plugin.

### Testing with mocks

For unit tests, this package publishes `@silvermine/tauri-plugin-download/mocks` so you
can keep using the real JavaScript API while mocking the Tauri backend.

This helper is designed for tests that need to:

   * Seed one or more downloads before the test runs
   * Exercise `get()`, `list()`, and download actions without a real Tauri app
   * Emit download change events for listener tests
   * Inject command errors for failure scenarios

The mock helper approximates backend/native state transitions for common test flows.
It is not a backend contract and does not transition downloads to `Completed`.
Use `emitChange()` to simulate progress updates or terminal-state events, or
`setDownload()` to seed a specific state without emitting an event.
It only simulates the desktop event path and returns `false` for `is_native`,
so tests for the native/mobile listener branch need a separate approach.

`createMockDownloadState()` computes `progress` from `receivedBytes` and
`totalBytes` when `progress` is not explicitly provided. For unknown-size
downloads, use `totalBytes: null`. A generated `Completed` state reports
`progress: 100`.

```ts
import { afterEach, expect, it } from 'vitest';
import {
   DownloadAction,
   DownloadStatus,
   get,
   hasAction,
} from '@silvermine/tauri-plugin-download';
import {
   clearDownloadMocks,
   createMockDownloadState,
   mockDownloadPlugin,
} from '@silvermine/tauri-plugin-download/mocks';

afterEach(() => {
   clearDownloadMocks();
});

it('starts a mocked download', async () => {
   mockDownloadPlugin({
      downloads: [
         createMockDownloadState(DownloadStatus.Idle, {
            path: '/tmp/file.zip',
         }),
      ],
   });

   const download = await get('/tmp/file.zip');

   if (hasAction(download, DownloadAction.Start)) {
      const response = await download.start();

      expect(response.download.status).toBe(DownloadStatus.InProgress);
   }
});
```

The mock helper currently simulates the desktop event flow and returns `false` for
`is_native`.

## Development Standards

This project follows the
[Silvermine standardization](https://github.com/silvermine/standardization)
guidelines. Key standards include:

   * **EditorConfig**: Consistent editor settings across the team
   * **Markdownlint**: Markdown linting for documentation
   * **Commitlint**: Conventional commit message format
   * **Code Style**: 3-space indentation, LF line endings

### Running Standards Checks

```bash
npm run standards
```

## Android Support

On Android, this plugin uses a pure Kotlin download manager library (`:lib` module)
backed by [WorkManager][workmanager] with `CoroutineWorker` for reliable background
execution.

[workmanager]: https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started

### How It Works

1. **App Running**: Downloads run as foreground services with notifications,
   with real-time progress updates
2. **App Backgrounded**: `WorkManager` ensures downloads continue reliably
3. **Resumable**: Supports HTTP `Range` headers for resuming interrupted
   downloads
4. **App Resumed**: The plugin reconciles state and emits completion events

### Project Structure

The `android/` directory is a 3-module Gradle build:

   * **Root module** (`:`): The Tauri plugin bridge (`DownloadPlugin.kt`), depends on
     `:lib` and `:tauri-android`
   * **`:lib` module**: Pure download manager library (`org.silvermine.downloadmanager`),
     no Tauri dependencies, independently buildable and testable
   * **`:example` module**: Standalone example app (Compose UI), depends only on `:lib`

### Running the Example App

Open the `android/` directory in Android Studio, select the `:example` run configuration,
and run on an emulator or device.

## iOS Support

On iOS, this plugin uses `URLSession` with a background configuration, which allows
downloads
to continue even when the app is suspended or terminated by the system.

### How It Works

1. **App Running**: Downloads proceed normally with real-time progress updates
2. **App Suspended**: iOS continues downloads in the background
3. **App Terminated**: iOS completes downloads and relaunches the app in the background
   to deliver results
4. **App Resumed**: The plugin reconciles state and emits completion events

### Running the Example App

Open `ios/DownloadManagerExample/DownloadManagerExample.xcodeproj` in Xcode,
select a simulator or device, and run.

### Tauri Apps

Background downloads work automatically in Tauri apps. When the app resumes, all delegate
callbacks are delivered and state is properly reconciled.

**Note**: Tauri's iOS architecture doesn't currently expose the `AppDelegate` hook for
`handleEventsForBackgroundURLSession`. Without calling this completion handler, iOS cannot
determine when background event processing is complete. This may cause iOS to:

   * Keep the app running longer than necessary (wasting battery)
   * Skip taking a UI snapshot for the app switcher
   * Deprioritize future background execution for this app

In practice, this has minimal impact for typical download scenarios since iOS delivers
all pending delegate callbacks when the app resumes regardless of whether the completion
handler is called.

### Future Integration

If Tauri exposes `AppDelegate` hooks in the future, add this for optimal background
handling:

```swift
import DownloadManagerKit

func application(_ application: UIApplication,
                 handleEventsForBackgroundURLSession identifier: String,
                 completionHandler: @escaping () -> Void) {
   DownloadManager.shared.setBackgroundCompletionHandler(completionHandler)
}
```

## License

MIT

## Contributing

Contributions are welcome! Please follow the established coding standards and commit
message conventions.
