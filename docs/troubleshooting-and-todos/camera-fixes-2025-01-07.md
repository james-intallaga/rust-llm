# Camera Crash Fixes - January 7, 2025

## Summary
Multiple attempts to fix camera crashes when opening camera or taking photos. The issue was related to memory management and Metal/GPU resource conflicts. **Final status: ALL CHANGES REVERTED - Using simple camera handling without memory suspension.**

---

## REVERTED STATE (Current)

All camera memory management code was **completely removed** for simplicity:
- **No `suspendForCamera()` call before opening camera**
- **No `cameraReady` state variable**
- **No `onChange` handlers for camera**
- **No memory polling or thresholds**
- **Simple `showCamera = true` to open camera**
- **Rust FFI suspend functions are NO-OPS**
- **Rust engine struct uses direct fields (not Option<>)**

### Current Code State

**MainView.swift:**
```swift
onCamera: {
    showAttachmentMenu = false
    showCamera = true  // Simple, no memory management
}
.sheet(isPresented: $showCamera) {
    CameraPicker(...)
}
```

**CameraPicker (photo capture):**
```swift
func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey : Any]) {
    if let image = info[.originalImage] as? UIImage {
        parent.selectedImage = image
        parent.onImageSelected(image)
    }
    parent.presentationMode.wrappedValue.dismiss()
}
```

**VisionViewModel.swift:**
```swift
func suspendForCamera() {
    do {
        try engine?.suspendForCamera()  // This is now a NO-OP in Rust
    } catch {
        print("[VisionViewModel] Failed to suspend: \(error)")
    }
}
```
Note: This function is NOT called from MainView, and even if called, it does nothing.

**engine.rs:**
- `suspend_for_camera()`, `is_suspended()`, `suspend_for_camera_full()`, `is_suspended_full()`, `recreate_context()`, `ensure_context()` - **ALL REMOVED**
- Struct fields are now direct (not `Option<>`): `model: LlamaModel`, `context: LlamaContext`, `sampler: Sampler`
- No model/context dropping or recreation logic

**ffi.rs:**
```rust
// ALL NO-OPS - kept for API compatibility
pub extern "C" fn forge_suspend_for_camera(_engine: ForgeHandle) -> ForgeResult {
    ForgeResult::Ok
}

pub extern "C" fn forge_resume_after_camera(_engine: ForgeHandle) -> ForgeResult {
    ForgeResult::Ok
}

pub extern "C" fn forge_is_suspended(_engine: ForgeHandle) -> bool {
    false
}
```

---

## FAILED ATTEMPTS (Documented Below for Reference)

---

## Problem Statement
- Camera crashes when opening (memory pressure)
- Camera crashes when taking photos (even after memory is freed)
- Memory drops too aggressively (from ~1328MB to 59MB) when model is dropped
- Camera only needs ~200MB but we were freeing 1000MB+ and still crashing

---

## Attempt 1: Drop Entire Model (TOO AGGRESSIVE - REVERTED)

### Changes Made

**File: `forge-rust/forge-core/src/ffi.rs`**
```rust
// BEFORE:
pub extern "C" fn forge_suspend_for_camera(engine: ForgeHandle) -> ForgeResult {
    match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => {
            e.suspend_for_camera_full();  // Drops model + context + sampler + multimodal
            ForgeResult::Ok
        }
        None => ForgeResult::NullPointer,
    }
}

// AFTER (Attempt 1):
pub extern "C" fn forge_suspend_for_camera(engine: ForgeHandle) -> ForgeResult {
    match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => {
            e.suspend_for_camera_full();  // Still dropping model
            ForgeResult::Ok
        }
        None => ForgeResult::NullPointer,
    }
}
```

**File: `forge-rust/forge-core/src/engine.rs`**
- Made `model: LlamaModel` → `model: Option<LlamaModel>`
- Added `model_path: Option<PathBuf>` to store path for reload
- `suspend_for_camera_full()` drops model via `self.model = None`
- `recreate_context()` reloads model if `self.model.is_none()`

**File: `example-swift/Sources/AmmaRecognize/ViewModels/VisionViewModel.swift`**
- Memory thresholds: `residentMB < 700.0 && footprintMB < 600.0`
- Wait loop: 15 iterations × 0.2 seconds = 3 seconds max
- Additional 0.2s wait after threshold met

**Result:**
- Memory dropped from ~1328MB to 59MB (too aggressive)
- Model was completely unmapped
- Camera still crashed
- **REVERTED** - Too aggressive, camera doesn't need that much memory freed

---

