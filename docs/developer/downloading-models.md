# Downloading Models In-App

Forge SDK works with GGUF models downloaded at runtime. This guide shows how to fetch models directly from Hugging Face.

---

## Why Use In-App Downloading?

- **Reduce App Size**: Keep your initial app download small and fetch the AI model only when needed.
- **Dynamic Updates**: Switch between different models without releasing a new app version.
- **User Choice**: Let users pick between "Small/Fast" or "Large/Quality" models.

---

## Using URLSession

For most use cases, `URLSession.downloadTask` is sufficient:

```swift
import Foundation

func downloadModel(from url: URL, to destination: URL, progress: @escaping (Double) -> Void, completion: @escaping (Result<URL, Error>) -> Void) {

    class DownloadDelegate: NSObject, URLSessionDownloadDelegate {
        let destination: URL
        let progress: (Double) -> Void
        let completion: (Result<URL, Error>) -> Void

        init(destination: URL, progress: @escaping (Double) -> Void, completion: @escaping (Result<URL, Error>) -> Void) {
            self.destination = destination
            self.progress = progress
            self.completion = completion
        }

        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
            if totalBytesExpectedToWrite > 0 {
                progress(Double(totalBytesWritten) / Double(totalBytesExpectedToWrite))
            }
        }

        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
            do {
                try? FileManager.default.removeItem(at: destination)
                try FileManager.default.moveItem(at: location, to: destination)
                completion(.success(destination))
            } catch {
                completion(.failure(error))
            }
        }

        func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
            if let error = error {
                completion(.failure(error))
            }
        }
    }

    let delegate = DownloadDelegate(destination: destination, progress: progress, completion: completion)
    let session = URLSession(configuration: .default, delegate: delegate, delegateQueue: nil)
    let task = session.downloadTask(with: url)
    task.resume()
}

// Usage
let modelURL = URL(string: "https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF/resolve/main/LFM2.5-VL-1.6B-Q8_0.gguf")!
let documentsURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
let destinationURL = documentsURL.appendingPathComponent("model.gguf")

downloadModel(from: modelURL, to: destinationURL, progress: { p in
    print("Progress: \(Int(p * 100))%")
}) { result in
    switch result {
    case .success(let url):
        print("Downloaded to: \(url.path)")
    case .failure(let error):
        print("Error: \(error)")
    }
}
```

---

## SwiftUI Progress View

```swift
import SwiftUI

struct ModelDownloadView: View {
    @State private var progress: Double = 0
    @State private var isDownloading = false
    @State private var status = "Ready"

    var body: some View {
        VStack(spacing: 20) {
            Text(status)

            if isDownloading {
                ProgressView(value: progress, total: 1.0) {
                    Text("Downloading...")
                } currentValueLabel: {
                    Text("\(Int(progress * 100))%")
                }
                .progressViewStyle(.linear)
                .padding()
            }

            Button("Download Model") {
                startDownload()
            }
            .disabled(isDownloading)
        }
        .padding()
    }

    func startDownload() {
        isDownloading = true
        status = "Downloading..."

        let url = URL(string: "https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF/resolve/main/LFM2.5-VL-1.6B-Q8_0.gguf")!
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let dest = docs.appendingPathComponent("model.gguf")

        downloadModel(from: url, to: dest, progress: { p in
            DispatchQueue.main.async {
                progress = p
            }
        }) { result in
            DispatchQueue.main.async {
                isDownloading = false
                switch result {
                case .success:
                    status = "Download complete!"
                case .failure(let error):
                    status = "Error: \(error.localizedDescription)"
                }
            }
        }
    }
}
```

---

## Hugging Face URL Format

For Hugging Face, use the **resolve** URL format:

```
https://huggingface.co/{org}/{repo}/resolve/main/{filename}
```

### Example URLs

| Model | URL |
|-------|-----|
| LFM2.5-VL 1.6B | `https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF/resolve/main/LFM2.5-VL-1.6B-Q8_0.gguf` |
| LFM2.5-VL CLIP | `https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF/resolve/main/mmproj-LFM2.5-VL-1.6b-Q8_0.gguf` |
| LFM2 2.6B | `https://huggingface.co/LiquidAI/LFM2-2.6B-Exp-GGUF/resolve/main/LFM2-2.6B-Exp-Q4_K_M.gguf` |
| Qwen2.5 3B | `https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf` |

---

## Best Practices

1. **Use Resolve URLs**: Ensure your URL contains `/resolve/main/` for direct file download.

2. **Check Disk Space**: Models are large (1GB–10GB). Check available space before downloading:
   ```swift
   let freeSpace = try? FileManager.default.attributesOfFileSystem(
       forPath: NSHomeDirectory()
   )[.systemFreeSize] as? Int64
   ```

3. **Show File Size**: Get the expected size from the Content-Length header:
   ```swift
   var request = URLRequest(url: modelURL)
   request.httpMethod = "HEAD"
   let (_, response) = try await URLSession.shared.data(for: request)
   let size = (response as? HTTPURLResponse)?.expectedContentLength ?? 0
   ```

4. **Resume Downloads**: For large models, implement resume capability using `Range` headers.

5. **Validate Downloads**: Check file integrity after download (file size at minimum).

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
