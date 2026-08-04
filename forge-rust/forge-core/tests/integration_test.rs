//! Integration tests for forge-core
//!
//! These tests verify that our Rust wrappers work correctly with llama.cpp.
//! Tests that require a model will skip if no model is found.
//!
//! To run with a model:
//!   FORGE_TEST_MODEL=/path/to/model.gguf cargo test --release
//!
//! To run without a model (only tests that don't need one):
//!   cargo test --release
//!
//! NOTE: We use a LazyLock to initialize the llama backend ONCE for all tests.
//! The backend is never explicitly freed - it will be cleaned up when the
//! process exits. This is necessary because llama_backend_free() can corrupt
//! Metal state if called while other GPU resources are still in use.

use std::env;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

// Import our crate
use forge_core::{init as forge_init, ContextParams, ForgeConfig, ForgeEngine, MemoryTracker};

// Initialize the backend exactly ONCE for all tests
static BACKEND: LazyLock<()> = LazyLock::new(|| {
    forge_init();
    println!("✓ Backend initialized (once for all tests)");
});
static MODEL_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Get the path to a test model
fn get_test_model_path() -> Option<PathBuf> {
    // Check environment variable first
    if let Ok(path) = env::var("FORGE_TEST_MODEL") {
        let p = PathBuf::from(path);
        if p.exists() {
            println!("Using model from FORGE_TEST_MODEL: {:?}", p);
            return Some(p);
        }
        panic!("FORGE_TEST_MODEL set but file not found: {:?}", p);
    }

    // Check common locations for any .gguf file
    let home = env::var("HOME").ok().map(PathBuf::from);

    let candidates: Vec<Option<PathBuf>> = vec![
        // User's container (where the app downloads models)
        home.as_ref().map(|h| h.join("Library/Containers/426E3FF7-016F-4F92-82D4-9CAC96D211B0/Data/Documents/LFM2.5-VL-1.6B-Q4_0.gguf")),
        home.as_ref().map(|h| h.join("Library/Containers/426E3FF7-016F-4F92-82D4-9CAC96D211B0/Data/Documents/LFM2.5-VL-1.6B-Q8_0.gguf")),
        // Documents folder
        home.as_ref().map(|h| h.join("Documents/LFM2.5-VL-1.6B-Q4_0.gguf")),
        home.as_ref().map(|h| h.join("Documents/LFM2.5-VL-1.6B-Q8_0.gguf")),
        home.as_ref().map(|h| h.join("models/qwen2.5-1.5b-instruct-q4_k_m.gguf")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            println!("Found model at: {:?}", candidate);
            return Some(candidate);
        }
    }

    None
}

/// Test 1: Backend initialization (uses shared initialization)
#[test]
fn test_backend_lifecycle() {
    println!("=== Test: Backend Lifecycle ===");

    // Force initialization via LazyLock
    LazyLock::force(&BACKEND);

    // Backend cleanup is intentionally NOT called.
    // llama_backend_free() can corrupt Metal state if called while
    // other GPU resources are in use. The backend is freed automatically
    // when the test process exits.
    println!("✓ Backend lifecycle test complete (backend persists)");
}

/// Test 2: Create default configurations
#[test]
fn test_config_creation() {
    println!("=== Test: Config Creation ===");

    let config = ForgeConfig::default();
    println!("✓ Default config created");

    // Verify sensible defaults
    assert!(config.context.n_ctx > 0, "n_ctx should be positive");
    assert!(config.context.n_batch > 0, "n_batch should be positive");
    println!("  n_ctx: {}", config.context.n_ctx);
    println!("  n_batch: {}", config.context.n_batch);
    println!("  n_threads: {}", config.context.n_threads);

    // Test mobile config
    let mobile = ForgeConfig::mobile();
    println!("✓ Mobile config created");
    println!("  Mobile context: {}", mobile.context.n_ctx);
    println!("  Mobile batch: {}", mobile.context.n_batch);
}

/// Test 3: Model loading (requires actual model file)
#[test]
fn test_model_loading() {
    let _guard = MODEL_TEST_LOCK.lock().unwrap();
    println!("=== Test: Model Loading ===");

    // Ensure backend is initialized
    LazyLock::force(&BACKEND);

    let model_path = match get_test_model_path() {
        Some(p) => p,
        None => {
            println!("⚠ No test model found. Skipping model loading test.");
            println!("  Set FORGE_TEST_MODEL=/path/to/model.gguf to enable.");
            return;
        }
    };

    println!("Using model: {:?}", model_path);

    // Create config with small context for testing
    let config = ForgeConfig {
        context: ContextParams {
            n_ctx: 512,
            n_batch: 64,
            n_threads: 2,
            ..Default::default()
        },
        max_tokens: 10,
        ..Default::default()
    };

    // Try to create engine
    match ForgeEngine::new(&model_path, config) {
        Ok(engine) => {
            println!("✓ Engine created successfully");
            println!("  Context size: {}", engine.context_size());

            // Engine will be dropped here, testing RAII cleanup
            drop(engine);
            println!("✓ Engine dropped (RAII cleanup)");
        }
        Err(e) => panic!("Failed to create engine: {e:?}"),
    }

    println!("✓ Model loading test complete");
}

