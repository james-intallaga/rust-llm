/**
 * Thin JNI bridge for ForgeKotlin
 *
 * This file provides JNI bindings to the Rust C FFI.
 * It's intentionally minimal - just type conversion, no logic.
 */

#include <jni.h>
#include <string>
#include <android/log.h>

// Include the Rust C FFI header
#include "forge_ffi.h"

#define LOG_TAG "ForgeJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// JNI package prefix
#define JNI_PREFIX Java_com_forge_sdk_ForgeNative_

// Helper to convert jstring to C string
static std::string jstring_to_string(JNIEnv *env, jstring jstr) {
    if (jstr == nullptr) return "";
    const char *cstr = env->GetStringUTFChars(jstr, nullptr);
    std::string result(cstr);
    env->ReleaseStringUTFChars(jstr, cstr);
    return result;
}

// Callback context for token streaming
struct TokenCallbackContext {
    JNIEnv *env;
    jobject callback;
    jmethodID methodId;
};

// Token callback that forwards to Kotlin
static void token_callback_impl(const char *token, void *user_data) {
    auto *ctx = static_cast<TokenCallbackContext *>(user_data);
    if (ctx == nullptr || ctx->callback == nullptr) return;

    jstring jtoken = ctx->env->NewStringUTF(token);
    ctx->env->CallVoidMethod(ctx->callback, ctx->methodId, jtoken);
    ctx->env->DeleteLocalRef(jtoken);
}

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
    std::string path = jstring_to_string(env, modelPath);
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
    std::string model = jstring_to_string(env, modelPath);
    std::string clip = jstring_to_string(env, clipPath);
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
    if (handle == 0) return ForgeResult_NullPointer;

    std::string promptStr = jstring_to_string(env, prompt);

    // Get callback method
    jclass callbackClass = env->GetObjectClass(callback);
    jmethodID methodId = env->GetMethodID(callbackClass, "onToken", "(Ljava/lang/String;)V");
    if (methodId == nullptr) {
        LOGE("Could not find onToken method");
        return ForgeResult_InvalidParameter;
    }

    TokenCallbackContext ctx = {env, callback, methodId};

    ForgeResult result = forge_generate(
            reinterpret_cast<ForgeHandle>(handle),
            promptStr.c_str(),
            token_callback_impl,
            &ctx
    );

    return static_cast<jint>(result);
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
    if (handle == 0) return ForgeResult_NullPointer;

    std::string promptStr = jstring_to_string(env, prompt);

    jclass callbackClass = env->GetObjectClass(callback);
    jmethodID methodId = env->GetMethodID(callbackClass, "onToken", "(Ljava/lang/String;)V");
    if (methodId == nullptr) {
        LOGE("Could not find onToken method");
        return ForgeResult_InvalidParameter;
    }

    TokenCallbackContext ctx = {env, callback, methodId};

    ForgeResult result = forge_generate_turn(
            reinterpret_cast<ForgeHandle>(handle),
            promptStr.c_str(),
            addBos,
            token_callback_impl,
            &ctx
    );

    return static_cast<jint>(result);
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
    if (handle == 0) return ForgeResult_NullPointer;

    std::string promptStr = jstring_to_string(env, prompt);

    // Get image bytes
    jsize imageLen = env->GetArrayLength(imageData);
    jbyte *imageBytes = env->GetByteArrayElements(imageData, nullptr);

    jclass callbackClass = env->GetObjectClass(callback);
    jmethodID methodId = env->GetMethodID(callbackClass, "onToken", "(Ljava/lang/String;)V");
    if (methodId == nullptr) {
        env->ReleaseByteArrayElements(imageData, imageBytes, JNI_ABORT);
        LOGE("Could not find onToken method");
        return ForgeResult_InvalidParameter;
    }

    TokenCallbackContext ctx = {env, callback, methodId};

    ForgeResult result = forge_generate_vision(
            reinterpret_cast<ForgeHandle>(handle),
            reinterpret_cast<const uint8_t *>(imageBytes),
            static_cast<size_t>(imageLen),
            promptStr.c_str(),
            token_callback_impl,
            &ctx
    );

    env->ReleaseByteArrayElements(imageData, imageBytes, JNI_ABORT);

    return static_cast<jint>(result);
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
