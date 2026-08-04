/**
 * Thin JNI bridge for ForgeKotlin
 *
 * This file provides JNI bindings to the Rust C FFI.
 * It's intentionally minimal - just type conversion, no logic.
 */

#include <jni.h>
#include <cmath>
#include <string>
#include <android/log.h>

// Include the Rust C FFI header
#include "forge_ffi.h"

#define LOG_TAG "ForgeJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// JNI package prefix
#define JNI_PREFIX Java_com_forge_sdk_ForgeNative_

namespace {
constexpr jint kMaxContextTokens = 1'048'576;
constexpr jint kMaxBatchTokens = 65'536;
constexpr jint kMaxGeneratedTokens = 1'048'576;
constexpr jint kMaxThreads = 1'024;
constexpr jint kMaxTopK = 1'000'000;
constexpr jint kMaxGpuLayers = 1'000'000;
constexpr jsize kMaxEncodedImageBytes = 64 * 1024 * 1024;

// Helper to convert jstring to a C++ string without dereferencing a null
// pointer when the VM is reporting an allocation failure.
static bool jstring_to_string(JNIEnv *env, jstring jstr, std::string *output) {
    if (jstr == nullptr || output == nullptr) return false;
    const char *cstr = env->GetStringUTFChars(jstr, nullptr);
    if (cstr == nullptr) return false;
    output->assign(cstr);
    env->ReleaseStringUTFChars(jstr, cstr);
    return !env->ExceptionCheck();
}

static bool valid_engine_params(
        jint nCtx,
        jint nBatch,
        jint nThreads,
        jint maxTokens,
        jfloat temperature,
        jint topK,
        jfloat topP,
        jint nGpuLayers
) {
    return nCtx > 0 && nCtx <= kMaxContextTokens &&
           nBatch > 0 && nBatch <= kMaxBatchTokens &&
           nThreads > 0 && nThreads <= kMaxThreads &&
           maxTokens > 0 && maxTokens <= kMaxGeneratedTokens &&
           std::isfinite(temperature) && temperature >= 0.0f && temperature <= 10.0f &&
           topK >= 0 && topK <= kMaxTopK &&
           std::isfinite(topP) && topP >= 0.0f && topP <= 1.0f &&
           nGpuLayers >= -1 && nGpuLayers <= kMaxGpuLayers;
}

// Callback context for token streaming
struct TokenCallbackContext {
    JNIEnv *env;
    jobject callback;
    jmethodID methodId;
    ForgeHandle handle;
    bool failed;
};

// Token callback that forwards to Kotlin
static void token_callback_impl(const char *token, void *user_data) {
    auto *ctx = static_cast<TokenCallbackContext *>(user_data);
    if (ctx == nullptr || ctx->callback == nullptr) return;

    if (token == nullptr || ctx->failed) return;
    jstring jtoken = ctx->env->NewStringUTF(token);
    if (jtoken == nullptr) {
        ctx->failed = true;
        forge_cancel(ctx->handle);
        return;
    }
    ctx->env->CallVoidMethod(ctx->callback, ctx->methodId, jtoken);
    ctx->env->DeleteLocalRef(jtoken);
    if (ctx->env->ExceptionCheck()) {
        ctx->env->ExceptionClear();
        ctx->failed = true;
        forge_cancel(ctx->handle);
    }
}
}  // namespace

