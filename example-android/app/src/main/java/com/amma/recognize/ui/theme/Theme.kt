package com.amma.recognize.ui.theme

import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

// Warm, restrained palette shared with the iOS app.
private val LightColorScheme = lightColorScheme(
    primary = Color(0xFF1A4334),
    onPrimary = Color.White,
    primaryContainer = Color(0xFF1A4334),
    onPrimaryContainer = Color.White,
    secondary = Color(0xFF6B6E66),
    onSecondary = Color.White,
    background = Color(0xFFF7F5F0),
    onBackground = Color(0xFF171A17),
    surface = Color.White,
    onSurface = Color(0xFF171A17),
    surfaceVariant = Color(0xFFF7F5F0),
    onSurfaceVariant = Color(0xFF6B6E66),
    outline = Color(0xFFDFDAD1),
    error = Color(0xFFFF3B30),
    onError = Color.White
)

@Composable
fun AmmaTheme(
    darkTheme: Boolean = false, // Force light theme like iOS
    content: @Composable () -> Unit
) {
    val colorScheme = LightColorScheme

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            window.statusBarColor = Color(0xFFF7F5F0).toArgb()
            window.navigationBarColor = Color(0xFFF7F5F0).toArgb()
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = true
                isAppearanceLightNavigationBars = true
            }
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography(),
        content = content
    )
}
