package com.amma.recognize

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import com.amma.recognize.ui.screens.MainScreen
import com.amma.recognize.ui.theme.AmmaTheme
import com.amma.recognize.viewmodel.VisionViewModel
import com.forge.sdk.ForgeEngine

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            AmmaTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    val viewModel: VisionViewModel = viewModel()

                    LaunchedEffect(Unit) {
                        viewModel.loadModel()
                    }

                    MainScreen(viewModel = viewModel)
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        ForgeEngine.cleanup()
    }
}
