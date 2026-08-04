import SwiftUI
import PhotosUI
import Combine
import ForgeSwift
import StoreKit
import UniformTypeIdentifiers

private enum AmmaPalette {
    static let canvas = Color(red: 0.969, green: 0.961, blue: 0.941)
    static let ink = Color(red: 0.090, green: 0.102, blue: 0.090)
    static let muted = Color(red: 0.420, green: 0.431, blue: 0.400)
    static let accent = Color(red: 0.100, green: 0.263, blue: 0.204)
    static let line = Color(red: 0.875, green: 0.855, blue: 0.820)
}

struct MainView: View {
    @StateObject private var viewModel = VisionViewModel()
    @State private var showCamera = false
    @State private var showPhotoPicker = false
    @State private var showFilePicker = false
    @State private var showAttachmentMenu = false
    @State private var showSidebar = false
    @State private var showSettings = false
    @State private var showChatMenu = false
    @State private var showRenameAlert = false
    @State private var renameText: String = ""
    @State private var inputText = ""
    @State private var selectedImages: [UIImage] = []
    @State private var selectedImageForViewing: UIImage? = nil

    // Focus state for input field
    @FocusState private var isInputFocused: Bool

    // Check if user has sent at least one message
    private var hasUserMessages: Bool {
        viewModel.messages.contains { $0.role == .user }
    }

