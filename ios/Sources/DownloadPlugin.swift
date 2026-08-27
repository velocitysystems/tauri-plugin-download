import DownloadManagerKit
import SwiftRs
import Tauri
import WebKit

class PathArgs: Decodable {
   let path: String
}

/// Settings pushed by the Rust plugin at startup.
///
/// Named for the payload rather than the setting so a second builder option is a new
/// field here, not a second command.
class ConfigArgs: Decodable {
   let userAgent: String
}

class CreateArgs: Decodable {
   let path: String
   let url: String
   let options: CreateOptions
}

class DownloadPlugin: Plugin {
   let downloadManager = DownloadManager.shared

   override init()
   {
      super.init()
      Task {
          for await download in DownloadManager.shared.changed {
             try? self.trigger("changed", data: download);
#if DEBUG
             Logger.debug("[\(download.path.lastPathComponent)] \(download.status) - \(String(format: "%.0f", download.progress))% (\(download.receivedBytes)/\(download.totalBytes.map { String($0) } ?? "unknown") bytes)")
#endif
          }
      }
   }

   /// Applies the settings from the Rust plugin's builder.
   ///
   /// Invoked by the Rust plugin during setup rather than from the webview: Tauri
   /// fills the config it hands to `load(webview:)` from `tauri.conf.json`, so a
   /// value set on the Rust builder can only arrive as a command.
   @objc public func configure(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(ConfigArgs.self)
      Task {
         await self.downloadManager.setUserAgent(args.userAgent)
         // No response payload anchors this call inside the Task the way every other
         // handler's does, so hoisting it out would still compile — and would let
         // Rust's blocking `run_mobile_plugin` return before the actor write lands.
         invoke.resolve()
      }
   }

   @objc public func list(_ invoke: Invoke) {
      Task {
         let response = await self.downloadManager.list()
         invoke.resolve(response)
      }
   }

   @objc public func get(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(PathArgs.self)
      let path = try parsePath(args.path)
      Task {
         let response = await self.downloadManager.get(path: path)
         invoke.resolve(response)
      }
   }
   
   @objc public func create(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(CreateArgs.self)
      let path = try parsePath(args.path)
      let url = try parseURL(args.url)
      Task {
         let response = await self.downloadManager.create(
            path: path,
            url: url,
            options: args.options
         )
         invoke.resolve(response)
      }
   }
   
   @objc public func start(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(PathArgs.self)
      let path = try parsePath(args.path)
      Task {
         do {
            let response = try await self.downloadManager.start(path: path)
            invoke.resolve(response)
         } catch {
            invoke.reject(error.localizedDescription)
         }
      }
   }
   
   @objc public func cancel(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(PathArgs.self)
      let path = try parsePath(args.path)
      Task {
         do {
            let response = try await self.downloadManager.cancel(path: path)
            invoke.resolve(response)
         } catch {
            invoke.reject(error.localizedDescription)
         }
      }
   }
   
   @objc public func pause(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(PathArgs.self)
      let path = try parsePath(args.path)
      Task {
         do {
            let response = try await self.downloadManager.pause(path: path)
            invoke.resolve(response)
         } catch {
            invoke.reject(error.localizedDescription)
         }
      }
   }
   
   @objc public func resume(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(PathArgs.self)
      let path = try parsePath(args.path)
      Task {
         do {
            let response = try await self.downloadManager.resume(path: path)
            invoke.resolve(response)
         } catch {
            invoke.reject(error.localizedDescription)
         }
      }
   }
}

@_cdecl("init_plugin_download")
func initPlugin() -> Plugin {
   return DownloadPlugin()
}
