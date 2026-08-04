# RAG (Retrieval-Augmented Generation)

> **Status:** RAG and `SimilaritySearchKit` are not bundled with this repository. This page is an integration pattern for applications that add a separate embeddings and vector-search dependency; it is not a built-in Forge API.

RAG lets your AI access local documents without sending data to the cloud. Forge SDK includes `SimilaritySearchKit` for on-device vector search.

---

## How RAG Works

```
┌──────────────────────────────────────────────────────────────┐
│ 1. INDEXING (One-time setup)                                 │
│    Documents → Text Chunks → Embeddings → Vector Database    │
└──────────────────────────────────────────────────────────────┘
                              ▼
┌──────────────────────────────────────────────────────────────┐
│ 2. RETRIEVAL (Each query)                                    │
│    User Question → Search Index → Top K Relevant Chunks      │
└──────────────────────────────────────────────────────────────┘
                              ▼
┌──────────────────────────────────────────────────────────────┐
│ 3. GENERATION                                                │
│    Context + Question → LLM → Answer Based on Documents      │
└──────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### 1. Create an Index

```swift
import SimilaritySearchKit
import SimilaritySearchKitDistilbert  // or MiniLM

// Initialize embedding model and index
let embeddings = DistilbertEmbeddings()  // Runs on CoreML
let index = await SimilarityIndex(
    name: "my_knowledge_base",
    model: embeddings,
    metric: CosineSimilarity()
)
```

### 2. Add Documents

```swift
// Add individual items
await index.addItem(
    id: "doc1",
    text: "Forge SDK supports LLaMA, Qwen, Phi, and Mistral models.",
    metadata: ["source": "docs/models.md"]
)

// Add multiple items
await index.addItems(
    ids: ["doc2", "doc3"],
    texts: [
        "Metal GPU acceleration is available on Apple Silicon.",
        "Context size affects memory usage and inference speed."
    ],
    metadata: [
        ["topic": "performance"],
        ["topic": "configuration"]
    ]
)
```

### 3. Search and Generate

```swift
// Search for relevant context
let query = "Which GPUs are supported?"
let results = await index.search(query, top: 3)

// Build augmented prompt
let context = results.map { "- \($0.text)" }.joined(separator: "\n")
let augmentedPrompt = """
Answer based on the following context:

\(context)

Question: \(query)
Answer:
"""

// Generate with context
let response = await forge.generate(prompt: augmentedPrompt)
```

---

## Complete Example

```swift
import Forge
import SimilaritySearchKit
import SimilaritySearchKitMiniLM

class RAGChatBot {
    let forge: ForgeSDK
    let index: SimilarityIndex

    init(modelPath: String) async throws {
        // Initialize LLM
        forge = ForgeSDK(modelPath: modelPath, config: .chat)
        try await forge.loadModel()

        // Initialize vector index
        let embeddings = MultiQAMiniLMEmbeddings()
        index = await SimilarityIndex(
            name: "chat_kb",
            model: embeddings,
            metric: CosineSimilarity()
        )
    }

    func indexDocuments(texts: [String], sources: [String]) async {
        for (i, text) in texts.enumerated() {
            // Split long texts into chunks
            let chunks = splitText(text, maxLength: 500)

            for (j, chunk) in chunks.enumerated() {
                await index.addItem(
                    id: "\(sources[i])_\(j)",
                    text: chunk,
                    metadata: ["source": sources[i]]
                )
            }
        }
    }

    func answer(question: String) async -> String {
        // Retrieve relevant context
        let results = await index.search(question, top: 3)

        guard !results.isEmpty else {
            return await forge.generate(prompt: question)
        }

        // Build prompt with context
        let context = results.map { result in
            "[\(result.metadata?["source"] ?? "unknown")]: \(result.text)"
        }.joined(separator: "\n\n")

        let prompt = """
        Use ONLY the following context to answer. If the answer isn't in the context, say "I don't know."

        Context:
        \(context)

        Question: \(question)

        Answer:
        """

        return await forge.generate(prompt: prompt)
    }

    func saveIndex(to url: URL) throws {
        try index.saveIndex(toDirectory: url, name: "chat_kb")
    }

    func loadIndex(from url: URL) throws {
        _ = try index.loadIndex(fromDirectory: url, name: "chat_kb")
    }

    private func splitText(_ text: String, maxLength: Int) -> [String] {
        var chunks: [String] = []
        var current = ""

        for sentence in text.components(separatedBy: ". ") {
            if current.count + sentence.count > maxLength {
                if !current.isEmpty {
                    chunks.append(current.trimmingCharacters(in: .whitespaces))
                }
                current = sentence
            } else {
                current += (current.isEmpty ? "" : ". ") + sentence
            }
        }

        if !current.isEmpty {
            chunks.append(current.trimmingCharacters(in: .whitespaces))
        }

        return chunks
    }
}
```

---

## Embedding Models

| Model | Import | Size | Speed | Quality |
|-------|--------|------|-------|---------|
| `DistilbertEmbeddings` | `SimilaritySearchKitDistilbert` | ~250MB | Medium | High |
| `MiniLMEmbeddings` | `SimilaritySearchKitMiniLM` | ~80MB | Fast | Good |
| `MultiQAMiniLMEmbeddings` | `SimilaritySearchKitMiniLM` | ~80MB | Fast | Best for Q&A |

All models run on **CoreML** for fast, on-device inference.

---

## Similarity Metrics

| Metric | When to Use |
|--------|-------------|
| `CosineSimilarity()` | Default choice, works well for most cases |
| `DotProduct()` | Faster, requires normalized embeddings |
| `EuclideanDistance()` | When absolute distances matter |

---

## Persistence

```swift
// Save index to disk
let documentsURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
let indexURL = try index.saveIndex(toDirectory: documentsURL, name: "my_index")
print("Saved to: \(indexURL)")

// Load existing index
if let items = try? index.loadIndex(fromDirectory: documentsURL, name: "my_index") {
    print("Loaded \(items.count) items")
}
```

---

## Tips

| Tip | Why |
|-----|-----|
| Chunk documents into 300-500 token pieces | Better retrieval accuracy |
| Use `MultiQAMiniLMEmbeddings` for Q&A | Optimized for question-answering |
| Store metadata (source, page, date) | Helps with citation and filtering |
| Limit context to 3-5 results | More isn't always better |
| Tell the LLM to only use provided context | Prevents hallucination |

---

## Reference Implementation

For a production-ready RAG implementation, see the `SimilaritySearchKit` examples in the `Tests/similarity-search-kit/Examples/` folder.