    var body: some View {
        ZStack(alignment: .leading) {
            // Background - Always visible to prevent black screen
            AmmaPalette.canvas
                .ignoresSafeArea()

            // Main Content
            ZStack(alignment: .top) {
                // Background - Full screen
                AmmaPalette.canvas
                    .ignoresSafeArea()

                VStack(spacing: 0) {
                    // Modern Header
                    ZStack {
                        HStack {
                            Button(action: {
                                hideKeyboard()
                                withAnimation(.spring(response: 0.35, dampingFraction: 0.8)) {
                                    showSidebar = true
                                }
                            }) {
                                Image(systemName: "line.3.horizontal")
                                    .font(.system(size: 18))
                                    .foregroundColor(.secondary)
                                    .frame(width: 44, height: 44) // Fixed tap target
                                    .contentShape(Rectangle())
                            }
                            .buttonStyle(SmoothButtonStyle())

                            Spacer()

                            // Show "New Chat" and "..." buttons after first message, otherwise show settings
                            if hasUserMessages {
                                HStack(spacing: 8) {
                                    Button(action: {
                                        hideKeyboard()
                                        viewModel.createNewChat()
                                    }) {
                                        Image(systemName: "square.and.pencil")
                                            .font(.system(size: 18))
                                            .foregroundColor(.secondary)
                                            .frame(width: 44, height: 44)
                                            .contentShape(Rectangle())
                                    }
                                    .buttonStyle(SmoothButtonStyle())

                                    Button(action: {
                                        hideKeyboard()
                                        showChatMenu = true
                                    }) {
                                        Image(systemName: "ellipsis")
                                            .font(.system(size: 18))
                                            .foregroundColor(.secondary)
                                            .frame(width: 44, height: 44)
                                            .contentShape(Rectangle())
                                    }
                                    .buttonStyle(SmoothButtonStyle())
                                    .confirmationDialog("", isPresented: $showChatMenu, titleVisibility: .hidden) {
                                        if let currentSession = viewModel.sessions.first(where: { $0.id == viewModel.currentSessionId }) {
                                            Button(action: {
                                                renameText = currentSession.title
                                                showRenameAlert = true
                                            }) {
                                                Label("Rename", systemImage: "pencil")
                                            }
                                            Button(role: .destructive, action: {
                                                withAnimation {
                                                    viewModel.deleteSession(currentSession)
                                                }
                                            }) {
                                                Label("Delete", systemImage: "trash")
                                            }
                                            Button("Cancel", role: .cancel) { }
                                        }
                                    }
                                }
                            } else {
                                Button(action: {
                                    hideKeyboard()
                                    showSettings = true
                                }) {
                                    Image(systemName: "gearshape")
                                        .font(.system(size: 18))
                                        .foregroundColor(.secondary)
                                        .frame(width: 44, height: 44) // Fixed tap target
                                        .contentShape(Rectangle())
                                }
                                .buttonStyle(SmoothButtonStyle())
                            }
                        }
                        .padding(.horizontal, 8)

                        // Center title
                        Text("Amma")
                            .font(.system(size: 19, weight: .medium, design: .serif))
                            .foregroundColor(AmmaPalette.ink)
                    }
                    .frame(height: 50)
                    .background(AmmaPalette.canvas)

                    Rectangle()
                        .fill(AmmaPalette.line)
                        .frame(height: 0.5)

                    // Chat area
                    ScrollViewReader { proxy in
                        ScrollView(showsIndicators: false) {
                            LazyVStack(spacing: 16) {
                                if viewModel.messages.isEmpty && viewModel.selectedImage == nil && !viewModel.isLoading {
                                    WelcomeView()
                                        .padding(.top, 40)
                                        .transition(.opacity.combined(with: .scale(scale: 0.95)))
                                }

                                ForEach(viewModel.messages) { message in
                                    MessageRow(message: message, onImageTap: { image in
                                        selectedImageForViewing = image
                                    })
                                        .id(message.id)
                                        .transition(.opacity.combined(with: .move(edge: .bottom)))
                                }

                                if !viewModel.currentResponse.isEmpty {
                                    MessageRow(message: ChatMessageUI(role: .assistant, text: viewModel.currentResponse))
                                        .id("streaming")
                                }

                                if viewModel.isGenerating && viewModel.currentResponse.isEmpty {
                                    AnalyzingView()
                                        .id("thinking")
                                        .transition(.opacity)
                                }
                            }
                            .padding(.horizontal, 16)
                            .padding(.top, 8)
                            .padding(.bottom, 10)
                            .animation(.easeInOut(duration: 0.25), value: viewModel.messages.count)
                        }
                        .modifier(ScrollDismissKeyboardModifier())
                        .onChange(of: viewModel.messages.count) { _ in
                            withAnimation(.easeOut(duration: 0.2)) {
                                proxy.scrollTo(viewModel.messages.last?.id, anchor: .bottom)
                            }
                        }
                        .onChange(of: viewModel.currentResponse) { _ in
                            proxy.scrollTo("streaming", anchor: .bottom)
                        }
                    }

                    // Input bar with focus binding
                    InputBar(
                        text: $inputText,
                        isFocused: $isInputFocused,
                        transcript: viewModel.transcript,
                        isRecording: viewModel.isRecording,
                        isGenerating: viewModel.isGenerating,
                        isDisabled: viewModel.isLoading || viewModel.isGenerating,
                        pendingImages: viewModel.pendingImages,
                        pendingFile: viewModel.pendingFile,
                        onRemoveImage: { index in
                            viewModel.pendingImages.remove(at: index)
                        },
                        onRemoveFile: {
                            viewModel.pendingFile = nil
                        },
                        onSend: {
                            let textToSend = inputText
                            inputText = ""
                            // Stop recording if user is recording
                            if viewModel.isRecording {
                                viewModel.stopListening()
                            }
                            viewModel.sendFollowUp(textToSend)
                        },
                        onAttachment: { showAttachmentMenu = true },
                        onMicTap: {
                            if viewModel.isRecording {
                                // Stop recording - the transcript is already in inputText via onReceive
                                viewModel.stopListening()
                            } else {
                                // Start fresh recording - clear any previous text
                                inputText = ""
                                viewModel.startListening()
                            }
                        },
                        onStopGeneration: {
                            viewModel.stopGeneration()
                        }
                    )
                    .padding(.bottom, 8)
                }
                .safeAreaInset(edge: .top) { Color.clear.frame(height: 0) }
            }
            .preferredColorScheme(.light)
            .onReceive(viewModel.$transcript) { newTranscript in
                if viewModel.isRecording {
                    inputText = newTranscript
                }
            }
            .overlay {
                if showAttachmentMenu {
                    ZStack {
                        // Background tap area to close menu
                        Color.clear
                            .contentShape(Rectangle())
                            .onTapGesture {
                                withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                                    showAttachmentMenu = false
                                }
                            }

                        // Menu panel
                        VStack {
                            Spacer()
                            AttachmentMenuView(
                                isPresented: $showAttachmentMenu,
                                onCamera: {
                                    showAttachmentMenu = false
                                    showCamera = true
                                },
                                onPhotos: {
                                    showAttachmentMenu = false
                                    showPhotoPicker = true
                                },
                                onFiles: {
                                    showAttachmentMenu = false
                                    showFilePicker = true
                                }
                            )
                            .padding(.bottom, 80) // Position above input bar
                        }
                    }
                }
            }
            .sheet(isPresented: $showCamera) {
                CameraPicker(selectedImage: $viewModel.selectedImage) { image in
                    // Add image to pending list for preview in input bar
                    viewModel.pendingImages.append(image)
                }
            }
            .sheet(isPresented: $showPhotoPicker) {
                MultiPhotoPicker(selectedImages: $selectedImages) { images in
                    // Only one image is supported, so take the first one
                    if let firstImage = images.first {
                        viewModel.pendingImages = [firstImage]
                    }
                }
            }
            .sheet(isPresented: $showFilePicker) {
                DocumentPicker(
                    onFileSelected: { url in
                        viewModel.processFile(at: url)
                    },
                    onError: { errorMessage in
                        viewModel.messages.append(ChatMessageUI(
                            role: .assistant,
                            text: errorMessage
                        ))
                    }
                )
            }
            .sheet(isPresented: $showSettings) {
                SettingsView(viewModel: viewModel)
            }
            .fullScreenCover(isPresented: Binding(
                get: { selectedImageForViewing != nil },
                set: { if !$0 { selectedImageForViewing = nil } }
            )) {
                if let image = selectedImageForViewing {
                    ImageViewer(image: image)
                }
            }
            .alert("Rename Chat", isPresented: $showRenameAlert) {
                TextField("Chat title", text: $renameText)
                Button("Cancel", role: .cancel) { }
                Button("Save") {
                    if let currentSession = viewModel.sessions.first(where: { $0.id == viewModel.currentSessionId }) {
                        viewModel.renameSession(currentSession, to: renameText)
                    }
                }
            } message: {
                Text("Enter a new name for this chat")
            }
            .overlay {
                if viewModel.isLoading {
                    LoadingOverlay(
                        status: viewModel.loadingStatus,
                        progress: viewModel.loadingProgress,
                        downloadSpeed: viewModel.downloadSpeed,
                        stage: viewModel.loadingStage
                    )
                    .transition(.opacity.combined(with: .scale(scale: 0.95)))
                    .zIndex(1000)
                }
            }
            .animation(.easeInOut(duration: 0.3), value: viewModel.isLoading)
            .onAppear {
                // Ensure loading state is set immediately if model needs to be loaded
                if !viewModel.isModelLoaded && !viewModel.isLoading {
                    viewModel.isLoading = true
                    viewModel.loadingStatus = "Preparing..."
                    viewModel.loadingStage = .preparing
                }

                Task {
                    await viewModel.loadModel()
                }
            }
            // Auto-focus input when model finishes loading (like ChatGPT)
            .onChange(of: viewModel.isLoading) { isLoading in
                if !isLoading {
                    // Small delay to ensure UI is ready
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                        isInputFocused = true
                    }
                }
            }
            // Re-focus when sidebar closes
            .onChange(of: showSidebar) { isSidebarShowing in
                if !isSidebarShowing {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) {
                        isInputFocused = true
                    }
                }
            }

            // Sidebar Overlay with smooth animation
            if showSidebar {
                Color.black.opacity(0.4)
                    .ignoresSafeArea()
                    .onTapGesture {
                        withAnimation(.spring(response: 0.35, dampingFraction: 0.8)) {
                            showSidebar = false
                        }
                    }
                    .transition(.opacity)

                SidebarView(viewModel: viewModel, isPresented: $showSidebar)
                    .transition(.move(edge: .leading).combined(with: .opacity))
                    .zIndex(2)
            }
        }
        .animation(.spring(response: 0.35, dampingFraction: 0.8), value: showSidebar)
    }

    private func hideKeyboard() {
        UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
    }
}

