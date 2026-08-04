# Lesson 09: Multimodal & Vision

Understanding vision-language models, audio models, and multimodal processing.

---

## What is Multimodal?

Multimodal models can process multiple types of input:
- **Text** - Natural language
- **Images** - Photos, screenshots, diagrams
- **Audio** - Speech, sounds

The Forge SDK supports all three modalities through a unified architecture.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    MULTIMODAL MODEL                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐     ┌─────────────────────┐                    │
│  │   Image     │     │   Vision Encoder    │                    │
│  │   Input     │ ──▶ │   (CLIP/SigLIP)     │                    │
│  │  (pixels)   │     │                     │                    │
│  └─────────────┘     └──────────┬──────────┘                    │
│                                 │                                │
│  ┌─────────────┐     ┌──────────▼──────────┐                    │
│  │   Audio     │     │   Audio Encoder     │                    │
│  │   Input     │ ──▶ │   (FastConformer)   │                    │
│  │  (samples)  │     │                     │                    │
│  └─────────────┘     └──────────┬──────────┘                    │
│                                 │                                │
│                      ┌──────────▼──────────┐                    │
│                      │     Projector       │                    │
│                      │  media → text space │                    │
│                      └──────────┬──────────┘                    │
│                                 │                                │
│  ┌─────────────┐     ┌──────────▼──────────┐                    │
│  │   Text      │     │   Embedding Space   │                    │
│  │   Input     │ ──▶ │ [media][text]       │                    │
│  └─────────────┘     └──────────┬──────────┘                    │
│                                 │                                │
│                      ┌──────────▼──────────┐                    │
│                      │    Language Model   │                    │
│                      │   (LLM Backbone)    │                    │
│                      └──────────┬──────────┘                    │
│                                 │                                │
│                      ┌──────────▼──────────┐                    │
│                      │   Text / Audio Out  │                    │
│                      └─────────────────────┘                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Vision Models

### Model Files

Vision models require two GGUF files:

| File | Description | Example |
|------|-------------|---------|
| **Main Model** | The LLM weights | `LFM2.5-VL-1.6B-Q8_0.gguf` |
| **Projector** | Vision encoder | `mmproj-LFM2.5-VL-1.6b-Q8_0.gguf` |

### Vision Setup

```swift
import ForgeSwift

initializeForge()

// Create vision engine
let engine = try ForgeEngine(
    modelPath: "/path/to/LFM2.5-VL-1.6B-Q8_0.gguf",
    clipPath: "/path/to/mmproj-LFM2.5-VL-1.6b-Q8_0.gguf",
    config: .vision()
)
```

### Image Processing

The Rust core handles image decoding internally:

```swift
// Load image as JPEG/PNG data
let image = UIImage(named: "photo")!
let imageData = image.jpegData(compressionQuality: 0.8)!

// The SDK automatically:
// 1. Decodes JPEG/PNG to pixels
// 2. Resizes if needed
// 3. Processes through vision encoder
// 4. Injects embeddings into context
```

### Vision Generation

```swift
let prompt = """
<|im_start|>system
You are a helpful assistant.
<|im_end|><|im_start|>user
<|image_start|><|image_end|>What is in this image?
<|im_end|><|im_start|>assistant
"""

for await token in engine.generateVisionStream(imageData: imageData, prompt: prompt) {
    print(token, terminator: "")
}
```

---

## Audio Models

### Model Files

Audio models require:

| File | Description | Example |
|------|-------------|---------|
| **Main Model** | Audio-capable LLM | `LFM2.5-Audio-1.5B.gguf` |
| **Encoder** | Audio encoder | `audio-encoder.gguf` |
| **Vocoder** | For audio output | `vocoder.gguf` (optional) |

### Audio Input

```swift
// Check if model supports audio
guard engine.supportsAudio else {
    print("Model doesn't support audio")
    return
}

// Get expected sample rate
let sampleRate = engine.audioSampleRate ?? 16000

// Capture PCM samples (f32, -1.0 to 1.0)
let samples: [Float] = captureAudioFromMicrophone(sampleRate: sampleRate)

// Generate text from audio
let prompt = "<|audio|>Transcribe this.<|im_end|><|im_start|>assistant\n"

for await token in engine.generateAudioStream(samples: samples, prompt: prompt) {
    print(token, terminator: "")
}
```

### Audio Output (Vocoder)

For speech-to-speech models:

