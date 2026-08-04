# Amma Recognize - Android Example

This is the Android example app for the Forge SDK, mirroring the functionality of `example-swift`. It demonstrates how to build a vision-enabled AI chat app using the ForgeAndroid SDK.

## Features

- **Vision Recognition**: Identify plants and animals from photos
- **Streaming Generation**: Real-time token-by-token response display
- **Chat History**: Full session management with search
- **Model Download**: Automatic HuggingFace model download with progress
- **Modern UI**: Material 3 design matching the iOS aesthetic

## Requirements

- Android Studio Ladybug (2024.2.1) or later
- Android SDK 35
- Kotlin 2.0.21+
- Device with ARM64 (arm64-v8a) support

## Project Structure

```
example-android/
├── app/
│   └── src/main/
│       └── java/com/amma/recognize/
│           ├── AmmaApplication.kt      # App initialization
│           ├── MainActivity.kt         # Entry point
│           ├── data/
│           │   └── Models.kt           # Data classes
│           ├── viewmodel/
│           │   └── VisionViewModel.kt  # Main logic (matches iOS)
│           └── ui/
│               ├── theme/
│               │   └── Theme.kt        # Material 3 theme
│               ├── screens/
│               │   └── MainScreen.kt   # Main UI
│               └── components/
│                   ├── WelcomeView.kt
│                   ├── MessageRow.kt
│                   ├── InputBar.kt
│                   ├── SidebarView.kt
│                   ├── LoadingOverlay.kt
│                   └── ...
├── settings.gradle.kts
└── build.gradle.kts
```

## Building

1. **Build native libraries first** (from project root):
   ```bash
   cd forge-rust/scripts
   ./build-android.sh
   ```

2. **Open in Android Studio**:
   - Open `example-android/` as a project
   - Wait for Gradle sync to complete

3. **Run on device**:
   - Connect an ARM64 Android device
   - Click Run

## Model Download

The app automatically downloads the mobile vision model from Hugging Face on first launch:

- **Assistant + vision model**: `LFM2.5-VL-450M-Q4_K_M.gguf` (~229MB)
- **Vision projector**: `mmproj-LFM2.5-VL-450m-Q8_0.gguf` (~103MB)

Models are stored in the app's internal files directory. Downloads are size-checked before activation, and the previous model is removed only after the replacement loads successfully.

## iOS Parity

This app provides feature parity with `example-swift`:

| Feature | iOS | Android |
|---------|-----|---------|
| Vision Recognition | ✅ | ✅ |
| Streaming Responses | ✅ | ✅ |
| Chat Sessions | ✅ | ✅ |
| Session Search | ✅ | ✅ |
| Model Download | ✅ | ✅ |
| Loading Progress | ✅ | ✅ |
| Image Viewer | ✅ | ✅ |
| Settings | ✅ | ✅ |
| Speech Input | ✅ | 🔜 |
| File Attachments | ✅ | 🔜 |

## Architecture

The app follows the same architecture as the iOS version:

- **VisionViewModel**: Manages model loading, chat state, and inference
- **ForgeEngine**: Kotlin wrapper for the Rust inference core
- **Jetpack Compose**: Modern UI framework (equivalent to SwiftUI)

## License

Same license as the main Forge SDK project.