// MARK: - Smooth Button Style
struct SmoothButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.92 : 1.0)
            .opacity(configuration.isPressed ? 0.7 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
    }
}

// MARK: - iOS 16+ Scroll Dismiss Keyboard Modifier
struct ScrollDismissKeyboardModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 16.0, *) {
            content.scrollDismissesKeyboard(.interactively)
        } else {
            content
        }
    }
}

// MARK: - Sidebar View
struct SidebarView: View {
    @ObservedObject var viewModel: VisionViewModel
    @Binding var isPresented: Bool
    @State private var sessionToRename: ChatSession?
    @State private var renameText: String = ""
    @State private var showRenameAlert = false

    var body: some View {
        VStack(spacing: 0) {
            // Header with search
            VStack(spacing: 16) {
                HStack(spacing: 12) {
                    HStack(spacing: 8) {
                        Image(systemName: "magnifyingglass")
                            .font(.system(size: 14, weight: .medium))
                            .foregroundColor(.secondary)
                        TextField("Search chats", text: $viewModel.searchText)
                            .font(.system(size: 16))
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(Color(.systemGray5))
                    .cornerRadius(10)

                    Button(action: {
                        viewModel.createNewChat()
                        withAnimation(.spring(response: 0.35, dampingFraction: 0.8)) {
                            isPresented = false
                        }
                    }) {
                        Image(systemName: "square.and.pencil")
                            .font(.system(size: 18))
                            .foregroundColor(.primary)
                            .frame(width: 36, height: 36)
                            .background(Color(.systemGray5))
                            .clipShape(Circle())
                    }
                    .buttonStyle(SmoothButtonStyle())
                }
                .padding(.horizontal, 16)
                .padding(.top, 24)
            }
            .padding(.bottom, 16)

            Divider()

            // History List
            ScrollView(showsIndicators: false) {
                LazyVStack(alignment: .leading, spacing: 0) {
                    Text("Chat History")
                        .font(.system(size: 13, weight: .medium))
                        .foregroundColor(.secondary)
                        .padding(.horizontal, 20)
                        .padding(.top, 20)
                        .padding(.bottom, 10)

                    if viewModel.filteredSessions.isEmpty {
                        VStack(spacing: 8) {
                            Image(systemName: viewModel.searchText.isEmpty ? "bubble.left.and.bubble.right" : "magnifyingglass")
                                .font(.system(size: 32))
                                .foregroundColor(.secondary.opacity(0.5))
                            Text(viewModel.searchText.isEmpty ? "No chats yet" : "No results found")
                                .font(.system(size: 14))
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.top, 40)
                        .transition(.opacity)
                    } else {
                        ForEach(viewModel.filteredSessions) { session in
                            SessionRow(
                                session: session,
                                isSelected: viewModel.currentSessionId == session.id,
                                onTap: {
                                    viewModel.loadSession(session)
                                    withAnimation(.spring(response: 0.35, dampingFraction: 0.8)) {
                                        isPresented = false
                                    }
                                },
                                onRename: {
                                    sessionToRename = session
                                    renameText = session.title
                                    showRenameAlert = true
                                },
                                onDelete: {
                                    withAnimation {
                                        viewModel.deleteSession(session)
                                    }
                                }
                            )
                        }
                    }
                }
                .animation(.easeInOut(duration: 0.2), value: viewModel.filteredSessions.count)
            }

            Spacer()
        }
        .frame(width: min(UIScreen.main.bounds.width * 0.8, 320)) // Cap max width
        .background(Color(.systemBackground))
        .ignoresSafeArea(edges: .bottom)
        .alert("Rename Chat", isPresented: $showRenameAlert) {
            TextField("Chat title", text: $renameText)
            Button("Cancel", role: .cancel) { }
            Button("Save") {
                if let session = sessionToRename {
                    viewModel.renameSession(session, to: renameText)
                }
            }
        } message: {
            Text("Enter a new name for this chat")
        }
    }
}

// MARK: - Session Row with Context Menu
struct SessionRow: View {
    let session: ChatSession
    let isSelected: Bool
    let onTap: () -> Void
    let onRename: () -> Void
    let onDelete: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 12) {
                Text(session.title)
                    .font(.system(size: 15))
                    .lineLimit(1)
                    .foregroundColor(isSelected ? .primary : .primary.opacity(0.7))
                Spacer()
                if isSelected {
                    Circle()
                        .fill(Color.blue)
                        .frame(width: 6, height: 6)
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 14)
            .background(isSelected ? Color(.systemGray5) : Color.clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(PlainButtonStyle())
        .contextMenu {
            Button(action: onRename) {
                Label("Rename", systemImage: "pencil")
            }

            Button(role: .destructive, action: onDelete) {
                Label("Delete", systemImage: "trash")
            }
        }
    }
}

// MARK: - Welcome View
struct WelcomeView: View {
    var body: some View {
        VStack(spacing: 24) {
            AmmaMark(size: 76)

            Text("Amma")
                .font(.system(size: 34, weight: .medium, design: .serif))
                .foregroundColor(AmmaPalette.ink)
        }
    }
}

private struct AmmaMark: View {
    let size: CGFloat