## Attempt 2: Only Clear KV Cache (INSUFFICIENT - REVERTED)

### Changes Made

**File: `forge-rust/forge-core/src/ffi.rs`**
```rust
// AFTER (Attempt 2):
pub extern "C" fn forge_suspend_for_camera(engine: ForgeHandle) -> ForgeResult {
    match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => {
            e.suspend_for_camera();  // Only clears KV cache, keeps model loaded
            ForgeResult::Ok
        }
        None => ForgeResult::NullPointer,
    }
}
```

**File: `forge-rust/forge-core/src/engine.rs`**
```rust
// BEFORE (Attempt 2):
pub fn suspend_for_camera(&mut self) {
    log::info!("Suspending for camera - clearing KV cache to free ~500-800MB");
    if let Some(ref mut ctx) = self.context {
        ctx.clear_kv_cache();  // Only clears KV cache data
    }
    self.n_past = 0;
}
```

**File: `example-swift/Sources/AmmaRecognize/ViewModels/VisionViewModel.swift`**
- Memory thresholds: `footprintMB < 300.0`
- Wait loop: 10 iterations × 0.2 seconds = 2 seconds max
- Expected: resident ~460-600MB, footprint ~100-200MB

**Result:**
- Memory didn't drop enough
- Metal/GPU resources still held by context
- Camera crashed due to GPU resource conflict
- **REVERTED** - Doesn't free GPU resources needed by camera

---

## Attempt 3: Drop Context But Keep Model (CURRENT - PARTIALLY WORKING)

### Changes Made

**File: `forge-rust/forge-core/src/ffi.rs`**
```rust
// CURRENT:
pub extern "C" fn forge_suspend_for_camera(engine: ForgeHandle) -> ForgeResult {
    match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => {
            e.suspend_for_camera();  // Drops context/sampler/multimodal, keeps model
            ForgeResult::Ok
        }
        None => ForgeResult::NullPointer,
    }
}

pub extern "C" fn forge_is_suspended(engine: ForgeHandle) -> bool {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.is_suspended(),  // Check if context is None
        None => true,
    }
}
```

**File: `forge-rust/forge-core/src/engine.rs`**
```rust
/// Suspend for camera - drops context/sampler/multimodal to free Metal/GPU resources.
/// The model stays loaded (mmap'd), but Metal resources are freed for camera.
pub fn suspend_for_camera(&mut self) {
    eprintln!("[forge-core] suspend_for_camera: DROPPING context/sampler/multimodal to free Metal/GPU resources");

    // Drop context (frees Metal/GPU resources via llama_free)
    if self.context.is_some() {
        eprintln!("[forge-core] Dropping LlamaContext (frees Metal/GPU)...");
        self.context = None;
        eprintln!("[forge-core] LlamaContext dropped");
    }

    // Drop sampler (frees sampler resources)
    if self.sampler.is_some() {
        eprintln!("[forge-core] Dropping Sampler...");
        self.sampler = None;
        eprintln!("[forge-core] Sampler dropped");
    }

    // Drop multimodal (frees CLIP Metal resources via mtmd_free)
    if self.multimodal.is_some() {
        eprintln!("[forge-core] Dropping MultimodalContext (frees CLIP Metal)...");
        self.multimodal = None;
        eprintln!("[forge-core] MultimodalContext dropped");
    }

    // Model stays loaded - we don't drop it
    // This keeps model weights mmap'd (~800MB resident, but swappable)

    self.n_past = 0;
    self.last_eog_token = None;
    eprintln!("[forge-core] suspend_for_camera: COMPLETE - Metal/GPU freed, model stays loaded");
}

/// Check if engine is in a "suspended" state (context dropped, Metal/GPU freed)
pub fn is_suspended(&self) -> bool {
    self.context.is_none()  // Changed from: self.n_past == 0
}
```