```swift
// Create vocoder
let decoder = try ForgeAudioDecoder()

print("Sample rate: \(decoder.sampleRate) Hz")
print("Embedding size: \(decoder.embeddingSize)")

// Process embeddings from model
while let embeddings = getNextEmbeddings() {
    let samples = try decoder.process(embeddings: embeddings)
    audioPlayer.enqueue(samples)
}

// Flush remaining audio
let finalSamples = decoder.flush()
audioPlayer.enqueue(finalSamples)
```

---

## Prompt Formats

Different models use different markers for media content.

### LFM2.5-VL (Liquid AI)

```
<|im_start|>user
<|image_start|><|image_end|>What is this?
<|im_end|><|im_start|>assistant
```

### LFM2.5-Audio (Liquid AI)

```
<|im_start|>user
<|audio|>Transcribe this.
<|im_end|><|im_start|>assistant
```

### Qwen2.5-VL

```
<|im_start|>user
<|vision_start|><|image_pad|><|vision_end|>Describe this.
<|im_end|><|im_start|>assistant
```

### LLaVA

```
USER: <image>
What's in this image?
ASSISTANT:
```

---

## Image Considerations

### Supported Formats

| Format | Extension | Notes |
|--------|-----------|-------|
| JPEG | .jpg, .jpeg | Recommended for photos |
| PNG | .png | Good for screenshots |
| WebP | .webp | Efficient compression |

### Resolution and Tokens

Images are converted to tokens. More pixels = more tokens = more context used.

| Image Size | Approximate Tokens |
|------------|-------------------|
| 384×384 | ~256 tokens |
| 768×768 | ~576-1024 tokens |
| 1024×1024 | ~1024-2000 tokens |

### Best Practices

1. **Resize large images** before sending (768px max recommended)
2. **Use JPEG** for photos (smaller, faster)
3. **Use PNG** for text/diagrams (preserves detail)
4. **Use `.vision()` config** (4096 context minimum)

```swift
func prepareImage(_ image: UIImage) -> Data? {
    let maxDimension: CGFloat = 768
    let size = image.size

    if max(size.width, size.height) > maxDimension {
        let scale = maxDimension / max(size.width, size.height)
        let newSize = CGSize(width: size.width * scale,
                            height: size.height * scale)

        UIGraphicsBeginImageContextWithOptions(newSize, false, 1.0)
        image.draw(in: CGRect(origin: .zero, size: newSize))
        let resized = UIGraphicsGetImageFromCurrentImageContext()
        UIGraphicsEndImageContext()

        return resized?.jpegData(compressionQuality: 0.85)
    }

    return image.jpegData(compressionQuality: 0.85)
}
```

---

## Audio Considerations

### Sample Format

The SDK expects:
- **PCM float32** samples
- **Normalized** to -1.0 to 1.0
- **Mono** channel (most models)
- **16kHz** sample rate (model-specific)

### Capturing Audio

```swift
import AVFoundation

class AudioCapture {
    let engine = AVAudioEngine()
    var samples: [Float] = []

    func start(sampleRate: Double = 16000) {
        let input = engine.inputNode
        let format = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: sampleRate,
            channels: 1,
            interleaved: false
        )!

        input.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in
            let ptr = buffer.floatChannelData![0]
            let count = Int(buffer.frameLength)
            self.samples.append(contentsOf: UnsafeBufferPointer(start: ptr, count: count))
        }

        try? engine.start()
    }

    func stop() -> [Float] {
        engine.stop()
        engine.inputNode.removeTap(onBus: 0)
        return samples
    }
}
```

---

## Supported Models

| Model | Vision | Audio | Audio Out |
|-------|--------|-------|-----------|
| LFM2.5-VL | ✅ | ❌ | ❌ |
| LFM2.5-Audio | ❌ | ✅ | ✅ |
| Qwen2.5-VL | ✅ | ❌ | ❌ |
| Qwen2-Audio | ❌ | ✅ | ❌ |
| LLaVA | ✅ | ❌ | ❌ |
| Ultravox | ❌ | ✅ | ❌ |
| Voxtral | ❌ | ✅ | ❌ |

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `multimodalLoadFailed` | Check projector/encoder path |
| Blank output from images | Check prompt format matches model |
| Audio not recognized | Check sample rate matches model |
| Out of memory | Use smaller model or reduce image size |

---

## Key Takeaways

1. **Two files** needed for multimodal (main + encoder)
2. **Prompt format** is model-specific
3. **Images become tokens** (~256-2000 per image)
4. **Audio needs normalization** (PCM f32, -1 to 1)
5. **Vocoder** converts embeddings back to audio
6. Use **`.vision()` config** for vision models

---

## Next Lesson

👉 **[Lesson 10: Performance Optimization →](./10-performance-optimization.md)**