    var body: some View {
        ZStack {
            Circle()
                .stroke(AmmaPalette.accent.opacity(0.22), lineWidth: 1)
            Circle()
                .fill(AmmaPalette.accent)
                .frame(width: size * 0.72, height: size * 0.72)
            Text("A")
                .font(.system(size: size * 0.38, weight: .medium, design: .serif))
                .foregroundColor(.white)
                .offset(y: -1)
        }
        .frame(width: size, height: size)
    }
}

// MARK: - Message Row
struct MessageRow: View {
    let message: ChatMessageUI
    var onImageTap: ((UIImage) -> Void)? = nil

    // Get the image (only one image is supported now)
    private var image: UIImage? {
        return message.images.first ?? message.image
    }

    var body: some View {
        VStack(alignment: message.role == .user ? .trailing : .leading, spacing: 8) {
            // Display image (only one image supported)
            if let image = image {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFill()
                    .frame(width: 120, height: 120)
                    .clipped()
                    .cornerRadius(16)
                    .onTapGesture {
                        onImageTap?(image)
                    }
            }

            // Display file attachment (if present - single file)
            if let fileName = message.fileNames.first {
                HStack(spacing: 8) {
                    Image(systemName: "doc.fill")
                        .font(.system(size: 16))
                        .foregroundColor(message.role == .user ? .white : AmmaPalette.accent)

                    Text(fileName)
                        .font(.system(size: 14))
                        .foregroundColor(message.role == .user ? .white : .primary)
                        .lineLimit(1)
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(message.role == .user ? AmmaPalette.accent : AmmaPalette.line.opacity(0.45))
                .cornerRadius(14)
            }

            // Display text (skip placeholder text for image-only or file-only messages)
            let textToShow = message.text
            let isImagePlaceholder = textToShow == "[Image attached]" || textToShow.hasPrefix("[") && textToShow.contains("images attached]")
            let isFilePlaceholder = textToShow == "[File attached]" || textToShow == "[Files attached]"

            if !textToShow.isEmpty && !isImagePlaceholder && !isFilePlaceholder {
                Text(textToShow)
                    .font(.system(size: 16))
                    .foregroundColor(message.role == .user ? .white : AmmaPalette.ink)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(message.role == .user ? AmmaPalette.accent : Color.clear)
                    .cornerRadius(18)
                    .textSelection(.enabled) // Allow text selection
            }
        }
        .frame(maxWidth: .infinity, alignment: message.role == .user ? .trailing : .leading)
    }
}

// MARK: - Analyzing View
struct AnalyzingView: View {
    @State private var dots = ""
    let timer = Timer.publish(every: 0.4, on: .main, in: .common).autoconnect()

    var body: some View {
        Text("Analyzing\(dots)")
            .font(.system(size: 15))
            .foregroundColor(AmmaPalette.muted)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .onReceive(timer) { _ in
                dots = dots.count >= 3 ? "" : dots + "."
            }
    }
}

// MARK: - Input Bar
struct InputBar: View {
    @Binding var text: String
    var isFocused: FocusState<Bool>.Binding
    let transcript: String
    let isRecording: Bool
    let isGenerating: Bool
    let isDisabled: Bool
    let pendingImages: [UIImage]
    let pendingFile: PendingFile?
    let onRemoveImage: (Int) -> Void
    let onRemoveFile: () -> Void
    let onSend: () -> Void
    let onAttachment: () -> Void
    let onMicTap: () -> Void
    let onStopGeneration: () -> Void

    private var showSend: Bool {
        !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !pendingImages.isEmpty || pendingFile != nil
    }

