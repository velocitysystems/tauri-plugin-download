# tauri-plugin-download

State-driven, resumable download API for Tauri 2.x apps.

   * Parallel, resumable download support
   * Persistable, thread-safe store
   * State and progress notifications
   * Cross-platform support (Linux, Windows, macOS, Android, iOS)

| Platform | Supported |
| -------- | --------- |
| Linux    | ✓         |
| Windows  | ✓         |
| macOS    | ✓         |
| Android  | ✓         |
| iOS¹     | ✓         |

¹ Supports fully interruptible and resumable background downloads, even when the app
is suspended or terminated using
[`URLSession`](https://developer.apple.com/documentation/foundation/urlsession) with a
background configuration.

## Installation

Note: These steps are an interim workaround until the plugin is published.

### Rust

Add the `tauri-plugin-download` crate to your `Cargo.toml`:

```toml
[dependencies]
tauri-plugin-download = { git = "https://github.com/silvermine/tauri-plugin-download.git" }
```

### TypeScript

Install the TypeScript bindings via npm:

```bash
npm install github:@silvermine/tauri-plugin-download
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

#### Create a download

```ts
import { create } from 'tauri-plugin-download';

async function createDownload() {
   const key = 'file.zip',
         url = 'https://example.com/file.zip',
         path = await join(await appDataDir(), 'downloads', key);

   const download = await create(key, url, path);

   console.debug(`Created '${download.key}':${download.url}`);
}
```

#### List downloads

```ts
import { list } from 'tauri-plugin-download';

async function listDownloads() {
   const downloads = await list();

   for (let download of downloads) {
      console.debug(`Found '${download.key}':${download.url} [${download.state}, ${download.progress}%]`)
   }
}
```

#### Get a download

```ts
import { get } from 'tauri-plugin-download';

async function getDownload() {
   const download = await get('file.zip');

   console.debug(`Found '${download.key}':${download.url} [${download.state}, ${download.progress}%]`)
}
```

#### Start, pause, resume or cancel a download

```ts
import { get } from 'tauri-plugin-download';

async function getDownloadAndUpdate() {
   const download = await get('file.zip');

   download.start();
   download.pause();
   download.resume();
   download.cancel();
}
```

#### Listen for progress notifications

```ts
import { get } from 'tauri-plugin-download';

async function getDownloadAndListen() {
   const download = await get('file.zip');

   const unlisten = await download.listen((updatedDownload) => {
      console.debug(`'${updatedDownload.key}':${updatedDownload.progress}%`);
   });

   // To stop listening
   unlisten();
}
```

### Examples

Check out the [examples/tauri-app](examples/tauri-app) directory for a working example of
how to use this plugin.

## How do I contribute?

We genuinely appreciate external contributions. See [our extensive
documentation](https://github.com/silvermine/silvermine-info#contributing) on how to
contribute.

## License

This software is released under the MIT license. See [the license file](LICENSE) for more
details.
