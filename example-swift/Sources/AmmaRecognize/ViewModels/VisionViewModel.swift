import SwiftUI
import ForgeSwift
import Combine
import CryptoKit
import Foundation
import PDFKit
import Darwin
import Metal

@MainActor
final class VisionViewModel: ObservableObject {
    @Published var isLoading = false
    @Published var loadingStatus: String = "Loading..."
    @Published var loadingProgress: Double = 0.0 // 0.0 to 1.0
    @Published var downloadSpeed: Double = 0.0 // bytes per second
    @Published var loadingStage: LoadingStage = .preparing
    @Published var messages: [ChatMessageUI] = []
    @Published var currentResponse: String = ""
    @Published var isGenerating = false
    @Published var selectedImage: UIImage?
    @Published var pendingImages: [UIImage] = [] // Images waiting to be sent
    @Published var pendingFile: PendingFile? = nil // Single file waiting to be sent (only one file allowed)
    @Published var transcript: String = ""
    @Published var isRecording: Bool = false

    // Chat History Management
    @Published var sessions: [ChatSession] = []
    @Published var currentSessionId: UUID?
    @Published var searchText: String = ""

    // ForgeSwift Engine
    private var engine: ForgeEngine?
    private var visionEngine: ForgeEngine?
    private var generationTask: Task<Void, Never>?
    private var speechRecognizer = SpeechRecognizer()
    private var cancellables = Set<AnyCancellable>()

    // Conversation state
    private var conversationHistory: [ConversationMessage] = []

#if DEBUG
    /// Lightweight memory logger (phys_footprint + resident) for on-device profiling
    @inline(__always) private func logMemory(_ tag: String) {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout.size(ofValue: info) / MemoryLayout<natural_t>.size)
        let kr: kern_return_t = withUnsafeMutablePointer(to: &info) { ptr in
            ptr.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        if kr == KERN_SUCCESS {
            let footprintMB = Double(info.phys_footprint) / 1024.0 / 1024.0
            let residentMB = Double(info.resident_size) / 1024.0 / 1024.0
            print(String(format: "MEM[%@]: footprint=%.1f MB, resident=%.1f MB", tag, footprintMB, residentMB))
        } else {
            print("MEM[\(tag)]: task_info error \(kr)")
        }
    }
#else
    private func logMemory(_ tag: String) {}
#endif

    private struct ModelFile {
        let repository: String
        let revision: String
        let name: String
        let size: Int64
        let sha256: String

        var downloadURL: String {
            "https://huggingface.co/LiquidAI/\(repository)/resolve/\(revision)/\(name)?download=true"
        }
    }

    private static let mobileModel = ModelFile(
        repository: "LFM2.5-VL-450M-GGUF",
        revision: "6f15859c2de1583b6180a9bc56338342592b589a",
        name: "LFM2.5-VL-450M-Q4_K_M.gguf",
        size: 229_313_568,
        sha256: "1093f1331319199bbcacdbd7ecc9aa6e5678db6b55073948d0f508be90c8ab68"
    )
    private static let mobileProjector = ModelFile(
        repository: "LFM2.5-VL-450M-GGUF",
        revision: "6f15859c2de1583b6180a9bc56338342592b589a",
        name: "mmproj-LFM2.5-VL-450m-Q8_0.gguf",
        size: 102_815_168,
        sha256: "ebfc428baa37efad8bae93864f914b2634a09009f91ad59f974fe1a1565d8561"
    )
    private static let desktopModel = ModelFile(
        repository: "LFM2.5-8B-A1B-GGUF",
        revision: "dfd5fdcad7a1c0d31473fb4ca443b8befbacddf0",
        name: "LFM2.5-8B-A1B-Q4_K_M.gguf",
        size: 5_155_564_768,
        sha256: "4923ec14f06b968b74d663e5949867d2d9c3bf13a20b8be1a9f9af39989b2bb0"
    )

    private var isRunningOnComputer: Bool {
        #if os(macOS)
        return true
        #elseif os(iOS)
        return ProcessInfo.processInfo.isiOSAppOnMac
        #else
        return false
        #endif
    }

    private var canUseDesktopModel: Bool {
        isRunningOnComputer && ProcessInfo.processInfo.physicalMemory >= 12 * 1024 * 1024 * 1024
    }
    private var downloadStartTime: Date?
    private var lastProgressUpdate: Date?

    // Model file paths (need to be set or downloaded)
    private var modelPath: String?
    private var clipPath: String?

    // Public property to check if model is loaded
    var isModelLoaded: Bool {
        engine != nil
    }

    init() {
        loadSessions()

        // Check if model needs to be loaded - if so, set loading state immediately
        checkModelStatus()

        speechRecognizer.$transcript
            .receive(on: RunLoop.main)
            .sink { [weak self] transcript in
                self?.transcript = transcript
            }
            .store(in: &cancellables)

        speechRecognizer.$isRecording
            .receive(on: RunLoop.main)
            .sink { [weak self] isRecording in
                self?.isRecording = isRecording
            }
            .store(in: &cancellables)
    }