    var body: some View {
        VStack(spacing: 0) {
            // Image and file previews (above input bar)
            if !pendingImages.isEmpty || pendingFile != nil {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        // Image previews
                        ForEach(Array(pendingImages.enumerated()), id: \.offset) { index, image in
                            ZStack(alignment: .topTrailing) {
                                Image(uiImage: image)
                                    .resizable()
                                    .scaledToFill()
                                    .frame(width: 60, height: 60)
                                    .clipped()
                                    .cornerRadius(8)

                                // Remove button
                                Button(action: { onRemoveImage(index) }) {
                                    Image(systemName: "xmark.circle.fill")
                                        .font(.system(size: 18))
                                        .foregroundColor(.white)
                                        .background(Color.black.opacity(0.6))
                                        .clipShape(Circle())
                                }
                                .offset(x: 6, y: -6)
                            }
                        }

                        // File preview (single file)
                        if let file = pendingFile {
                            ZStack(alignment: .topTrailing) {
                                VStack(spacing: 4) {
                                    Image(systemName: "doc.fill")
                                        .font(.system(size: 24))
                                        .foregroundColor(.blue)

                                    Text(file.fileName)
                                        .font(.system(size: 10))
                                        .foregroundColor(.primary)
                                        .lineLimit(1)
                                        .frame(maxWidth: 60)
                                }
                                .frame(width: 60, height: 60)
                                .background(Color(.systemGray6))
                                .cornerRadius(8)

                                // Remove button
                                Button(action: { onRemoveFile() }) {
                                    Image(systemName: "xmark.circle.fill")
                                        .font(.system(size: 18))
                                        .foregroundColor(.white)
                                        .background(Color.black.opacity(0.6))
                                        .clipShape(Circle())
                                }
                                .offset(x: 6, y: -6)
                            }
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.top, 8)
                    .padding(.bottom, 4)
                }
            }

            // Input bar
            HStack(spacing: 10) {
                // + button for attachments (fixed size)
                Button(action: onAttachment) {
                    Image(systemName: "plus")
                        .font(.system(size: 20, weight: .medium))
                        .foregroundColor(.secondary)
                        .frame(width: 36, height: 36)
                        .background(AmmaPalette.line.opacity(0.5))
                        .clipShape(Circle())
                        .contentShape(Circle())
                }
                .buttonStyle(SmoothButtonStyle())
                .disabled(isDisabled)
                .opacity(isDisabled ? 0.4 : 1)

                // Text field container - expands to fill available space
                HStack(spacing: 8) {
                    TextField(isRecording ? "Listening..." : "Ask anything", text: $text)
                        .font(.system(size: 16))
                        .focused(isFocused)
                        .disabled(isDisabled || isRecording)
                        .textInputAutocapitalization(.sentences)
                        .submitLabel(.send)
                        .onSubmit {
                            if showSend {
                                onSend()
                            }
                        }

                    // Mic button (fixed size inside text field) - only show when not generating
                    if !isGenerating {
                        Button(action: onMicTap) {
                            Image(systemName: isRecording ? "stop.circle.fill" : "mic")
                                .font(.system(size: 18))
                                .foregroundColor(isRecording ? .red : .secondary)
                                .frame(width: 28, height: 28)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(SmoothButtonStyle())
                    }

                    // Generation status indicator - show when generating
                    if isGenerating {
                        Button(action: onStopGeneration) {
                            HStack(spacing: 4) {
                                ProgressView()
                                    .scaleEffect(0.7)
                                    .tint(.secondary)
                                Text("Generating")
                                    .font(.system(size: 12))
                                    .foregroundColor(.secondary)
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color(.systemGray5))
                            .cornerRadius(12)
                        }
                        .buttonStyle(SmoothButtonStyle())
                        .transition(.scale.combined(with: .opacity))
                    }

                    // Send button inside text field area when visible and not generating
                    if showSend && !isGenerating {
                        Button(action: onSend) {
                            Image(systemName: "arrow.up.circle.fill")
                                .font(.system(size: 28))
                                .foregroundColor(.black)
                        }
                        .buttonStyle(SmoothButtonStyle())
                        .disabled(isDisabled)
                        .transition(.scale.combined(with: .opacity))
                    }
                }
                .padding(.leading, 14)
                .padding(.trailing, 10)
                .padding(.vertical, 8)
                .background(Color.white.opacity(0.72))
                .clipShape(Capsule())
                .overlay(Capsule().stroke(AmmaPalette.line, lineWidth: 1))
                .animation(.spring(response: 0.25, dampingFraction: 0.7), value: showSend)
                .animation(.spring(response: 0.25, dampingFraction: 0.7), value: isGenerating)
            }
            .padding(.horizontal, 12)
            .padding(.top, 4)
            .padding(.bottom, 8)
        }
        .background(AmmaPalette.canvas)
    }
}

// MARK: - Loading Overlay
struct LoadingOverlay: View {
    let status: String
    let progress: Double
    let downloadSpeed: Double
    let stage: LoadingStage

    var body: some View {
        ZStack {
            AmmaPalette.canvas.ignoresSafeArea()

            VStack(spacing: 28) {
                AmmaMark(size: 82)

                Text(status)
                    .font(.system(size: 16, weight: .medium))
                    .foregroundColor(AmmaPalette.ink)
                    .lineLimit(1)

                if stage == .downloading || stage == .loading {
                    VStack(spacing: 10) {
                        ProgressView(value: progress)
                            .tint(AmmaPalette.accent)
                            .frame(width: 176)
                        Text("\(Int(progress * 100))%")
                            .font(.system(size: 12, weight: .medium, design: .monospaced))
                            .foregroundColor(AmmaPalette.muted)
                    }
                } else {
                    ProgressView()
                        .tint(AmmaPalette.accent)
                }
            }
            .offset(y: -20)
        }
    }
}


// Helper Pickers
struct CameraPicker: UIViewControllerRepresentable {
    @Binding var selectedImage: UIImage?
    var onImageSelected: (UIImage) -> Void
    @Environment(\.presentationMode) var presentationMode

    func makeUIViewController(context: Context) -> UIImagePickerController {
        let picker = UIImagePickerController()
        picker.sourceType = .camera
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: UIImagePickerController, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, UIImagePickerControllerDelegate, UINavigationControllerDelegate {
        let parent: CameraPicker

        init(_ parent: CameraPicker) {
            self.parent = parent
        }

        func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey : Any]) {
            if let image = info[.originalImage] as? UIImage {
                parent.selectedImage = image
                parent.onImageSelected(image)
            }
            parent.presentationMode.wrappedValue.dismiss()
        }
    }
}

struct PhotoPicker: UIViewControllerRepresentable {
    @Binding var selectedImage: UIImage?
    var onImageSelected: (UIImage) -> Void
    @Environment(\.presentationMode) var presentationMode

