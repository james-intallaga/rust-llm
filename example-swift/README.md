# AmmaRecognize Example App

A full-featured iOS app demonstrating the **ForgeSwift** SDK for on-device AI inference.

## Features

- ✅ Multi-turn conversations with context
- ✅ Image recognition (camera + photo library)
- ✅ Speech-to-text input
- ✅ Document processing (PDF, DOCX)
- ✅ Chat session management

## Architecture

```
┌─────────────────────────────────────────────────┐
│              AmmaRecognize App                  │
├─────────────────────────────────────────────────┤
│                VisionViewModel                  │
│        (Business logic, state management)       │
├─────────────────────────────────────────────────┤
│                 ForgeSwift                      │
│       (Swift API with device auto-detection)   │
├─────────────────────────────────────────────────┤
│                  forge-core                     │
│      (Rust: RAII wrappers, sampler chain)       │
├─────────────────────────────────────────────────┤
│               llama.cpp (C/C++)                 │
│        (Metal GPU, model loading, KV cache)     │
└─────────────────────────────────────────────────┘
```

## Building

1. Generate Xcode project:
   ```bash
   cd example-swift
   xcodegen generate
   ```

2. Open in Xcode:
   ```bash
   open AmmaRecognize-Rust.xcodeproj
   ```

3. Build and run on device or simulator

## Project Structure

```
example-swift/
├── Sources/AmmaRecognize/
│   ├── AmmaRecognizeApp.swift      # Entry point
│   ├── ViewModels/
│   │   └── VisionViewModel.swift   # ForgeEngine integration
│   ├── Views/
│   │   └── MainView.swift          # UI
│   └── Models/
│       ├── DocumentProcessor.swift # PDF/DOCX extraction
│       └── SpeechRecognizer.swift  # Speech-to-text
├── project.yml                     # XcodeGen config
└── README.md
```

## Model Download

The app selects models for the device automatically:

- **iPhone and iPad**: `LFM2.5-VL-450M-Q4_K_M` plus its vision projector (~332MB total)
- **Mac with at least 12GB RAM**: `LFM2.5-8B-A1B-Q4_K_M` for conversation, with the 450M model retained for image requests
- **Lower-memory Mac**: the 450M model is used as a safe fallback

Every download is size-checked before activation. Existing working models remain in place until their replacement loads successfully.

You can also manually place models in the app's Documents folder.

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