**File: `example-swift/Sources/AmmaRecognize/ViewModels/VisionViewModel.swift`**
```swift
func suspendForCamera() async {
    print("[VisionViewModel] Suspending for camera - dropping context to free Metal/GPU resources...")
    logMemory("before_suspend")

    autoreleasepool {
        do {
            try engine?.suspendForCamera()
            print("[VisionViewModel] Successfully suspended - context dropped (Metal/GPU freed)...")

            // Force Metal to complete any pending work
            if let device = MTLCreateSystemDefaultDevice(),
               let commandQueue = device.makeCommandQueue(),
               let commandBuffer = commandQueue.makeCommandBuffer() {
                commandBuffer.commit()
                commandBuffer.waitUntilCompleted()
                print("[VisionViewModel] Metal sync complete - GPU resources freed")
            }

            autoreleasepool { }
        } catch {
            print("[VisionViewModel] Failed to suspend: \(error)")
        }
    }

    // Wait for memory to drop and GPU to be released
    // Expected: resident ~460-600MB, footprint ~100-200MB
    for i in 1...8 {
        try? await Task.sleep(nanoseconds: 200_000_000) // 0.2 seconds per iteration
        logMemory("after_suspend_sleep_\(i)")

        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout.size(ofValue: info) / MemoryLayout<natural_t>.size)
        let kr: kern_return_t = withUnsafeMutablePointer(to: &info) { ptr in
            ptr.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        if kr == KERN_SUCCESS {
            let residentMB = Double(info.resident_size) / 1024.0 / 1024.0
            let footprintMB = Double(info.phys_footprint) / 1024.0 / 1024.0
            // Threshold: footprint < 300MB (dirty memory from KV cache)
            if footprintMB < 300.0 {
                print("[VisionViewModel] GPU resources freed, memory dropped to footprint=\(footprintMB) MB, resident=\(residentMB) MB - safe to open camera")
                try? await Task.sleep(nanoseconds: 200_000_000) // Additional 0.2s wait
                break
            }
        }
    }

    logMemory("after_suspend_final")
}
```

**File: `example-swift/Sources/AmmaRecognize/Views/MainView.swift`**
```swift
// Camera button handler
onCamera: {
    showAttachmentMenu = false
    Task {
        await viewModel.suspendForCamera()  // Async suspend
        showCamera = true  // Show camera after suspend completes
    }
}

// Camera sheet presentation
.sheet(isPresented: $cameraReady) {
    CameraPicker(selectedImage: $viewModel.selectedImage) { image in
        viewModel.pendingImages.append(image)
    }
}
.onChange(of: showCamera) { isShowing in
    if isShowing {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            cameraReady = true  // Delay before showing camera
        }
    } else {
        cameraReady = false
    }
}
.onChange(of: cameraReady) { isReady in
    if !isReady {
        showCamera = false  // Reset when camera dismissed
    }
}

// Photo capture handler
func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey : Any]) {
    parent.presentationMode.wrappedValue.dismiss()  // Dismiss first

    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {  // Delay before processing
        if let image = info[.originalImage] as? UIImage {
            self.parent.selectedImage = image
            self.parent.onImageSelected(image)
        }
    }
}
```

### Memory Thresholds (Current)
- **Footprint threshold**: `< 300.0 MB` (dirty memory)
- **Resident threshold**: Not checked (model stays loaded, ~460-600MB expected)
- **Wait loop**: 8 iterations × 0.2s = 1.6 seconds max
- **Additional wait**: 0.2s after threshold met
- **Total max wait**: ~1.8 seconds

### Expected Memory Behavior
- **Before suspend**: resident ~1328MB, footprint ~665MB
- **After suspend**: resident ~460-600MB, footprint ~100-200MB
- **Model**: Stays mmap'd (~800MB resident, swappable by iOS)
- **KV cache**: Freed (~500-800MB dirty memory)
- **Metal/GPU**: Freed (buffers, command queues, residency sets)

### Result
- **Status**: Partially working (accidentally works sometimes)
- **Memory**: Drops correctly (~460-600MB resident)
- **GPU**: Resources freed
- **Camera**: Still crashes unpredictably
- **Photo capture**: Works occasionally but not reliably

---

## Key Functions and Parameters

### Rust Functions

**`forge-rust/forge-core/src/engine.rs`**
- `suspend_for_camera()` - Drops context/sampler/multimodal, keeps model
- `is_suspended()` - Returns `self.context.is_none()`
- `recreate_context()` - Recreates context/sampler/multimodal if dropped

**`forge-rust/forge-core/src/ffi.rs`**
- `forge_suspend_for_camera(engine: ForgeHandle) -> ForgeResult` - FFI wrapper
- `forge_is_suspended(engine: ForgeHandle) -> bool` - FFI wrapper

### Swift Functions

**`example-swift/Sources/AmmaRecognize/ViewModels/VisionViewModel.swift`**
- `suspendForCamera() async` - Async suspend with memory polling
- `logMemory(_ tag: String)` - Logs `phys_footprint` and `resident_size`

**`example-swift/Sources/AmmaRecognize/Views/MainView.swift`**
- Camera button handler - Calls `suspendForCamera()` then shows camera
- `CameraPicker` - UIImagePickerController wrapper
- Photo capture handler - Dismisses camera, delays 0.1s, then processes image

---

## Memory Metrics

