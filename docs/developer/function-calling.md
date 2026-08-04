# Function Calling & Structured Output

> **Status:** Forge does not currently expose llama.cpp grammar constraints in its Rust, Swift, or Kotlin public APIs. The lower-level `AI` and `ModelAndContextParams` examples below come from the earlier API and do not compile against the current SDK. Today, use prompt-based JSON output and validate it in your application.

Forge SDK allows you to constrain the AI's output to specific formats like JSON using **GBNF (GGML BNF) Grammars**. This is the foundation for reliable function calling and structured output.

---

## What is a Grammar?

A grammar forces the LLM to only output tokens that match a specific pattern. Instead of *hoping* the AI outputs valid JSON, the grammar **guarantees** it.

```
Without grammar: "The weather is... {\"temp\": 72, \"conditions\": sunny}"  ❌ Invalid JSON
With grammar:    {"function": "get_weather", "location": "London"}         ✅ Always valid
```

---

## ⚠️ Important: Using Grammars Requires Lower-Level API

The high-level `ForgeConfig` does not expose `grammarPath`. You need to use the `AI` class directly:

```swift
import Forge

// Use the lower-level AI class for grammar support
let ai = AI(_modelPath: modelPath, _chatName: "function_calling")

var contextParams = ModelAndContextParams()
contextParams.context = 2048
contextParams.use_metal = true
contextParams.grammar_path = Bundle.main.path(forResource: "json", ofType: "gbnf")

ai.initModel(.LLama_gguf, contextParams: contextParams)
```

---

## Step-by-Step: Function Calling

### 1. Create a GBNF Grammar File

Define what valid output looks like. Save as `tools.gbnf`:

```gbnf
# Grammar for function calling
root ::= function-call

function-call ::= "{" ws "\"function\":" ws function-name "," ws "\"arguments\":" ws arguments "}" ws

function-name ::= "\"get_weather\"" | "\"search_web\"" | "\"calculate\""

arguments ::= "{" ws argument-list? "}"

argument-list ::= argument ("," ws argument)*

argument ::= "\"" key "\":" ws value

key ::= [a-z_]+
value ::= string | number

string ::= "\"" [^"]* "\""
number ::= "-"? [0-9]+ ("." [0-9]+)?

ws ::= [ \t\n]*
```

### 2. Set Up the AI with Grammar

```swift
import Forge

class FunctionCallingAgent {
    let ai: AI

    init(modelPath: String, grammarPath: String) {
        ai = AI(_modelPath: modelPath, _chatName: "agent")

        var params = ModelAndContextParams()
        params.context = 4096
        params.use_metal = true
        params.grammar_path = grammarPath  // Key: set grammar here

        ai.initModel(.LLama_gguf, contextParams: params)
    }

    func loadModel() async throws {
        try ai.loadModel_sync()
    }

    func call(prompt: String, systemPrompt: String) async -> String? {
        return await withCheckedContinuation { continuation in
            var result = ""

            ai.conversation(prompt, { token, _ in
                result += token
            }, nil, { output in
                continuation.resume(returning: output)
            }, system_prompt: systemPrompt, img_path: nil)
        }
    }
}
```

### 3. Implement the Function Loop

```swift
// Define your tools
struct ToolCall: Codable {
    let function: String
    let arguments: [String: String]
}

func runAgent(query: String) async {
    let agent = FunctionCallingAgent(
        modelPath: "/path/to/model.gguf",
        grammarPath: Bundle.main.path(forResource: "tools", ofType: "gbnf")!
    )

    try? await agent.loadModel()

    let systemPrompt = """
    You are an assistant with access to these functions:
    - get_weather(location): Get current weather for a city
    - search_web(query): Search the internet
    - calculate(expression): Evaluate a math expression

    Respond with a function call in JSON format.
    """

    // Get structured output from LLM
    guard let output = await agent.call(prompt: query, systemPrompt: systemPrompt),
          let data = output.data(using: .utf8),
          let toolCall = try? JSONDecoder().decode(ToolCall.self, from: data) else {
        print("Failed to parse function call")
        return
    }

    // Execute the function
    let result: String
    switch toolCall.function {
    case "get_weather":
        let location = toolCall.arguments["location"] ?? "Unknown"
        result = await fetchWeather(location: location)

    case "search_web":
        let query = toolCall.arguments["query"] ?? ""
        result = await searchWeb(query: query)

    case "calculate":
        let expr = toolCall.arguments["expression"] ?? "0"
        result = evaluate(expression: expr)

    default:
        result = "Unknown function"
    }

    print("Function: \(toolCall.function)")
    print("Result: \(result)")
}
```

---

## Pre-Built Grammars

The llama.cpp library includes grammar files in `Forge/llama.cpp/grammars/`:

| File | Purpose |
|------|---------|
| `json.gbnf` | Valid JSON objects |
| `json_arr.gbnf` | JSON arrays |
| `list.gbnf` | Bulleted lists |
| `arithmetic.gbnf` | Math expressions |
| `chess.gbnf` | Chess notation |

See the [llama.cpp grammars README](https://github.com/ggerganov/llama.cpp/tree/master/grammars) for more examples.

---

## JSON Schema → Grammar Conversion

Writing GBNF manually is tedious. Use the llama.cpp converter:

```bash
# Clone llama.cpp if you haven't
git clone https://github.com/ggerganov/llama.cpp

# Convert a JSON Schema to GBNF
python3 llama.cpp/examples/json_schema_to_grammar.py schema.json > output.gbnf
```

Example JSON Schema:

```json
{
  "type": "object",
  "properties": {
    "function": { "enum": ["get_weather", "search"] },
    "location": { "type": "string" }
  },
  "required": ["function", "location"]
}
```

---

## Alternative: Native Tool Calling (No Grammar)

Modern models like **Llama 3.1+**, **Qwen 2.5**, and **Mistral** have built-in tool calling. Instead of grammars, use their prompt format:

### Llama 3.1+ Format

```swift
let systemPrompt = """
<|begin_of_text|><|start_header_id|>system<|end_header_id|>

You have access to the following tools:

{"type": "function", "function": {"name": "get_weather", "description": "Get weather for a location", "parameters": {"type": "object", "properties": {"location": {"type": "string"}}, "required": ["location"]}}}

When you need to use a tool, respond with:
<|python_tag|>{"name": "tool_name", "parameters": {...}}
<|eom_id|>
"""
```

### Qwen 2.5 Format

```swift
let systemPrompt = """
You are a helpful assistant with access to tools.

# Tools
You can call the following tools:

```json
{"name": "get_weather", "description": "Get weather", "parameters": {"location": "string"}}
```

To use a tool, respond with:
<tool_call>
{"name": "get_weather", "arguments": {"location": "London"}}
</tool_call>
"""
```

---

## Best Practices

| Do | Don't |
|----|-------|
| Use grammars for guaranteed structure | Rely on prompt engineering alone |
| Use smaller context with tool calls | Use huge contexts for simple tasks |
| Parse output immediately after generation | Assume output is always valid |
| Use native tool calling for supported models | Force grammars on all models |

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Output doesn't match grammar | Check GBNF syntax; use a simpler grammar first |
| Generation is slow | Grammars add overhead; reduce `contextSize` |
| Model ignores function format | Use a model fine-tuned for tool calling |
| Parse errors | Ensure grammar covers all edge cases (empty strings, special chars) |
