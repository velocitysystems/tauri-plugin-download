import Combine
import DownloadManagerKit
import SwiftRs
import Tauri
import WebKit

class CreateRequestArgs: Decodable {
   let key: String
   let url: String
   let path: String
}

class GetRequestArgs: Decodable {
   let key: String
}

class DownloadPlugin: Plugin {
   let downloadManager = DownloadManager.shared
   private var cancellables = Set<AnyCancellable>()
   
   override init()
   {
      super.init()
      downloadManager.downloadItemChanged
         .sink { download in
            Logger.debug("[\(download.key)] \(download.state) - \(String(format: "%.0f", download.progress))%")
         }
         .store(in: &cancellables)
   }
   
   @objc public func create(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(CreateRequestArgs.self)
      let response = try downloadManager.create(key: args.key, url: URL(string: args.url)!, path: URL(string: args.path)!)
      invoke.resolve(response)
   }
   
   @objc public func get(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(GetRequestArgs.self)
      let response = try downloadManager.get(key: args.key)
      invoke.resolve(response)
   }
   
   @objc public func list(_ invoke: Invoke) throws {
      let response = downloadManager.list()
      invoke.resolve(response)
   }
   
   @objc public func start(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(GetRequestArgs.self)
      let response = try downloadManager.start(key: args.key)
      invoke.resolve(response)
   }
   
   @objc public func cancel(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(GetRequestArgs.self)
      let response = try downloadManager.cancel(key: args.key)
      invoke.resolve(response)
   }
   
   @objc public func pause(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(GetRequestArgs.self)
      let response = try downloadManager.pause(key: args.key)
      invoke.resolve(response)
   }
   
   @objc public func resume(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(GetRequestArgs.self)
      let response = try downloadManager.resume(key: args.key)
      invoke.resolve(response)
   }
}

@_cdecl("init_plugin_download")
func initPlugin() -> Plugin {
   return DownloadPlugin()
}
