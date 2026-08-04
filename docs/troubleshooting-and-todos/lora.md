# LoRA Adapters

> **Status:** The current ForgeSwift and ForgeAndroid public APIs do not load LoRA adapters or fine-tune models. The examples below are retained as design notes from the earlier LLM Farm API and will not compile against this SDK. Merge an adapter into a GGUF model before using it with Forge today.

## Connecting LoRA Adapters

Put adapter files in the `lora_adapters` directory.

**Note:** You cannot use mmap when connecting an adapter, so memory consumption will be higher. To use mmap with LoRA, export the adapter merged into the model.

### Single Adapter

```swift
var params = ModelAndContextParams()
params.lora_adapters = [("/path/to/adapter.bin", 1.0)]
```

### Multiple Adapters

```swift
var params = ModelAndContextParams()
params.lora_adapters = [
    ("/path/to/adapter1.bin", 0.9),
    ("/path/to/adapter2.bin", 1.0)
]
```

Or configure via JSON:

```json
{
    "prompt_format": "{{prompt}}",
    "model": "model.gguf",
    "lora_adapters": [
        {
            "adapter": "adapter1.bin",
            "scale": 0.9
        },
        {
            "adapter": "adapter2.bin",
            "scale": 1.0
        }
    ]
}
```

## Fine-Tuning

Fine-tuning can be performed using the `FineTune` class:

```swift
let fineTune = FineTune(
    "/path/to/base-model.gguf",
    "/path/to/output-lora.bin",
    "/path/to/training-data.txt",
    threads: 8,
    adam_iter: 30,
    batch: 4,
    ctx: 64
)

fineTune.Run_Train()
```

**Note:** Fine-tuning consumes significant RAM. On iOS, only 3B models with minimum context/batch settings are practical.

See [llama.cpp finetune documentation](https://github.com/ggerganov/llama.cpp/tree/master/examples/finetune) for more details.

## Export LoRA as Model

Merge a LoRA adapter into the base model:

```swift
fineTune.Run_Export()
```

For iOS devices, merge with Q4_K models or lower quantization for best compatibility.

See [llama.cpp export-lora documentation](https://github.com/ggerganov/llama.cpp/tree/master/examples/export-lora) for details.
