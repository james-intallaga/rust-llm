# Multimodal Support

Forge SDK supports multimodal models that can process images and audio alongside text.

---

## Vision Models

Vision-capable models can analyze images and answer questions about them.

### Requirements

Vision models require **two files**:

| File | Description | Example |
|------|-------------|---------|
| **Main Model** | The LLM weights in GGUF format | `LFM2.5-VL-1.6B-Q8_0.gguf` |
| **CLIP Projector** | Vision encoder (mmproj) | `mmproj-LFM2.5-VL-1.6b-Q8_0.gguf` |

### Recommended Vision Models

| Model | Main Model | CLIP Projector | Size | Notes |
|-------|------------|----------------|------|-------|
| [LFM2.5-VL 1.6B](https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF) | `LFM2.5-VL-1.6B-Q8_0.gguf` | `mmproj-LFM2.5-VL-1.6b-Q8_0.gguf` | ~1.6GB | ✅ Recommended, mobile optimized |
| [Qwen2.5-VL 7B](https://huggingface.co/Qwen/Qwen2.5-VL-7B-Instruct-GGUF) | `qwen2.5-vl-7b-*.gguf` | Included | ~5GB | High quality |
| [LLaVA 1.5 7B](https://huggingface.co/mys/ggml_llava-v1.5-7b) | `ggml-model-*.gguf` | `mmproj-model-f16.gguf` | ~4GB | Classic |

### Vision Setup

```swift
import ForgeSwift

// Initialize
initializeForge()

// Create vision engine
let engine = try ForgeEngine(
    modelPath: "/path/to/LFM2.5-VL-1.6B-Q8_0.gguf",
    clipPath: "/path/to/mmproj-LFM2.5-VL-1.6b-Q8_0.gguf",
    config: .vision()  // 4096 context, 512 batch
)

// Load image as JPEG/PNG data
let image = UIImage(named: "photo")!
let imageData = image.jpegData(compressionQuality: 0.8)!

// Format prompt with image markers (model-specific)
let prompt = """
<|im_start|>system
You are a helpful assistant with vision capabilities.
<|im_end|><|im_start|>user
<|image_start|><|image_end|>What is in this image?
<|im_end|><|im_start|>assistant
"""

// Generate with streaming
for await token in engine.generateVisionStream(imageData: imageData, prompt: prompt) {
    print(token, terminator: "")
}
```

### Image Format Tips

- **JPEG** for photos (smaller, faster)
- **PNG** for screenshots/diagrams (lossless)
- Images are automatically decoded and resized in the Rust core
- Maximum recommended: 1024x1024 (larger images work but take longer)

---

## Audio Models

Audio models can process speech input and generate text or speech output.

### Requirements

Audio models require:

| File | Description | Example |
|------|-------------|---------|
| **Main Model** | The audio-capable LLM | `LFM2.5-Audio-1.5B.gguf` |
| **Audio Encoder** | Processes audio input | `audio-encoder.gguf` |
| **Vocoder** (optional) | For audio output | `vocoder.gguf` |

### Recommended Audio Models

| Model | Provider | Size | Capabilities |
|-------|----------|------|--------------|
| [LFM2.5-Audio 1.5B](https://huggingface.co/LiquidAI/LFM2.5-Audio-1.5B-GGUF) | Liquid AI | ~1.5GB | ✅ ASR, TTS, speech-to-speech |
| [Qwen2-Audio](https://huggingface.co/Qwen/Qwen2-Audio-GGUF) | Alibaba | ~3GB | ASR, audio understanding |
| [Ultravox](https://huggingface.co/fixie-ai/ultravox-v0_4-llama-3_1-8b) | Fixie.ai | ~5GB | Speech-to-speech |

### Audio Input

```swift
import ForgeSwift
import AVFoundation

// Create engine with audio model
let engine = try ForgeEngine(
    modelPath: "/path/to/LFM2.5-Audio.gguf",
    clipPath: "/path/to/audio-encoder.gguf",
    config: .auto()
)

// Check audio support
guard engine.supportsAudio else {
    print("Model doesn't support audio")
    return
}

// Capture audio as PCM f32 samples (-1.0 to 1.0)
let samples: [Float] = captureAudioFromMicrophone()

// Generate text from audio
let prompt = "<|audio|>Transcribe this audio.<|im_end|><|im_start|>assistant\n"
for await token in engine.generateAudioStream(samples: samples, prompt: prompt) {
    print(token, terminator: "")
}
```

### Audio Output (Vocoder)

For speech-to-speech models, use the audio decoder to convert embeddings to audio:

```swift
import ForgeSwift

// Create vocoder with LFM2.5-Audio parameters
let decoder = try ForgeAudioDecoder()

// Process embeddings from the model
while let embeddings = getNextEmbeddingsFromModel() {
    let samples = try decoder.process(embeddings: embeddings)
    audioPlayer.enqueue(samples)
}

// Flush remaining audio
let finalSamples = decoder.flush()
audioPlayer.enqueue(finalSamples)
```

---

## Model Support Matrix

| Projector Type | Vision | Audio | Audio Output | Models |
|----------------|--------|-------|--------------|--------|
| `lfm2` | ✅ | ❌ | ❌ | LFM2-VL |
| `lfm2a` | ❌ | ✅ | ✅ | LFM2.5-Audio |
| `qwen2a` | ❌ | ✅ | ❌ | Qwen2-Audio |
| `qwen2vl` | ✅ | ❌ | ❌ | Qwen2-VL |
| `qwen25vl` | ✅ | ❌ | ❌ | Qwen2.5-VL |
| `qwen3vl` | ✅ | ❌ | ❌ | Qwen3-VL |
| `ultravox` | ❌ | ✅ | ❌ | Ultravox |
| `voxtral` | ❌ | ✅ | ❌ | Voxtral |
| `glma` | ❌ | ✅ | ❌ | GLM-Audio |
| `gemma3` | ✅ | ❌ | ❌ | Gemma3 |
| `llama4` | ✅ | ❌ | ❌ | LLaMA4 |

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
<|vision_start|><|image_pad|><|vision_end|>Describe this image.
<|im_end|><|im_start|>assistant
```

### LLaVA

```
USER: <image>
What's in this image?
ASSISTANT:
```

---

## Pending Model Support

The following models are **not yet supported** in llama.cpp:

| Model | Status | Notes |
|-------|--------|-------|
| **Fun-Audio-Chat-8B** | ❌ Pending | No GGUF conversion yet |
| **Qwen3-Omni** | ❌ Pending | MoE architecture, complex |

Once llama.cpp adds support, Forge SDK will automatically support them through the existing multimodal API.

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `ForgeError.multimodalLoadFailed` | Check CLIP/encoder path is correct |
| Blank output from images | Ensure prompt format matches the model |
| Audio not working | Check `engine.supportsAudio` returns true |
| Slow image processing | Use `.vision()` config (512 batch) |

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
