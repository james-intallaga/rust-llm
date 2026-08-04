# Advanced Multimodal Pipelines

Forge SDK provides a sophisticated pipeline for handling multimodal inputs like images and (coming soon) audio. Understanding how these inputs are processed can help you optimize your application's performance.

## The Inference Pipeline

When you call `generate()` with multiple inputs, the SDK follows a specific evaluation order to build the KV (Key-Value) cache:

1.  **System Prompt**: Evaluated first to set the context.
2.  **Image Embeddings**: If an image is provided, the CLIP model generates embeddings which are then "injected" into the context.
3.  **User Prompt**: The text query is evaluated last.
4.  **Prediction Loop**: The model begins generating tokens based on the combined state.

### Vision Pipeline Internals

The vision pipeline in `LLaMa_MModal.swift` is designed for memory efficiency:

-   **On-Demand CLIP**: The CLIP model is loaded only when an image needs processing and is freed immediately after the embeddings are evaluated into the KV cache. This prevents the large CLIP model from occupying RAM during the text generation phase.
-   **Image Preprocessing**: Images are automatically resized to a maximum dimension of 1024px while maintaining aspect ratio. This ensures compatibility with most CLIP models while keeping memory usage low.

```swift
// Internal snippet from LLMBase._eval_img
try ExceptionCather.catchException {
    _ = self.load_clip_model()      // 1. Load CLIP model
    _ = self.make_image_embed(path) // 2. Preprocess & Embed
    _ = try? self.llm_eval_clip()   // 3. Evaluate into KV Cache
    self.deinit_clip_model()        // 4. Free CLIP memory
}
```

## Audio Pipeline (Speech-to-Text)

Currently, the recommended way to handle audio in the Amma Forge ecosystem is using the native **Apple Speech Framework** (`SFSpeechRecognizer`). This provides the best latency and power efficiency on iOS.

### Example: Voice-to-Chat Pipeline

1.  Capture audio using `AVAudioEngine`.
2.  Stream audio buffers to `SFSpeechAudioBufferRecognitionRequest`.
3.  On receiving the final transcript, pass it to `ForgeSDK`.

```swift
// See AmmaRecognize/Services/SpeechRecognizer.swift for implementation
let recognizer = SpeechRecognizer()
recognizer.start()

// ... on transcript received ...
let response = await forge.generate(prompt: recognizer.transcript)
```

*Note: Native Whisper.cpp integration within the Forge pipeline is currently in the research phase.*