    func makeUIViewController(context: Context) -> PHPickerViewController {
        var config = PHPickerConfiguration()
        config.filter = .images
        let picker = PHPickerViewController(configuration: config)
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: PHPickerViewController, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, PHPickerViewControllerDelegate {
        let parent: PhotoPicker

        init(_ parent: PhotoPicker) {
            self.parent = parent
        }

        func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            parent.presentationMode.wrappedValue.dismiss()
            guard let provider = results.first?.itemProvider, provider.canLoadObject(ofClass: UIImage.self) else { return }

            provider.loadObject(ofClass: UIImage.self) { image, _ in
                if let image = image as? UIImage {
                    DispatchQueue.main.async {
                        self.parent.selectedImage = image
                        self.parent.onImageSelected(image)
                    }
                }
            }
        }
    }
}

// MARK: - Multi Photo Picker (supports up to 5 images)
struct MultiPhotoPicker: UIViewControllerRepresentable {
    @Binding var selectedImages: [UIImage]
    var onImagesSelected: ([UIImage]) -> Void
    @Environment(\.presentationMode) var presentationMode

    func makeUIViewController(context: Context) -> PHPickerViewController {
        var config = PHPickerConfiguration()
        config.filter = .images
        config.selectionLimit = 1 // Limit to 1 image (model only supports one image per message)
        let picker = PHPickerViewController(configuration: config)
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: PHPickerViewController, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, PHPickerViewControllerDelegate {
        let parent: MultiPhotoPicker

        init(_ parent: MultiPhotoPicker) {
            self.parent = parent
        }

        func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
            parent.presentationMode.wrappedValue.dismiss()

            guard !results.isEmpty else { return }

            var loadedImages: [UIImage] = []
            let group = DispatchGroup()

            for result in results {
                group.enter()
                result.itemProvider.loadObject(ofClass: UIImage.self) { image, _ in
                    if let image = image as? UIImage {
                        loadedImages.append(image)
                    }
                    group.leave()
                }
            }

            group.notify(queue: .main) {
                self.parent.selectedImages = loadedImages
                self.parent.onImagesSelected(loadedImages)
            }
        }
    }
}

// MARK: - Document Picker
struct DocumentPicker: UIViewControllerRepresentable {
    @Environment(\.presentationMode) var presentationMode
    var onFileSelected: (URL) -> Void
    var onError: ((String) -> Void)? = nil // Optional error callback

    func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
        // Support PDF, TXT, MD, DOCX, XLSX file types
        var contentTypes: [UTType] = [
            UTType.pdf,
            UTType.text,
            UTType.plainText
        ]

        // Add DOCX support if available
        if let docxType = UTType("org.openxmlformats.wordprocessingml.document") {
            contentTypes.append(docxType)
        }

        // Add XLSX support if available
        if let xlsxType = UTType("org.openxmlformats.spreadsheetml.sheet") {
            contentTypes.append(xlsxType)
        }

        // Add legacy Excel support
        if let xlsType = UTType("com.microsoft.excel.xls") {
            contentTypes.append(xlsType)
        }

        // Add markdown support if available
        if let mdType = UTType("net.daringfireball.markdown") {
            contentTypes.append(mdType)
        }

        // Fallback to data type for other formats
        contentTypes.append(UTType.data)

        let picker = UIDocumentPickerViewController(forOpeningContentTypes: contentTypes, asCopy: true)
        picker.delegate = context.coordinator
        picker.allowsMultipleSelection = false // Only allow single file selection
        return picker
    }

    func updateUIViewController(_ uiViewController: UIDocumentPickerViewController, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    class Coordinator: NSObject, UIDocumentPickerDelegate {
        let parent: DocumentPicker

        init(_ parent: DocumentPicker) {
            self.parent = parent
        }

        func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
            parent.presentationMode.wrappedValue.dismiss()

            // Only process the first file (single file selection)
            guard let url = urls.first else { return }

            // Check file size before processing (500KB limit)
            do {
                let resourceValues = try url.resourceValues(forKeys: [.fileSizeKey])
                if let fileSize = resourceValues.fileSize, fileSize > 500_000 {
                    // File too large - show friendly error message
                    let fileSizeKB = Double(fileSize) / 1_000.0
                    let fileSizeMB = Double(fileSize) / 1_000_000.0
                    let sizeString = fileSizeMB >= 1.0
                        ? String(format: "%.2f MB", fileSizeMB)
                        : String(format: "%.0f KB", fileSizeKB)

                    let errorMessage = "❌ File is too large (\(sizeString)).\n\n" +
                                      "Please select a file smaller than 500KB."

                    // Call error callback if available
                    parent.onError?(errorMessage)
                    return
                }
            } catch {
                // If we can't check size, proceed and let DocumentProcessor handle it
            }

            // Start accessing security-scoped resource immediately
            // This is required for files outside the app's sandbox (like iCloud Drive)
            _ = url.startAccessingSecurityScopedResource()

            // Call the callback with the URL
            parent.onFileSelected(url)
        }

        func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
            parent.presentationMode.wrappedValue.dismiss()
        }
    }
}

// MARK: - Settings View
struct SettingsView: View {
    @Environment(\.dismiss) private var dismiss
    @ObservedObject var viewModel: VisionViewModel
    @AppStorage("user_name") private var userName: String = ""
    @AppStorage("user_birthday") private var userBirthday: String = ""

    // Track name changes
    @State private var previousName: String = ""