### iOS Memory Types
- **`phys_footprint`** (footprint): Dirty memory that iOS cannot compress
- **`resident_size`** (resident): Total physical pages (including mmap'd files)

### Memory Breakdown (Before Suspend)
- Model weights (mmap'd): ~800MB resident
- KV cache: ~500-800MB footprint (dirty)
- Context buffers: ~40MB footprint
- Total: ~1328MB resident, ~665MB footprint

### Memory Breakdown (After Suspend)
- Model weights (mmap'd): ~800MB resident (stays, but swappable)
- KV cache: 0MB (freed)
- Context buffers: 0MB (freed)
- Total: ~460-600MB resident, ~100-200MB footprint

---

## Timing Parameters

### Delays
- **Memory polling interval**: 0.2 seconds
- **Max wait iterations**: 8 (total 1.6s)
- **Additional wait after threshold**: 0.2s
- **Camera sheet delay**: 0.2s after `showCamera = true`
- **Photo processing delay**: 0.1s after camera dismiss

### Total Timeline
1. User taps camera → `suspendForCamera()` called
2. Context dropped → Metal sync (immediate)
3. Memory polling (0-1.6s, exits when footprint < 300MB)
4. Additional wait (0.2s)
5. Camera sheet shows (0.2s delay)
6. **Total**: ~0.4-2.0 seconds before camera opens

---

## Metal/GPU Resources

### What Gets Freed
- **LlamaContext**: Metal buffers, command queues, GPU memory
- **MultimodalContext**: CLIP Metal resources
- **Sampler**: Sampler chain resources

### What Stays
- **LlamaModel**: Model weights (mmap'd file, not GPU memory)
- **Metal device**: Shared device instance (not freed)

### Metal Sync
```swift
if let device = MTLCreateSystemDefaultDevice(),
   let commandQueue = device.makeCommandQueue(),
   let commandBuffer = commandQueue.makeCommandBuffer() {
    commandBuffer.commit()
    commandBuffer.waitUntilCompleted()  // Blocks until GPU work done
}
```

---

## Known Issues

1. **Unreliable**: Camera works "accidentally" sometimes but not consistently
2. **Photo capture crash**: Still crashes when taking photos (even after memory freed)
3. **Timing sensitive**: May need more/less delay depending on device
4. **Memory threshold**: 300MB footprint may not be optimal for all devices

---

## Files Modified

1. `forge-rust/forge-core/src/engine.rs` - `suspend_for_camera()`, `is_suspended()`
2. `forge-rust/forge-core/src/ffi.rs` - `forge_suspend_for_camera()`, `forge_is_suspended()`
3. `example-swift/Sources/AmmaRecognize/ViewModels/VisionViewModel.swift` - `suspendForCamera()`, memory thresholds
4. `example-swift/Sources/AmmaRecognize/Views/MainView.swift` - Camera button handler, photo capture handler

---

## Reversion Instructions

If breaking changes are made to the SDK and we need to revert:

1. **Restore `suspend_for_camera()` implementation** in `engine.rs`:
   - Drop context/sampler/multimodal
   - Keep model loaded
   - Set `n_past = 0`, `last_eog_token = None`

2. **Restore FFI** in `ffi.rs`:
   - `forge_suspend_for_camera()` calls `e.suspend_for_camera()`
   - `forge_is_suspended()` calls `e.is_suspended()` (checks `context.is_none()`)

3. **Restore Swift memory thresholds** in `VisionViewModel.swift`:
   - Footprint threshold: `< 300.0 MB`
   - Wait loop: 8 iterations × 0.2s
   - Additional wait: 0.2s after threshold

4. **Restore camera timing** in `MainView.swift`:
   - Camera sheet delay: 0.2s
   - Photo processing delay: 0.1s

5. **Rebuild**:
   ```bash
   cd forge-rust && ./scripts/build-ios.sh
   cd forge-rust && ./scripts/package-xcframework.sh
   cp -R forge-rust/build/ForgeRustCore.xcframework ForgeSwift/
   cd example-swift && xcodebuild clean && xcodebuild build
   ```

---

## Next Steps (Future Investigation)

1. Investigate why camera crashes even with GPU resources freed
2. Check if Metal residency sets need explicit release
3. Test with different memory thresholds (200MB, 250MB, 350MB)
4. Investigate photo capture crash separately (may be different issue)
5. Further investigate Metal/GPU resource management

---

**Document Date**: January 7, 2025
**Status**: Partially Working (Unreliable)
**Last Test**: Memory drops correctly, GPU freed, but camera still crashes unpredictably
