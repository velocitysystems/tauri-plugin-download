//
//  DownloadsView.swift
//  DownloadManagerExample
//

import Combine
import SwiftUI
import DownloadManagerKit

struct DownloadsView: View {
   @StateObject private var manager = DownloadManager.shared
   @State private var downloadUrl: String = ""

   init() {
      Task {
         for await download in DownloadManager.shared.changed {
            print("[\(download.key)] \(download.state) - \(String(format: "%.0f", download.progress))%")
         }
      }
   }

   var body: some View {
      NavigationView {
         VStack {
            HStack {
               TextField("Enter URL to download", text: $downloadUrl)
                  .textFieldStyle(RoundedBorderTextFieldStyle())
                  .autocapitalization(.none)
                  .disableAutocorrection(true)
                  .keyboardType(.URL)
               
               Button(action: createDownload) {
                  Text("Create")
                     .padding(.horizontal, 10)
                     .padding(.vertical, 5)
                     .background(Color.blue)
                     .foregroundColor(.white)
                     .cornerRadius(8)
               }
               .disabled(downloadUrl.isEmpty)
            }
            .padding()

            List {
               ForEach(manager.downloads) { item in
                  VStack(alignment: .leading) {
                     Text(item.url.lastPathComponent)
                        .font(.headline)
                     ProgressView(value: item.progress / 100)
                        .progressViewStyle(LinearProgressViewStyle())
                     switch item.state {
                     case .created:
                        HStack(spacing: 8) {
                           Button(action: { _ = try? manager.start(key: item.key) }) {
                              Text("Start")
                                 .padding(8)
                                 .background(Color.blue.opacity(0.2))
                                 .cornerRadius(8)
                           }.buttonStyle(PlainButtonStyle())
                           Button(action: { _ = try? manager.cancel(key: item.key) }) {
                              Text("Cancel")
                                 .padding(8)
                                 .background(Color.red.opacity(0.2))
                                 .cornerRadius(8)
                           }.buttonStyle(PlainButtonStyle())
                        }
                     case .inProgress:
                        HStack(spacing: 8) {
                           Button(action: { _ = try? manager.pause(key: item.key) }) {
                              Text("Pause")
                                 .padding(8)
                                 .background(Color.blue.opacity(0.2))
                                 .cornerRadius(8)
                           }.buttonStyle(PlainButtonStyle())
                           Button(action: { _ = try? manager.cancel(key: item.key) }) {
                              Text("Cancel")
                                 .padding(8)
                                 .background(Color.red.opacity(0.2))
                                 .cornerRadius(8)
                           }.buttonStyle(PlainButtonStyle())
                        }
                     case .paused:
                        HStack(spacing: 8) {
                           Button(action: { _ = try? manager.resume(key: item.key) }) {
                              Text("Resume")
                                 .padding(8)
                                 .background(Color.blue.opacity(0.2))
                                 .cornerRadius(8)
                           }.buttonStyle(PlainButtonStyle())
                           Button(action: { _ = try? manager.cancel(key: item.key) }) {
                              Text("Cancel")
                                 .padding(8)
                                 .background(Color.red.opacity(0.2))
                                 .cornerRadius(8)
                           }.buttonStyle(PlainButtonStyle())
                        }
                     default:
                        EmptyView()
                     }
                  }
                  .padding(.vertical, 4)
               }
            }
         }
         .navigationTitle("Downloads")
      }
   }

   private func createDownload() {
      guard !downloadUrl.isEmpty else {
         return
      }

      guard let url = URL(string: downloadUrl), url.scheme != nil && url.host != nil else {
         let alertController = UIAlertController(
            title: "Invalid URL",
            message: "The URL you entered is not valid.",
            preferredStyle: .alert
         )
         alertController.addAction(UIAlertAction(title: "OK", style: .default))

         if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
            let rootViewController = windowScene.windows.first?.rootViewController {
            rootViewController.present(alertController, animated: true)
         }
         return
      }

      let filename = url.lastPathComponent
      let path = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent(filename)
      _ = try? manager.create(
         key: filename,
         url: url,
         path: path)

      downloadUrl = ""
   }
}