    // Birthday date picker state
    @State private var birthdayDate: Date = Date()
    private let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "MM/dd/yyyy"
        return formatter
    }()

    // Placeholder App Store link - update with real App ID when published
    private let appStoreLink = "https://apps.apple.com/app/amma-recognize/id000000000"
    private let appStoreReviewLink = "https://apps.apple.com/app/amma-recognize/id000000000?action=write-review"
    private let privacyURL = URL(string: "https://privacy.amma.live")!
    private let termsURL = URL(string: "https://term.amma.live")!

    var body: some View {
        NavigationView {
            List {
                // Profile Section
                Section {
                    HStack {
                        Text("Name")
                            .foregroundColor(.secondary)
                        TextField("Enter your name", text: $userName)
                            .multilineTextAlignment(.trailing)
                            .onChange(of: userName) { newValue in
                                // Refresh system prompt when name changes
                                viewModel.refreshSystemPrompt()
                            }
                    }

                    HStack {
                        Text("Birthday")
                            .foregroundColor(.secondary)
                        Spacer()
                        DatePicker("", selection: $birthdayDate, displayedComponents: .date)
                            .datePickerStyle(.compact)
                            .labelsHidden()
                            .onChange(of: birthdayDate) { newDate in
                                userBirthday = dateFormatter.string(from: newDate)
                            }
                    }
                } header: {
                    Text("Profile")
                }

                // Share Section
                Section {
                    Button(action: shareWithFriends) {
                        HStack {
                            Image(systemName: "person.2.fill")
                                .foregroundColor(.blue)
                                .frame(width: 28)
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Recommend to Friends")
                                    .foregroundColor(.primary)
                                Text("Share Amma with your friends")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                            Image(systemName: "square.and.arrow.up")
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("Share")
                }

                // About Section
                Section {
                    HStack {
                        Image(systemName: "dollarsign.circle.fill")
                            .foregroundColor(.green)
                            .frame(width: 28)
                        VStack(alignment: .leading, spacing: 2) {
                            Text("$1 USD Forever")
                                .fontWeight(.medium)
                            Text("One-time purchase, lifetime access")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    HStack {
                        Image(systemName: "iphone")
                            .foregroundColor(.blue)
                            .frame(width: 28)
                        VStack(alignment: .leading, spacing: 2) {
                            Text("All Data on Your Phone")
                                .fontWeight(.medium)
                            Text("100% privacy protected")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    HStack {
                        Image(systemName: "sparkles")
                            .foregroundColor(.green)
                            .frame(width: 28)
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Personal Assistant")
                                .fontWeight(.medium)
                            Text("Writing, planning, images and documents")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("About Amma")
                }

                // Rate & Support Section
                Section {
                    Button(action: requestReview) {
                        HStack {
                            Image(systemName: "star.fill")
                                .foregroundColor(.yellow)
                                .frame(width: 28)
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Rate Amma ⭐⭐⭐⭐⭐")
                                    .foregroundColor(.primary)
                                Text("Love Amma? Give us 5 stars!")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                            Image(systemName: "arrow.up.right")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("Support")
                }

                // Legal Section
                Section {
                    Link(destination: privacyURL) {
                        HStack {
                            Image(systemName: "hand.raised.fill")
                                .foregroundColor(.purple)
                                .frame(width: 28)
                            Text("Privacy Policy")
                                .foregroundColor(.primary)
                            Spacer()
                            Image(systemName: "arrow.up.right")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    Link(destination: termsURL) {
                        HStack {
                            Image(systemName: "doc.text.fill")
                                .foregroundColor(.orange)
                                .frame(width: 28)
                            Text("Terms of Use")
                                .foregroundColor(.primary)
                            Spacer()
                            Image(systemName: "arrow.up.right")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("Legal")
                }

                // App Info
                Section {
                    HStack {
                        Text("Version")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("1.0.0 (Rust)")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                    .font(.body.weight(.semibold))
                }
            }
            .onAppear {
                // Load birthday from storage when view appears
                if !userBirthday.isEmpty, let date = dateFormatter.date(from: userBirthday) {
                    birthdayDate = date
                } else {
                    // If no birthday is set, use a default date (e.g., 18 years ago)
                    let calendar = Calendar.current
                    if let defaultDate = calendar.date(byAdding: .year, value: -18, to: Date()) {
                        birthdayDate = defaultDate
                    }
                }
            }
        }
    }

    private func shareWithFriends() {
        let message = """
        Meet Amma, your private personal assistant.

        ✓ $1 USD forever
        ✓ All data stays on your phone
        ✓ Writing, planning, images and documents

        Download here: \(appStoreLink)
        """

        let activityVC = UIActivityViewController(
            activityItems: [message],
            applicationActivities: nil
        )

        // Present the share sheet
        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
           let rootViewController = windowScene.windows.first?.rootViewController {
            // Find the topmost presented view controller
            var topController = rootViewController
            while let presented = topController.presentedViewController {
                topController = presented
            }

            // iPad requires popover presentation
            if let popover = activityVC.popoverPresentationController {
                popover.sourceView = topController.view
                popover.sourceRect = CGRect(x: topController.view.bounds.midX, y: topController.view.bounds.midY, width: 0, height: 0)
                popover.permittedArrowDirections = []
            }

            topController.present(activityVC, animated: true)
        }
    }

    private func requestReview() {
        // Use SKStoreReviewController for broader iOS compatibility
        if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene {
            SKStoreReviewController.requestReview(in: windowScene)
        }
    }
}


// MARK: - Corner Radius Extension
extension View {
    func cornerRadius(_ radius: CGFloat, corners: UIRectCorner) -> some View {
        clipShape(RoundedCorner(radius: radius, corners: corners))
    }
}

struct RoundedCorner: Shape {
    var radius: CGFloat = .infinity
    var corners: UIRectCorner = .allCorners

    func path(in rect: CGRect) -> Path {
        let path = UIBezierPath(
            roundedRect: rect,
            byRoundingCorners: corners,
            cornerRadii: CGSize(width: radius, height: radius)
        )
        return Path(path.cgPath)
    }
}

// MARK: - SwiftUI Preview (for hot reload)
#if DEBUG
struct MainView_Previews: PreviewProvider {
    static var previews: some View {
        MainView()
            .previewDevice("iPhone 15 Pro")
            .previewDisplayName("iPhone 15 Pro")
    }
}

// iOS 17+ Preview macro (faster)
#Preview("Main View") {
    MainView()
}

#Preview("Attachment Menu") {
    AttachmentMenuView(
        isPresented: .constant(true),
        onCamera: {},
        onPhotos: {},
        onFiles: {}
    )
}
#endif

// MARK: - Attachment Menu View (Horizontal layout with icons)
struct AttachmentMenuView: View {
    @Binding var isPresented: Bool
    let onCamera: () -> Void
    let onPhotos: () -> Void
    let onFiles: () -> Void

    var body: some View {
        HStack(spacing: 20) {
            // Camera button
            AttachmentButton(
                icon: "camera",
                title: "Camera",
                action: {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                        isPresented = false
                    }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                        onCamera()
                    }
                }
            )

            // Photos button
            AttachmentButton(
                icon: "photo.on.rectangle",
                title: "Photos",
                action: {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                        isPresented = false
                    }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                        onPhotos()
                    }
                }
            )

            // Files button
            AttachmentButton(
                icon: "paperclip",
                title: "Files",
                action: {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                        isPresented = false
                    }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                        onFiles()
                    }
                }
            )
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 20)
        .background {
            // Frosted glass effect (透明玻璃气泡效果)
            if #available(iOS 15.0, *) {
                RoundedRectangle(cornerRadius: 16)
                    .fill(.ultraThinMaterial)
            } else {
                RoundedRectangle(cornerRadius: 16)
                    .fill(Color(.systemBackground))
                    .opacity(0.95)
            }
        }
        .padding(.horizontal, 20)
        .shadow(color: .black.opacity(0.15), radius: 20, y: -2)
        .scaleEffect(isPresented ? 1.0 : 0.8)
        .opacity(isPresented ? 1.0 : 0.0)
        .transition(.scale.combined(with: .opacity))
        .animation(.spring(response: 0.3, dampingFraction: 0.8), value: isPresented)
    }
}

// MARK: - Attachment Button Component
struct AttachmentButton: View {
    let icon: String
    let title: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 10) {
                Image(systemName: icon)
                    .font(.system(size: 32, weight: .regular))
                    .foregroundColor(.primary)
                    .frame(width: 70, height: 70)
                    .background(Color(.systemGray6))
                    .clipShape(RoundedRectangle(cornerRadius: 14))

                Text(title)
                    .font(.system(size: 15, weight: .regular))
                    .foregroundColor(.primary)
            }
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

// MARK: - Image Viewer (Full Screen)
struct ImageViewer: View {
    let image: UIImage
    @Environment(\.dismiss) private var dismiss
    @State private var scale: CGFloat = 1.0
    @State private var lastScale: CGFloat = 1.0
    @State private var offset: CGSize = .zero
    @State private var lastOffset: CGSize = .zero

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            GeometryReader { geometry in
                Image(uiImage: image)
                    .resizable()
                    .scaledToFit()
                    .scaleEffect(scale)
                    .offset(offset)
                    .gesture(
                        SimultaneousGesture(
                            // Pinch to zoom
                            MagnificationGesture()
                                .onChanged { value in
                                    scale = lastScale * value
                                }
                                .onEnded { _ in
                                    lastScale = scale
                                    // Reset if too small
                                    if scale < 1.0 {
                                        withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                                            scale = 1.0
                                            lastScale = 1.0
                                            offset = .zero
                                            lastOffset = .zero
                                        }
                                    } else if scale > 4.0 {
                                        withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                                            scale = 4.0
                                            lastScale = 4.0
                                        }
                                    }
                                },
                            // Drag to pan (only when zoomed)
                            DragGesture()
                                .onChanged { value in
                                    if scale > 1.0 {
                                        offset = CGSize(
                                            width: lastOffset.width + value.translation.width,
                                            height: lastOffset.height + value.translation.height
                                        )
                                    }
                                }
                                .onEnded { _ in
                                    lastOffset = offset
                                }
                        )
                    )
                    .onTapGesture(count: 2) {
                        // Double tap to zoom
                        withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                            if scale > 1.0 {
                                // Zoom out
                                scale = 1.0
                                lastScale = 1.0
                                offset = .zero
                                lastOffset = .zero
                            } else {
                                // Zoom in
                                scale = 2.0
                                lastScale = 2.0
                            }
                        }
                    }
                    .frame(width: geometry.size.width, height: geometry.size.height)
            }

            // Close button
            VStack {
                HStack {
                    Spacer()
                    Button(action: { dismiss() }) {
                        Image(systemName: "xmark.circle.fill")
                            .font(.system(size: 32))
                            .foregroundColor(.white.opacity(0.9))
                            .background(Color.black.opacity(0.3))
                            .clipShape(Circle())
                    }
                    .padding(20)
                }
                Spacer()
            }
        }
    }
}

// MARK: - Presentation Detents Modifier for iOS Version Compatibility
struct PresentationDetentsModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 16.0, *) {
            content
                .presentationDetents([.height(200)])
                .presentationDragIndicator(.visible)
        } else {
            content
        }
    }
}
