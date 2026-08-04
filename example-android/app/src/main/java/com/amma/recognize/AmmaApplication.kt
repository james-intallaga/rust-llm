package com.amma.recognize

import android.app.Application
import com.forge.sdk.ForgeEngine

class AmmaApplication : Application() {

    override fun onCreate() {
        super.onCreate()
        // Initialize Forge backend early
        ForgeEngine.initialize()
    }

    override fun onTerminate() {
        super.onTerminate()
        // Cleanup Forge backend
        ForgeEngine.cleanup()
    }
}