    // MARK: - Model Status Check

    private func checkModelStatus() {
        // Always set loading state immediately if model is not loaded
        // This prevents black screen on first launch
        // The actual model check will happen in loadModel()
        if engine == nil {
            // Set loading state immediately to show loading overlay
            // This ensures UI is visible from the start
            isLoading = true
            loadingStatus = "Preparing..."
            loadingStage = .preparing
            loadingProgress = 0.0
        }
    }

    // MARK: - Chat Session Management

    private func loadSessions() {
        let legacyKey = "amma_rust_chat_sessions"
        let data = try? Data(contentsOf: sessionsFileURL)
        let legacyData = UserDefaults.standard.data(forKey: legacyKey)
        if let data = data ?? legacyData,
           let decoded = try? JSONDecoder().decode([ChatSession].self, from: data) {
            self.sessions = decoded.map { session in
                var migrated = session
                migrated.history = session.history.map { message in
                    guard message.role == .assistant,
                          message.displayText.contains("Show me a plant or animal") else {
                        return message
                    }
                    return ConversationMessage(
                        role: .assistant,
                        fullText: greetingMessage,
                        displayText: greetingMessage,
                        imageFilenames: message.imageFilenames,
                        fileNames: message.fileNames
                    )
                }
                return migrated
            }
            saveSessions()
            UserDefaults.standard.removeObject(forKey: legacyKey)
        }
    }

    private func saveSessions() {
        if let encoded = try? JSONEncoder().encode(sessions) {
            try? encoded.write(to: sessionsFileURL, options: [.atomic, .completeFileProtection])
            protectPrivateFile(sessionsFileURL)
        }
    }

    func createNewChat() {
        guard engine != nil else { return }

        // Save current session if it has meaningful messages (not just the greeting)
        updateCurrentSession()

        // Remove current session if it's empty (just greeting, no user interaction)
        if let currentId = currentSessionId,
           let index = sessions.firstIndex(where: { $0.id == currentId }) {
            let session = sessions[index]
            // Check if session only has the initial greeting (no user messages)
            let hasUserMessages = session.history.contains { $0.role == .user }
            if !hasUserMessages && session.title == "New Chat" {
                sessions.remove(at: index)
            }
        }

        // Reset for new chat - clear messages to show welcome page
        let newSession = ChatSession(title: "New Chat")
        sessions.insert(newSession, at: 0)
        currentSessionId = newSession.id
        messages = [] // Empty messages to show welcome page
        selectedImage = nil // Clear any selected image
        pendingImages = [] // Clear pending images
        pendingFile = nil // Clear pending file
        conversationHistory = [] // Clear conversation history

        // Reset the engine context for new conversation
        resetEngines()

        saveSessions()
    }

    /// Personalized greeting message
    private var greetingMessage: String {
        let userName = UserDefaults.standard.string(forKey: "user_name") ?? ""
        let trimmedName = userName.trimmingCharacters(in: .whitespaces)

        if trimmedName.isEmpty {
            return "Hello! How can I help?"
        } else {
            return "Hello, \(trimmedName). How can I help?"
        }
    }

    func loadSession(_ session: ChatSession) {
        guard engine != nil else { return }

        // Save current session before switching
        updateCurrentSession()

        currentSessionId = session.id

        // Reconstruct messages from history
        self.messages = session.history.map { msg -> ChatMessageUI in
            let role = msg.role == .user ? ChatRole.user : ChatRole.assistant

            // Load images from disk
            let uiImages = msg.imageFilenames.compactMap { filename -> UIImage? in
                if let data = self.loadImageFromDisk(filename: filename) {
                    return UIImage(data: data)
                }
                return nil
            }

            return ChatMessageUI(
                role: role,
                text: msg.displayText,
                images: uiImages,
                fileNames: msg.fileNames
            )
        }

        // Restore conversation history
        self.conversationHistory = session.history

        // Reset engine and replay context
        resetEngines()

        // Update the session's position in the list (most recent first)
        if let index = sessions.firstIndex(where: { $0.id == session.id }) {
            let session = sessions.remove(at: index)
            sessions.insert(session, at: 0)
        }
    }

