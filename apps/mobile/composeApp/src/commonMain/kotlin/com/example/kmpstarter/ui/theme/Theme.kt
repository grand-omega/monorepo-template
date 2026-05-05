package com.example.kmpstarter.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable

/** Per-platform dynamic color (e.g. Android 12+ Material You). Returns null when unsupported. */
@Composable
expect fun platformDynamicColorScheme(useDarkTheme: Boolean): ColorScheme?

@Composable
fun AppTheme(
    useDarkTheme: Boolean = isSystemInDarkTheme(),
    useDynamicColor: Boolean = true,
    content: @Composable () -> Unit,
) {
    val dynamic = if (useDynamicColor) platformDynamicColorScheme(useDarkTheme) else null
    val colorScheme = dynamic ?: if (useDarkTheme) DarkColors else LightColors

    MaterialTheme(
        colorScheme = colorScheme,
        typography = AppTypography,
        content = content,
    )
}
