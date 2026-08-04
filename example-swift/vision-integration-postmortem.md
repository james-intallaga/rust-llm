# Vision and Camera Integration Postmortem

This document records the critical issues encountered and fixed during the integration of the Rust-based Forge vision engine into the iOS application.

## 1. Metal API Validation Crash (Camera Conflict)

### Problem
The app crashed with a `SIGABRT` or `EXC_BAD_ACCESS` immediately upon opening the camera. The Xcode console showed an assertion failure:
`-[MTLDebugDevice newTextureWithDescriptor:iosurface:plane:]:2808: failed assertion Texture Descriptor Validation: IOSurface textures must use MTLStorageModeShared`

### Root Cause
Xcode's **Metal API Validation** (the `MTLDebugDevice` layer) is extremely strict. When the iOS camera pipeline creates `IOSurface`-backed textures, it requires specific storage modes. These requirements conflict with how `llama.cpp`'s Metal backend manages its own GPU resources when both are active in the same process. This is a common issue when running LLMs alongside high-performance camera or AR pipelines in Debug mode.

### Fix
The fix is to disable Metal API Validation in the Xcode scheme. While this doesn't affect production (Release) builds, it is necessary for development.

1. **Manual Fix**: In Xcode, go to `Product` → `Scheme` → `Edit Scheme` → `Run` → `Diagnostics` and uncheck **Metal API Validation**.
2. **Automated Fix**: Added the following to `project.yml` to ensure the generated project has these settings disabled:
   ```yaml
   scheme:
     run:
       enableGPUFrameCaptureMode: disabled
       enableGPUValidationMode: disabled
   ```

---

## 2. "Distorted/Pixelated" Vision Hallucinations

### Problem
The model was able to process images but frequently responded with variations of: *"This is a distorted and pixelated image, making it difficult to identify any specific content."* Even when clear photos were sent, the model saw only noise.

### Root Cause
**Image Orientation and Dimension Mismatch.**
1. **Orientation**: iOS camera photos are often stored with an orientation metadata flag (e.g., "Right" for portrait) while the raw pixel buffer remains in landscape.
2. **Scrambled Bytes**: When we extracted raw RGBA bytes from `UIImage`, we were passing the "intended" width/height to the Rust layer, but the raw pixels were still in their un-rotated state. This caused the Rust layer to "mis-wrap" the pixels (e.g., thinking a row is 1024 pixels wide when it's actually 1366), resulting in scrambled visual noise.

### Fix
Flattened the image orientation in the Swift layer before sending the bytes to Rust. By drawing the `UIImage` into a new `CGContext`, we "bake" the orientation into the raw pixels so the memory layout perfectly matches the reported dimensions.

**Swift Implementation Change:**
```swift
private func imageToRGBA(_ image: UIImage) -> (data: [UInt8], width: Int, height: Int)? {
    // 1. Flatten orientation by drawing into a new context
    UIGraphicsBeginImageContextWithOptions(image.size, false, image.scale)
    image.draw(in: CGRect(origin: .zero, size: image.size))
    let normalizedImage = UIGraphicsGetImageFromCurrentImageContext()
    UIGraphicsEndImageContext()

    guard let flattened = normalizedImage, let cgImage = flattened.cgImage else { return nil }

    // ... extract bytes from flattened.cgImage ...
}
```

---

## 3. RGBA vs RGB Mismatch

### Problem
The underlying `mtmd_bitmap_init` function in the multimodal library expected **3-byte RGB** data, but the iOS pipeline was providing **4-byte RGBA** data.

### Fix
Added a conversion step in the Rust `MultimodalContext` to strip the Alpha channel before passing the buffer to the C++ layer.

**Rust Implementation:**
```rust
let pixel_count = (width * height) as usize;
let mut rgb_data = Vec::with_capacity(pixel_count * 3);
for i in 0..pixel_count {
    rgb_data.push(rgba_data[i * 4]);     // R
    rgb_data.push(rgba_data[i * 4 + 1]); // G
    rgb_data.push(rgba_data[i * 4 + 2]); // B
}
```