    private func updateCurrentSession() {
        guard let currentId = currentSessionId else { return }

        if let index = sessions.firstIndex(where: { $0.id == currentId }) {
            // Update title if it's still default or if we have a better title source
            if sessions[index].title == "New Chat" || sessions[index].title == "[Image attached]" {
                // Find first meaningful text message (skip image-only messages)
                if let firstTextMsg = messages.first(where: { msg in
                    msg.role == .user &&
                    msg.text != "[Image attached]" &&
                    !msg.text.trimmingCharacters(in: .whitespaces).isEmpty
                }) {
                    // Use the text message as title
                    let text = firstTextMsg.text.trimmingCharacters(in: .whitespaces)
                    sessions[index].title = String(text.prefix(30)) + (text.count > 30 ? "..." : "")
                } else if let firstUserMsg = messages.first(where: { $0.role == .user }),
                          firstUserMsg.text == "[Image attached]" {
                    // If first message is image, try to use AI response as title
                    if let firstAssistantMsg = messages.first(where: { $0.role == .assistant }),
                       !firstAssistantMsg.text.isEmpty {
                        let responseText = firstAssistantMsg.text.trimmingCharacters(in: .whitespaces)
                        // Extract first meaningful words (skip greetings)
                        let words = responseText.components(separatedBy: .whitespacesAndNewlines)
                        if words.count > 0 {
                            // Take first few words, but skip common greetings
                            let skipWords = ["Hello", "Hi", "I'm", "I", "am", "This", "is", "a", "an", "the"]
                            let meaningfulWords = words.filter { !skipWords.contains($0.capitalized) }
                            if !meaningfulWords.isEmpty {
                                let title = meaningfulWords.prefix(4).joined(separator: " ")
                                sessions[index].title = String(title.prefix(30)) + (title.count > 30 ? "..." : "")
                            } else {
                                sessions[index].title = "Image Recognition"
                            }
                        } else {
                            sessions[index].title = "Image Recognition"
                        }
                    } else {
                        // No AI response yet, use default
                        sessions[index].title = "Image Recognition"
                    }
                }
            }

            sessions[index].history = conversationHistory
            sessions[index].updatedAt = Date()
            saveSessions()
        }
    }

    var filteredSessions: [ChatSession] {
        // Filter out empty sessions (no user messages)
        let meaningfulSessions = sessions.filter { session in
            session.history.contains { $0.role == .user }
        }

        if searchText.isEmpty {
            return meaningfulSessions
        } else {
            // Search in both title AND message content
            return meaningfulSessions.filter { session in
                // Check title
                if session.title.localizedCaseInsensitiveContains(searchText) {
                    return true
                }

                // Check message content
                for message in session.history {
                    if message.fullText.localizedCaseInsensitiveContains(searchText) {
                        return true
                    }
                }

                return false
            }
        }
    }

    // MARK: - Session Management (Delete & Rename)

    func deleteSession(_ session: ChatSession) {
        sessions.removeAll { $0.id == session.id }

        // Clean up image files from disk
        for message in session.history {
            for filename in message.imageFilenames {
                deleteImageFromDisk(filename: filename)
            }
        }

        // If we deleted the current session, show welcome page (first page)
        if currentSessionId == session.id {
            // Create a new empty session for the welcome page
            let newSession = ChatSession(title: "New Chat")
            sessions.insert(newSession, at: 0)
            currentSessionId = newSession.id
            // Clear messages and image to show welcome page
            messages = []
            selectedImage = nil
            pendingImages = [] // Clear pending images
            pendingFile = nil // Clear pending file
            conversationHistory = []
            resetEngines()
        }

        saveSessions()
    }

    func renameSession(_ session: ChatSession, to newTitle: String) {
        guard !newTitle.trimmingCharacters(in: .whitespaces).isEmpty else { return }

        if let index = sessions.firstIndex(where: { $0.id == session.id }) {
            sessions[index].title = newTitle.trimmingCharacters(in: .whitespaces)
            saveSessions()
        }
    }

    // MARK: - Model Loading

    /// Dynamic system prompt that includes the user's name if set
    var systemPrompt: String {
        let userName = UserDefaults.standard.string(forKey: "user_name") ?? ""
        let trimmedName = userName.trimmingCharacters(in: .whitespaces)

        if trimmedName.isEmpty {
            return "You are Amma, a private personal assistant running on the user's device. Help with writing, planning, explanations, brainstorming, documents, and images. Be concise, practical, and honest. Never claim to have current or live information unless the user provides it. Ask a clarifying question only when it is necessary."
        } else {
            return "You are Amma, \(trimmedName)'s private personal assistant running on their device. Help with writing, planning, explanations, brainstorming, documents, and images. Be concise, practical, and honest. Never claim to have current or live information unless the user provides it. Use \(trimmedName)'s name occasionally, and ask a clarifying question only when it is necessary."
        }
    }

    /// Call this when user updates their name in settings to refresh the conversation
    func refreshSystemPrompt() {
        // Since we are using stateless prompts with prefix matching,
        // we just need to update the prompt for the next turn.
        print("System prompt refreshed with name: \(UserDefaults.standard.string(forKey: "user_name") ?? "none")")
    }

    func startListening() {
        // Clear any previous transcript before starting fresh
        transcript = ""
        speechRecognizer.start()
    }

    func stopListening() {
        speechRecognizer.stop()
    }

    func stopGeneration() {
        // Cancel the current generation task
        generationTask?.cancel()
        generationTask = nil

        // Reset generating state
        isGenerating = false

        // If there's a partial response, add it to messages
        if !currentResponse.isEmpty {
            messages.append(ChatMessageUI(role: .assistant, text: currentResponse))
            conversationHistory.append(ConversationMessage(
                role: .assistant,
                fullText: currentResponse,
                displayText: currentResponse,
                imageFilenames: [],
                fileNames: []
            ))
            currentResponse = ""
        }

        // Update session
        updateCurrentSession()
    }