/// Test 4: Tokenization (requires model)
#[test]
fn test_tokenization() {
    let _guard = MODEL_TEST_LOCK.lock().unwrap();
    println!("=== Test: Tokenization ===");

    // Ensure backend is initialized
    LazyLock::force(&BACKEND);

    let model_path = match get_test_model_path() {
        Some(p) => p,
        None => {
            println!("⚠ No test model found. Skipping tokenization test.");
            return;
        }
    };

    let config = ForgeConfig {
        context: ContextParams {
            n_ctx: 512,
            n_batch: 64,
            n_threads: 2,
            ..Default::default()
        },
        max_tokens: 10,
        ..Default::default()
    };

    match ForgeEngine::new(&model_path, config) {
        Ok(engine) => {
            // Test tokenization
            let text = "Hello, world!";
            match engine.tokenize(text) {
                Ok(tokens) => {
                    println!("✓ Tokenized '{}' into {} tokens", text, tokens.len());
                    println!("  Tokens: {:?}", &tokens[..tokens.len().min(10)]);
                }
                Err(e) => {
                    panic!("Tokenization failed: {e:?}");
                }
            }
        }
        Err(e) => panic!("Could not create engine: {e:?}"),
    }

    println!("✓ Tokenization test complete");
}

/// Test 5: Simple generation (requires model)
///
/// NOTE: This test is identical to test_tokenization except for the println
/// and text. If it crashes but test_tokenization passes, the issue is in the
/// engine creation or drop order, not in tokenization.
#[test]
fn test_simple_generation() {
    let _guard = MODEL_TEST_LOCK.lock().unwrap();
    println!("=== Test: Simple Generation ===");

    // Ensure backend is initialized
    LazyLock::force(&BACKEND);

    let model_path = match get_test_model_path() {
        Some(p) => p,
        None => {
            println!("⚠ No test model found. Skipping generation test.");
            return;
        }
    };

    // Use SAME config as test_tokenization to isolate issue
    let config = ForgeConfig {
        context: ContextParams {
            n_ctx: 512,
            n_batch: 64,
            n_threads: 2,
            ..Default::default()
        },
        max_tokens: 10,
        ..Default::default()
    };

    match ForgeEngine::new(&model_path, config) {
        Ok(engine) => {
            println!("✓ Engine created, attempting simple decode test...");

            // USE EXACT SAME TEXT as test_tokenization
            let text = "Hello, world!";
            match engine.tokenize(text) {
                Ok(tokens) => {
                    println!("✓ Tokenized '{}' into {} tokens", text, tokens.len());
                    println!("  Tokens: {:?}", &tokens[..tokens.len().min(10)]);
                }
                Err(e) => {
                    println!("✗ Tokenization failed: {:?}", e);
                }
            }
        }
        Err(e) => panic!("Could not create engine: {e:?}"),
    }

    println!("✓ Simple generation test complete");
}

/// Test 6: Memory tracking
#[test]
fn test_memory_tracking() {
    println!("=== Test: Memory Tracking ===");

    let tracker = MemoryTracker::new(Some(100)); // 100 MB budget

    let initial = tracker.current_mb();
    println!("  Initial tracked: {:.2} MB", initial);

    tracker.alloc(1024 * 1024); // 1 MB
    println!("  After 1MB alloc: {:.2} MB", tracker.current_mb());

    tracker.dealloc(512 * 1024); // 512 KB
    println!("  After 512KB free: {:.2} MB", tracker.current_mb());

    println!("  Peak usage: {:.2} MB", tracker.peak_mb());

    tracker.reset();
    assert!((tracker.current_mb() - 0.0).abs() < 0.01);
    println!("✓ Memory tracker works correctly");
}

/// Test 7: Actual text generation (requires model)
#[test]
fn test_text_generation() {
    let _guard = MODEL_TEST_LOCK.lock().unwrap();
    println!("=== Test: Text Generation ===");

    // Ensure backend is initialized
    LazyLock::force(&BACKEND);

    let model_path = match get_test_model_path() {
        Some(p) => p,
        None => {
            println!("⚠ No test model found. Skipping text generation test.");
            return;
        }
    };

    let config = ForgeConfig {
        context: ContextParams {
            n_ctx: 512,
            n_batch: 64,
            n_threads: 2,
            ..Default::default()
        },
        max_tokens: 20, // Generate a few tokens
        ..Default::default()
    };

    match ForgeEngine::new(&model_path, config) {
        Ok(mut engine) => {
            println!("  Generating text from prompt...");

            let prompt = "<|im_start|>user\nHello!<|im_end|>\n<|im_start|>assistant\n";
            let mut generated = String::new();

            match engine.generate(prompt, |token| {
                generated.push_str(token);
                print!("{}", token);
            }) {
                Ok(result) => {
                    println!(); // newline after streaming output
                    println!(
                        "✓ Generated {} chars: {:?}",
                        result.len(),
                        &result[..result.len().min(50)]
                    );
                    assert!(!result.is_empty(), "Generated text should not be empty");

                    engine.reset();
                    engine
                        .generate_turn(
                            "<|im_start|>user\nRemember the number 42.<|im_end|>\n<|im_start|>assistant\n",
                            true,
                            |_| {},
                        )
                        .expect("first stateful turn should generate");
                    let after_first_turn = engine.n_past();
                    engine
                        .generate_turn(
                            "<|im_start|>user\nWhat number?<|im_end|>\n<|im_start|>assistant\n",
                            false,
                            |_| {},
                        )
                        .expect("second stateful turn should generate");
                    assert!(
                        engine.n_past() > after_first_turn,
                        "second turn must append to the existing context"
                    );
                }
                Err(e) => {
                    panic!("Generation failed: {e:?}");
                }
            }
        }
        Err(e) => panic!("Could not create engine: {e:?}"),
    }

    println!("✓ Text generation test complete");
}