extern "C" {

// ============================================================
// LIFECYCLE
// ============================================================

JNIEXPORT void JNICALL
Java_com_forge_sdk_ForgeNative_init(JNIEnv *env, jclass clazz) {
    LOGI("Initializing Forge backend");
    forge_init();
}

JNIEXPORT void JNICALL
Java_com_forge_sdk_ForgeNative_cleanup(JNIEnv *env, jclass clazz) {
    LOGI("Cleaning up Forge backend");
    forge_cleanup();
}

// ============================================================
// ENGINE
// ============================================================

JNIEXPORT jlong JNICALL
Java_com_forge_sdk_ForgeNative_engineCreate(
        JNIEnv *env,
        jclass clazz,
        jstring modelPath,
        jint nCtx,
        jint nBatch,
        jint nThreads,
        jint maxTokens,
        jfloat temperature,
        jint topK,
        jfloat topP,
        jboolean flashAttn,
        jint nGpuLayers
) {
    if (!valid_engine_params(nCtx, nBatch, nThreads, maxTokens, temperature, topK, topP,
                             nGpuLayers)) {
        LOGE("Rejected invalid engine parameters");
        return 0;
    }
    std::string path;
    if (!jstring_to_string(env, modelPath, &path) || path.empty()) return 0;
    LOGI("Creating engine with model: %s", path.c_str());

    ForgeParams params = forge_params_default();
    params.n_ctx = static_cast<uint32_t>(nCtx);
    params.n_batch = static_cast<uint32_t>(nBatch);
    params.n_threads = nThreads;
    params.max_tokens = static_cast<uint32_t>(maxTokens);
    params.temperature = temperature;
    params.top_k = topK;
    params.top_p = topP;
    params.flash_attn = flashAttn;
    params.n_gpu_layers = nGpuLayers;

    ForgeHandle handle = forge_engine_create(path.c_str(), &params);
    if (handle == nullptr) {
        LOGE("Failed to create engine");
        return 0;
    }

    LOGI("Engine created successfully");
    return reinterpret_cast<jlong>(handle);
}

JNIEXPORT jlong JNICALL
Java_com_forge_sdk_ForgeNative_engineCreateVision(
        JNIEnv *env,
        jclass clazz,
        jstring modelPath,
        jstring clipPath,
        jint nCtx,
        jint nBatch,
        jint nThreads,
        jint maxTokens,
        jfloat temperature,
        jint topK,
        jfloat topP,
        jboolean flashAttn,
        jint nGpuLayers
) {
    if (!valid_engine_params(nCtx, nBatch, nThreads, maxTokens, temperature, topK, topP,
                             nGpuLayers)) {
        LOGE("Rejected invalid vision engine parameters");
        return 0;
    }
    std::string model;
    std::string clip;
    if (!jstring_to_string(env, modelPath, &model) || model.empty() ||
        !jstring_to_string(env, clipPath, &clip) || clip.empty()) return 0;
    LOGI("Creating vision engine with model: %s, clip: %s", model.c_str(), clip.c_str());

    ForgeParams params = forge_params_default();
    params.n_ctx = static_cast<uint32_t>(nCtx);
    params.n_batch = static_cast<uint32_t>(nBatch);
    params.n_threads = nThreads;
    params.max_tokens = static_cast<uint32_t>(maxTokens);
    params.temperature = temperature;
    params.top_k = topK;
    params.top_p = topP;
    params.flash_attn = flashAttn;
    params.n_gpu_layers = nGpuLayers;

    ForgeHandle handle = forge_engine_create_vision(model.c_str(), clip.c_str(), &params);
    if (handle == nullptr) {
        LOGE("Failed to create vision engine");
        return 0;
    }

    LOGI("Vision engine created successfully");
    return reinterpret_cast<jlong>(handle);
}

JNIEXPORT void JNICALL
Java_com_forge_sdk_ForgeNative_engineDestroy(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return;
    LOGI("Destroying engine");
    forge_engine_destroy(reinterpret_cast<ForgeHandle>(handle));
}

JNIEXPORT jint JNICALL
Java_com_forge_sdk_ForgeNative_cancel(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return ForgeResult_NullPointer;
    return static_cast<jint>(forge_cancel(reinterpret_cast<ForgeHandle>(handle)));
}

// ============================================================
// GENERATION
// ============================================================

JNIEXPORT jint JNICALL
Java_com_forge_sdk_ForgeNative_generate(
        JNIEnv *env,
        jclass clazz,
        jlong handle,
        jstring prompt,
        jobject callback
) {
    if (handle == 0 || prompt == nullptr || callback == nullptr) return ForgeResult_NullPointer;

    std::string promptStr;
    if (!jstring_to_string(env, prompt, &promptStr)) return ForgeResult_InvalidParameter;

    // Get callback method
    jclass callbackClass = env->GetObjectClass(callback);
    if (callbackClass == nullptr) return ForgeResult_InvalidParameter;
    jmethodID methodId = env->GetMethodID(callbackClass, "onToken", "(Ljava/lang/String;)V");
    if (methodId == nullptr) {
        env->DeleteLocalRef(callbackClass);
        LOGE("Could not find onToken method");
        return ForgeResult_InvalidParameter;
    }

    auto forgeHandle = reinterpret_cast<ForgeHandle>(handle);
    TokenCallbackContext ctx = {env, callback, methodId, forgeHandle, false};

    ForgeResult result = forge_generate(
            forgeHandle,
            promptStr.c_str(),
            token_callback_impl,
            &ctx
    );
    env->DeleteLocalRef(callbackClass);

    return ctx.failed ? ForgeResult_InvalidParameter : static_cast<jint>(result);
}

JNIEXPORT jint JNICALL
Java_com_forge_sdk_ForgeNative_generateTurn(
        JNIEnv *env,
        jclass clazz,
        jlong handle,
        jstring prompt,
        jboolean addBos,
        jobject callback
) {
    if (handle == 0 || prompt == nullptr || callback == nullptr) return ForgeResult_NullPointer;

    std::string promptStr;
    if (!jstring_to_string(env, prompt, &promptStr)) return ForgeResult_InvalidParameter;

    jclass callbackClass = env->GetObjectClass(callback);
    if (callbackClass == nullptr) return ForgeResult_InvalidParameter;
    jmethodID methodId = env->GetMethodID(callbackClass, "onToken", "(Ljava/lang/String;)V");
    if (methodId == nullptr) {
        env->DeleteLocalRef(callbackClass);
        LOGE("Could not find onToken method");
        return ForgeResult_InvalidParameter;
    }

    auto forgeHandle = reinterpret_cast<ForgeHandle>(handle);
    TokenCallbackContext ctx = {env, callback, methodId, forgeHandle, false};

    ForgeResult result = forge_generate_turn(
            forgeHandle,
            promptStr.c_str(),
            addBos,
            token_callback_impl,
            &ctx
    );
    env->DeleteLocalRef(callbackClass);

    return ctx.failed ? ForgeResult_InvalidParameter : static_cast<jint>(result);
}

JNIEXPORT jint JNICALL
Java_com_forge_sdk_ForgeNative_generateVision(
        JNIEnv *env,
        jclass clazz,
        jlong handle,
        jbyteArray imageData,
        jstring prompt,
        jobject callback
) {
    if (handle == 0 || imageData == nullptr || prompt == nullptr || callback == nullptr) {
        return ForgeResult_NullPointer;
    }

    std::string promptStr;
    if (!jstring_to_string(env, prompt, &promptStr)) return ForgeResult_InvalidParameter;

    // Get image bytes
    jsize imageLen = env->GetArrayLength(imageData);
    if (imageLen <= 0 || imageLen > kMaxEncodedImageBytes) return ForgeResult_InvalidParameter;
    jbyte *imageBytes = env->GetByteArrayElements(imageData, nullptr);
    if (imageBytes == nullptr) return ForgeResult_MemoryExceeded;

    jclass callbackClass = env->GetObjectClass(callback);
    if (callbackClass == nullptr) {
        env->ReleaseByteArrayElements(imageData, imageBytes, JNI_ABORT);
        return ForgeResult_InvalidParameter;
    }
    jmethodID methodId = env->GetMethodID(callbackClass, "onToken", "(Ljava/lang/String;)V");
    if (methodId == nullptr) {
        env->ReleaseByteArrayElements(imageData, imageBytes, JNI_ABORT);
        env->DeleteLocalRef(callbackClass);
        LOGE("Could not find onToken method");
        return ForgeResult_InvalidParameter;
    }

    auto forgeHandle = reinterpret_cast<ForgeHandle>(handle);
    TokenCallbackContext ctx = {env, callback, methodId, forgeHandle, false};

    ForgeResult result = forge_generate_vision(
            forgeHandle,
            reinterpret_cast<const uint8_t *>(imageBytes),
            static_cast<size_t>(imageLen),
            promptStr.c_str(),
            token_callback_impl,
            &ctx
    );

    env->ReleaseByteArrayElements(imageData, imageBytes, JNI_ABORT);
    env->DeleteLocalRef(callbackClass);

    return ctx.failed ? ForgeResult_InvalidParameter : static_cast<jint>(result);
}

// ============================================================
// STATE
// ============================================================

JNIEXPORT jboolean JNICALL
Java_com_forge_sdk_ForgeNative_isFirstTurn(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return JNI_TRUE;
    return forge_is_first_turn(reinterpret_cast<ForgeHandle>(handle)) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jint JNICALL
Java_com_forge_sdk_ForgeNative_nPast(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return 0;
    return forge_n_past(reinterpret_cast<ForgeHandle>(handle));
}

JNIEXPORT jint JNICALL
Java_com_forge_sdk_ForgeNative_reset(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return ForgeResult_NullPointer;
    return static_cast<jint>(forge_reset(reinterpret_cast<ForgeHandle>(handle)));
}

JNIEXPORT jdouble JNICALL
Java_com_forge_sdk_ForgeNative_memoryCurrentMb(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return 0.0;
    return forge_memory_current_mb(reinterpret_cast<ForgeHandle>(handle));
}

JNIEXPORT jdouble JNICALL
Java_com_forge_sdk_ForgeNative_memoryPeakMb(JNIEnv *env, jclass clazz, jlong handle) {
    if (handle == 0) return 0.0;
    return forge_memory_peak_mb(reinterpret_cast<ForgeHandle>(handle));
}

JNIEXPORT jstring JNICALL
Java_com_forge_sdk_ForgeNative_getMediaMarker(JNIEnv *env, jclass clazz) {
    const char *marker = forge_get_media_marker();
    return env->NewStringUTF(marker ? marker : "<image>");
}

} // extern "C"