    func loadModel() async {
        guard engine == nil else { return }

        // Set loading state immediately
        isLoading = true
        loadingStage = .preparing
        loadingStatus = "Preparing model..."
        loadingProgress = 0.0
        downloadStartTime = Date()
        lastProgressUpdate = Date()

        defer {
            isLoading = false
            loadingProgress = 0.0
            downloadSpeed = 0.0
            downloadStartTime = nil
            lastProgressUpdate = nil
        }

        do {
            let documentsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            let mobileModelURL = try await ensureModel(Self.mobileModel, in: documentsPath, status: "Downloading mobile assistant...")
            let mobileProjectorURL = try await ensureModel(Self.mobileProjector, in: documentsPath, status: "Downloading mobile vision...")

            var desktopModelURL: URL?
            if canUseDesktopModel {
                desktopModelURL = try await ensureModel(Self.desktopModel, in: documentsPath, status: "Downloading desktop assistant...")
            }

            // Update stage to loading
            loadingStage = .loading
            loadingStatus = "Loading model into memory..."
            loadingProgress = 0.9

            var visionConfig = canUseDesktopModel ? ForgeConfiguration.desktop() : ForgeConfiguration.vision()
            visionConfig.contextSize = max(visionConfig.contextSize, 4096)
            visionConfig.temperature = 0.1
            visionConfig.maxTokens = 256
            let newVisionEngine = try ForgeEngine(
                modelPath: mobileModelURL.path,
                clipPath: mobileProjectorURL.path,
                config: visionConfig
            )

            let newEngine: ForgeEngine
            if let desktopModelURL {
                var desktopConfig = ForgeConfiguration.desktop()
                desktopConfig.temperature = 0.2
                desktopConfig.topK = 80
                newEngine = try ForgeEngine(modelPath: desktopModelURL.path, config: desktopConfig)
                print("Desktop routing active: 8B assistant + 450M vision")
            } else {
                var mobileTextConfig = ForgeConfiguration.auto()
                mobileTextConfig.contextSize = max(mobileTextConfig.contextSize, 2048)
                mobileTextConfig.temperature = 0.1
                mobileTextConfig.maxTokens = max(mobileTextConfig.maxTokens, 256)
                newEngine = try ForgeEngine(modelPath: mobileModelURL.path, config: mobileTextConfig)
                if isRunningOnComputer {
                    print("Computer has less than 12 GB RAM; using the safe mobile model")
                }
            }

            // Set system prompt tokens to keep during context shifting
            let systemPromptTokens = systemPrompt.count / 4 // Rough estimate: 4 chars per token
            newEngine.tokensToKeep = Int32(systemPromptTokens + 10) // Add some buffer

            self.engine = newEngine
            self.visionEngine = newVisionEngine

            print("Model loaded successfully")

            // Final stage
            loadingProgress = 1.0
            loadingStatus = "Almost ready..."

            print(canUseDesktopModel ? "Desktop models loaded successfully" : "Mobile model loaded successfully")

            // Reclaim storage from the previous model only after its replacement is ready.
            for legacyName in [
                "LFM2.5-VL-1.6B-Q8_0.gguf",
                "LFM2.5-VL-1.6B-Q4_0.gguf",
                "mmproj-LFM2.5-VL-1.6b-Q8_0.gguf"
            ] {
                try? FileManager.default.removeItem(at: documentsPath.appendingPathComponent(legacyName))
            }

            // Small delay to show completion
            try? await Task.sleep(nanoseconds: 300_000_000) // 0.3 seconds

            if sessions.isEmpty {
                let initialSession = ChatSession(title: "New Chat")
                sessions.append(initialSession)
                currentSessionId = initialSession.id
                messages.append(ChatMessageUI(role: .assistant, text: greetingMessage))
            } else if let firstSession = sessions.first {
                loadSession(firstSession)
            }
        } catch {
            print("Failed to load vision model: \(error)")
            loadingStatus = "Failed to load model: \(error.localizedDescription)"
            loadingStage = .error
        }
    }

    // MARK: - Model Download

