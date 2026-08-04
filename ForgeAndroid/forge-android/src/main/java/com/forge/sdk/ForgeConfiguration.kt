package com.forge.sdk

import android.app.ActivityManager
import android.content.Context
import android.util.Log

/**
 * Configuration for the Forge inference engine.
 *
 * Mirrors the ForgeConfiguration from ForgeSwift.
 */
data class ForgeConfiguration(
    /** Context size (number of tokens that can be processed) */
    var contextSize: Int = 2048,

    /** Batch size for prompt processing */
    var batchSize: Int = 128,

    /** Number of threads for inference */
    var threads: Int = 2,

    /** Maximum tokens to generate per turn */
    var maxTokens: Int = 128,

    /** Temperature (0.0 = greedy, higher = more random) */
    var temperature: Float = 0.3f,

    /** Top-K sampling (0 = disabled) */
    var topK: Int = 40,

    /** Top-P (nucleus) sampling (1.0 = disabled) */
    var topP: Float = 0.95f,

    /** Enable flash attention */
    var flashAttention: Boolean = true,

    /** Number of GPU layers (-1 = all, 0 = CPU only) */
    var gpuLayers: Int = 0  // CPU by default on Android
) {
    companion object {
        private const val TAG = "ForgeConfiguration"

        /**
         * Automatically detect device capabilities and return optimal configuration.
         */
        fun auto(context: Context): ForgeConfiguration {
            val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
            val memoryInfo = ActivityManager.MemoryInfo()
            activityManager.getMemoryInfo(memoryInfo)

            val totalMemoryGB = memoryInfo.totalMem / (1024.0 * 1024.0 * 1024.0)
            val processorCount = Runtime.getRuntime().availableProcessors()

            Log.i(TAG, "Device: ${totalMemoryGB.format(1)} GB RAM, $processorCount cores")

            val config = when {
                totalMemoryGB >= 8 -> {
                    // High-end device (8GB+)
                    ForgeConfiguration(
                        contextSize = 2048,
                        batchSize = 256,
                        threads = minOf(processorCount, 4),
                        maxTokens = 512
                    )
                }
                totalMemoryGB >= 6 -> {
                    // Mid-range device (6GB)
                    ForgeConfiguration(
                        contextSize = 1024,
                        batchSize = 128,
                        threads = minOf(processorCount, 2),
                        maxTokens = 256
                    )
                }
                else -> {
                    // Lower-end device
                    ForgeConfiguration(
                        contextSize = 512,
                        batchSize = 64,
                        threads = 2,
                        maxTokens = 128
                    )
                }
            }

            Log.i(TAG, "Auto config: ctx=${config.contextSize}, batch=${config.batchSize}, threads=${config.threads}")
            return config
        }

        /**
         * Configuration for vision models.
         * Vision models need larger context for image embeddings.
         */
        fun vision(context: Context): ForgeConfiguration {
            val config = auto(context)
            config.contextSize = maxOf(config.contextSize, 4096)
            config.batchSize = 512  // Larger batch for image token prefill
            Log.i(TAG, "Vision config: ctx=${config.contextSize}, batch=${config.batchSize}")
            return config
        }

        /**
         * Conservative configuration for low-memory devices.
         */
        fun mobile(): ForgeConfiguration {
            return ForgeConfiguration(
                contextSize = 1024,
                batchSize = 128,
                threads = 2,
                maxTokens = 128
            )
        }

        private fun Double.format(digits: Int) = "%.${digits}f".format(this)
    }
}