    private func ensureModel(_ model: ModelFile, in directory: URL, status: String) async throws -> URL {
        let destination = directory.appendingPathComponent(model.name)
        let trustMarker = destination.appendingPathExtension("sha256")
        if FileManager.default.fileExists(atPath: destination.path) {
            let attributes = try FileManager.default.attributesOfItem(atPath: destination.path)
            let size = (attributes[.size] as? NSNumber)?.int64Value ?? 0
            let marker = try? String(contentsOf: trustMarker, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
            let digestMatches: Bool
            if size != model.size {
                digestMatches = false
            } else if marker == model.sha256 {
                digestMatches = true
            } else {
                digestMatches = try await Task.detached(priority: .utility) {
                    try Self.sha256(of: destination) == model.sha256
                }.value
            }
            if size == model.size && digestMatches {
                if marker != model.sha256 {
                    try model.sha256.write(to: trustMarker, atomically: true, encoding: .utf8)
                }
                excludeFromBackup(destination)
                excludeFromBackup(trustMarker)
                return destination
            }
            try FileManager.default.removeItem(at: destination)
            try? FileManager.default.removeItem(at: trustMarker)
        }

        loadingStatus = status
        try await downloadModel(model, to: destination)
        return destination
    }

    private func downloadModel(_ model: ModelFile, to destination: URL) async throws {
        guard let url = URL(string: model.downloadURL) else {
            throw NSError(domain: "VisionViewModel", code: -1, userInfo: [NSLocalizedDescriptionKey: "Invalid URL"])
        }

        let delegate = DownloadProgressDelegate()
        let startTime = Date()

        // We use a dedicated session because URLSession.shared delegate support
        // can be unreliable for task-specific delegates, especially with redirects.
        let session = URLSession(configuration: .default, delegate: nil, delegateQueue: nil)

        delegate.onProgress = { progress, speed in
            Task { @MainActor in
                self.loadingProgress = progress
                self.downloadSpeed = speed

                let speedMB = speed / (1024 * 1024)
                let speedText = speedMB >= 1.0 ? String(format: "%.1f MB/s", speedMB) : String(format: "%.0f KB/s", speed / 1024)

                let now = Date()
                let totalElapsed = now.timeIntervalSince(startTime)
                let remaining = progress > 0 ? totalElapsed * (1.0 - progress) / progress : 0

                var timeText = ""
                if remaining > 0 && remaining < 3600 {
                    let minutes = Int(remaining / 60)
                    let seconds = Int(remaining.truncatingRemainder(dividingBy: 60))
                    timeText = minutes > 0 ? " • ~\(minutes)m \(seconds)s left" : " • ~\(seconds)s left"
                }

                let progressText = progress > 0 ? "\(Int(progress * 100))%" : "Calculating..."
                self.loadingStatus = "Downloading: \(progressText) • \(speedText)\(timeText)"
            }
        }

        // Use the modern async download with a delegate for high-speed chunked transfers
        let (tempURL, response) = try await session.download(from: url, delegate: delegate)

        guard let httpResponse = response as? HTTPURLResponse,
              httpResponse.statusCode == 200 else {
            throw NSError(domain: "VisionViewModel", code: -2, userInfo: [NSLocalizedDescriptionKey: "Download failed"])
        }

        let attributes = try FileManager.default.attributesOfItem(atPath: tempURL.path)
        let downloadedSize = (attributes[.size] as? NSNumber)?.int64Value ?? 0
        guard downloadedSize == model.size else {
            throw NSError(
                domain: "VisionViewModel",
                code: -3,
                userInfo: [NSLocalizedDescriptionKey: "Incomplete download (\(downloadedSize) of \(model.size) bytes)"]
            )
        }
        let actualSHA256 = try await Task.detached(priority: .utility) {
            try Self.sha256(of: tempURL)
        }.value
        guard actualSHA256 == model.sha256 else {
            throw NSError(
                domain: "VisionViewModel",
                code: -4,
                userInfo: [NSLocalizedDescriptionKey: "Downloaded model failed SHA-256 verification"]
            )
        }

        // Move from temporary location to final destination
        if FileManager.default.fileExists(atPath: destination.path) {
            try FileManager.default.removeItem(at: destination)
        }
        try FileManager.default.moveItem(at: tempURL, to: destination)
        let trustMarker = destination.appendingPathExtension("sha256")
        try model.sha256.write(to: trustMarker, atomically: true, encoding: .utf8)
        excludeFromBackup(destination)
        excludeFromBackup(trustMarker)
    }

    private nonisolated static func sha256(of url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let data = try handle.read(upToCount: 1024 * 1024), !data.isEmpty {
            hasher.update(data: data)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    /// Internal delegate to track high-speed download progress
    private final class DownloadProgressDelegate: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
        var onProgress: ((Double, Double) -> Void)?
        private var lastUpdate = Date()
        private var lastBytes: Int64 = 0

        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
            let now = Date()
            let timeInterval = now.timeIntervalSince(lastUpdate)

            // Update UI at most every 0.1 seconds (faster for high speed)
            if timeInterval >= 0.1 {
                let progress = totalBytesExpectedToWrite > 0 ? Double(totalBytesWritten) / Double(totalBytesExpectedToWrite) : 0
                let bytesSinceLast = totalBytesWritten - lastBytes
                let speed = Double(bytesSinceLast) / timeInterval

                onProgress?(progress, speed)

                lastUpdate = now
                lastBytes = totalBytesWritten
            }
        }

        func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
            // Handled by the await session.download call
        }
    }

    func recognizeImage(_ image: UIImage) {
        recognizeImages([image])
    }

    func recognizeImages(_ images: [UIImage], withText text: String = "") {
        guard let visionEngine = visionEngine else {
            print("Error: No vision engine available")
            return
        }

        guard let image = images.first else { return }

        // Cancel any existing generation task
        generationTask?.cancel()
        generationTask = nil

        // 1. Immediately update UI (user message)
        let displayText = text.isEmpty ? "[Image attached]" : text
        let userMessageUI = ChatMessageUI(role: .user, text: displayText, images: [image])
        messages.append(userMessageUI)

        // Reset state for new response
        currentResponse = ""
        isGenerating = true

        // 2. Start generation task
        generationTask = Task { @MainActor in
            let promptText = text.isEmpty ? "Describe this image and help me understand what matters." : text

            // Move heavy work to background
            let jpegData = await Task.detached(priority: .userInitiated) {
                // Heavy work: JPEG compression
                return image.jpegData(compressionQuality: 0.7) ?? Data()
            }.value

            // Add to history (after UI update)
            let filename = self.saveImageToDisk(jpegData)
            self.conversationHistory.append(ConversationMessage(
                role: .user,
                fullText: promptText,
                displayText: displayText,
                imageFilenames: [filename].compactMap { $0 },
                fileNames: []
            ))

            // Rebuild vision context so only this request's bitmap is active.
            try? visionEngine.reset()
            let fullPrompt = self.formatPrompt(systemPrompt: self.systemPrompt, userMessage: promptText)

            logMemory("forge_vision_stream_start")

            // Use the new non-blocking stream API
            for await token in visionEngine.generateVisionStream(imageData: jpegData, prompt: fullPrompt) {
                if Task.isCancelled { break }
                self.currentResponse += token
            }

            logMemory("forge_vision_stream_finished")

            if !Task.isCancelled && !self.currentResponse.isEmpty {
                self.messages.append(ChatMessageUI(role: .assistant, text: self.currentResponse))
                self.conversationHistory.append(ConversationMessage(
                    role: .assistant,
                    fullText: self.currentResponse,
                    displayText: self.currentResponse,
                    imageFilenames: [],
                    fileNames: []
                ))
                self.currentResponse = ""

                // The text engine did not see this vision turn. Its next request will replay
                // textual history once, then return to incremental generation.
                try? self.engine?.reset()
            }

            self.updateCurrentSession()

            self.isGenerating = false
        }
    }

    private func resizeImage(_ image: UIImage, maxDimension: CGFloat) -> UIImage {
        // Use CoreImage for GPU-accelerated scaling
        guard let cgImage = image.cgImage else { return image }

        let size = image.size
        let maxSide = max(size.width, size.height)

        // Safety: Prevent NaN or 0 dimensions
        guard maxSide > 0, maxDimension > 0 else { return image }

        if maxSide <= maxDimension {
            return image
        }

        let scale = maxDimension / maxSide
        let filter = CIFilter(name: "CILanczosScaleTransform")!
        filter.setValue(CIImage(cgImage: cgImage), forKey: kCIInputImageKey)
        filter.setValue(scale, forKey: kCIInputScaleKey)
        filter.setValue(1.0, forKey: kCIInputAspectRatioKey)

        let context = CIContext(options: [.useSoftwareRenderer: false])
        if let outputImage = filter.outputImage,
           let scaledCgImage = context.createCGImage(outputImage, from: outputImage.extent) {
            return UIImage(cgImage: scaledCgImage)
        }

        return image
    }

    private func formatPrompt(
        systemPrompt: String,
        userMessage: String,
        includeActiveImage: Bool = true,
        includeHistory: Bool = true
    ) -> String {
        // Unified format for both vision and text follow-ups
        var prompt = ""

        if !includeHistory {
            return "<|im_start|>user\n\(userMessage)<|im_end|>\n<|im_start|>assistant\n"
        }

        prompt = "<|im_start|>system\n\(systemPrompt)<|im_end|>\n"

        // Find the index of the LATEST message that contains an image
        // We only show the image markers for the most recent image because
        // the engine only receives the latest image data per request.
        let latestImageIndex = conversationHistory.lastIndex(where: { $0.role == .user && !$0.imageFilenames.isEmpty })

        // Add conversation history
        for (index, msg) in conversationHistory.enumerated() {
            let role = msg.role == .user ? "user" : "assistant"

            let content: String
            if includeActiveImage && index == latestImageIndex {
                // Only put markers for the active image
                let mediaMarker = getMediaMarker()
                content = "\(mediaMarker)\n\(msg.fullText)"
            } else {
                // Old images are treated as text history (prevents "number of bitmaps" mismatch)
                content = msg.fullText
            }

            prompt += "<|im_start|>\(role)\n\(content)<|im_end|>\n"
        }

        // Final assistant turn start
        prompt += "<|im_start|>assistant\n"

        return prompt
    }

    private func formatTextPrompt(systemPrompt: String, userMessage: String, isFirstTurn: Bool) -> String {
        return formatPrompt(
            systemPrompt: systemPrompt,
            userMessage: userMessage,
            includeActiveImage: false,
            includeHistory: isFirstTurn
        )
    }

    /// Reasoning models may stream a private `<think>...</think>` block before the answer.
    /// Keep that work out of both the UI and persisted conversation history.
    private func visibleAssistantText(from rawText: String) -> String {
        guard let thinkingStart = rawText.range(of: "<think>") else {
            return rawText
        }
        guard let thinkingEnd = rawText.range(
            of: "</think>",
            range: thinkingStart.upperBound..<rawText.endIndex
        ) else {
            return String(rawText[..<thinkingStart.lowerBound])
        }

        let answer = rawText[thinkingEnd.upperBound...]
        return String(answer.drop(while: { $0.isWhitespace }))
    }

    private func resetEngines() {
        try? engine?.reset()
        if let visionEngine, visionEngine !== engine {
            try? visionEngine.reset()
        }
    }

    func sendFollowUp(_ text: String) {
        guard let engine = engine else {
            print("Error: No engine available in sendFollowUp")
            return
        }

        // If there are pending images, send them with the text
        if !pendingImages.isEmpty {
            let imagesToSend = pendingImages
            pendingImages = [] // Clear pending images
            recognizeImages(imagesToSend, withText: text)
            return
        }

        // If there is a pending file, combine it with user text
        var messageText = text
        var displayText = text
        var fileNames: [String] = []

        if let fileToSend = pendingFile {
            // Combine user input with file content
            let userInputText = text.trimmingCharacters(in: .whitespaces)
            if userInputText.isEmpty {
                messageText = "Please analyze this document:\n\n\(fileToSend.content)"
                displayText = "[File attached]"
            } else {
                messageText = "\(text)\n\n--- Document Content ---\n\(fileToSend.content)"
                displayText = text
            }
            fileNames = [fileToSend.fileName]

            // Clear pending file after using it
            pendingFile = nil
        }

        // 1. Immediately update UI
        messages.append(ChatMessageUI(
            role: .user,
            text: displayText,
            fileNames: fileNames
        ))

        // Add to conversation history
        conversationHistory.append(ConversationMessage(
            role: .user,
            fullText: text, // Store the raw user text
            displayText: displayText,
            imageFilenames: [],
            fileNames: fileNames
        ))

        // If no text, return
        guard !messageText.trimmingCharacters(in: .whitespaces).isEmpty else { return }

        // Cancel any existing generation task
        generationTask?.cancel()

        currentResponse = ""
        isGenerating = true

        // 2. Start generation task
        let isFirstTurn = engine.isFirstTurn
        let fullPrompt = formatTextPrompt(
            systemPrompt: systemPrompt,
            userMessage: messageText,
            isFirstTurn: isFirstTurn
        )

        generationTask = Task { @MainActor in
            var rawResponse = ""
            logMemory("forge_followup_stream_start")

            // Use the new non-blocking stream API
            for await token in engine.generateTurnStream(prompt: fullPrompt, addBOS: isFirstTurn) {
                if Task.isCancelled { break }
                rawResponse += token
                self.currentResponse = self.visibleAssistantText(from: rawResponse)
            }

            logMemory("forge_followup_stream_finished")

            if !Task.isCancelled && !self.currentResponse.isEmpty {
                self.messages.append(ChatMessageUI(role: .assistant, text: self.currentResponse))
                self.conversationHistory.append(ConversationMessage(
                    role: .assistant,
                    fullText: self.currentResponse,
                    displayText: self.currentResponse,
                    imageFilenames: [],
                    fileNames: []
                ))
                self.currentResponse = ""
            }

            self.updateCurrentSession()

            self.isGenerating = false
        }
    }

    // MARK: - File Processing

    func processFile(at url: URL) {
        // Start accessing security-scoped resource before processing
        // This ensures we have access to files outside the app's sandbox
        let isAccessing = url.startAccessingSecurityScopedResource()

        Task { @MainActor in
            isLoading = true
            loadingStatus = "Processing file..."

            defer {
                // Stop accessing security-scoped resource when done
                if isAccessing {
                    url.stopAccessingSecurityScopedResource()
                }
                isLoading = false
            }

            do {
                // Check file size first (500KB limit)
                let fileSize = try getFileSize(url: url)
                if fileSize > 500_000 {
                    let fileSizeMB = Double(fileSize) / 1_000_000.0
                    messages.append(ChatMessageUI(
                        role: .assistant,
                        text: "❌ File is too large (\(String(format: "%.2f", fileSizeMB)) MB).\n\n" +
                              "Please select a file smaller than 500KB."
                    ))
                    if isAccessing {
                        url.stopAccessingSecurityScopedResource()
                    }
                    return
                }

                // Check if there's already a pending file (only one file allowed)
                if pendingFile != nil {
                    messages.append(ChatMessageUI(
                        role: .assistant,
                        text: "⚠️ You already have a file selected. Please remove it first before selecting a new file."
                    ))
                    if isAccessing {
                        url.stopAccessingSecurityScopedResource()
                    }
                    return
                }

                let extractedText = try await DocumentProcessor.extractText(from: url)

                // If text was extracted successfully, set as pending file
                if !extractedText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    let fileName = url.lastPathComponent

                    // Set as pending file (will be shown in input bar preview)
                    pendingFile = PendingFile(fileName: fileName, content: extractedText)
                } else {
                    messages.append(ChatMessageUI(
                        role: .assistant,
                        text: "The file appears to be empty or contains no extractable text."
                    ))
                }
            } catch {
                let errorMessage: String
                if let docError = error as? DocumentProcessingError {
                    errorMessage = docError.localizedDescription
                } else {
                    errorMessage = "Failed to process file: \(error.localizedDescription)"
                }

                // Show detailed error message with helpful suggestions
                var errorText = "❌ \(errorMessage)"

                // Add helpful suggestions based on error type
                if let docError = error as? DocumentProcessingError {
                    switch docError {
                    case .contentTooLong:
                        errorText += "\n\n💡 Tip: Try splitting your document into smaller parts, or use a shorter document."
                    case .fileTooLarge:
                        errorText += "\n\n💡 Tip: Please use files smaller than 500KB."
                    case .unsupportedFormat:
                        errorText += "\n\n💡 Tip: Try converting your file to PDF or TXT format first."
                    case .emptyDocument:
                        errorText += "\n\n💡 Tip: The file may be image-based. Try using the image recognition feature instead."
                    default:
                        break
                    }
                }

                messages.append(ChatMessageUI(
                    role: .assistant,
                    text: errorText
                ))
            }
        }
    }

    private func getFileSize(url: URL) throws -> Int {
        let resourceValues = try url.resourceValues(forKeys: [.fileSizeKey])
        return resourceValues.fileSize ?? 0
    }

    // MARK: - Disk Storage Helpers

    private var imagesDirectory: URL {
        let imagesDir = privateDataDirectory.appendingPathComponent("chat_images", isDirectory: true)
        if !FileManager.default.fileExists(atPath: imagesDir.path) {
            let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            let legacyImages = documents.appendingPathComponent("chat_images", isDirectory: true)
            if FileManager.default.fileExists(atPath: legacyImages.path) {
                try? FileManager.default.moveItem(at: legacyImages, to: imagesDir)
            } else {
                try? FileManager.default.createDirectory(at: imagesDir, withIntermediateDirectories: true)
            }
        }
        protectPrivateFile(imagesDir)
        return imagesDir
    }

    private var privateDataDirectory: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let directory = base.appendingPathComponent("PrivateAssistantData", isDirectory: true)
        if !FileManager.default.fileExists(atPath: directory.path) {
            try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        }
        protectPrivateFile(directory)
        return directory
    }

    private var sessionsFileURL: URL {
        privateDataDirectory.appendingPathComponent("chat_sessions.json")
    }

    private func excludeFromBackup(_ url: URL) {
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var mutableURL = url
        try? mutableURL.setResourceValues(values)
    }

    private func protectPrivateFile(_ url: URL) {
        excludeFromBackup(url)
        try? FileManager.default.setAttributes(
            [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
            ofItemAtPath: url.path
        )
    }

    private func saveImageToDisk(_ data: Data) -> String? {
        let filename = UUID().uuidString + ".jpg"
        let fileURL = imagesDirectory.appendingPathComponent(filename)
        do {
            try data.write(to: fileURL, options: [.atomic, .completeFileProtection])
            protectPrivateFile(fileURL)
            return filename
        } catch {
            print("❌ Failed to save image to disk: \(error)")
            return nil
        }
    }

    private func loadImageFromDisk(filename: String) -> Data? {
        let fileURL = imagesDirectory.appendingPathComponent(filename)
        return try? Data(contentsOf: fileURL)
    }

    private func deleteImageFromDisk(filename: String) {
        let fileURL = imagesDirectory.appendingPathComponent(filename)
        try? FileManager.default.removeItem(at: fileURL)
    }
}

// MARK: - Conversation Message (for persistence)

struct ConversationMessage: Codable {
    let role: ConversationRole
    let fullText: String
    let displayText: String
    let imageFilenames: [String] // Changed from Data to Filenames to keep UserDefaults small
    let fileNames: [String]
}

enum ConversationRole: String, Codable {
    case user
    case assistant
}

// MARK: - Chat Session

struct ChatSession: Identifiable, Codable {
    let id: UUID
    var title: String
    var history: [ConversationMessage]
    var updatedAt: Date

    init(id: UUID = UUID(), title: String, history: [ConversationMessage] = [], updatedAt: Date = Date()) {
        self.id = id
        self.title = title
        self.history = history
        self.updatedAt = updatedAt
    }
}

struct PendingFile: Identifiable {
    let id = UUID()
    let fileName: String
    let content: String
}

struct ChatMessageUI: Identifiable {
    let id = UUID()
    let role: ChatRole
    let text: String
    var image: UIImage? { images.first } // Backward compatibility
    var images: [UIImage] = []
    var fileNames: [String] = []
}

enum ChatRole {
    case user
    case assistant
}

enum LoadingStage {
    case preparing
    case downloading
    case loading
    case error

    var displayText: String {
        switch self {
        case .preparing:
            return "Preparing..."
        case .downloading:
            return "Downloading model..."
        case .loading:
            return "Loading into memory..."
        case .error:
            return "Error occurred"
        }
    }
}
