package com.example.kmpstarter.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

/**
 * Material 3 type scale, lightly tuned: tighter display tracking and
 * heavier display weight for a more native-feeling hierarchy. Falls
 * back to the platform's default font (Roboto on Android, SF on iOS).
 */
internal val AppTypography = Typography().run {
    copy(
        displayLarge = displayLarge.copy(fontWeight = FontWeight.SemiBold, letterSpacing = (-0.5).sp),
        displayMedium = displayMedium.copy(fontWeight = FontWeight.SemiBold, letterSpacing = (-0.25).sp),
        displaySmall = displaySmall.copy(fontWeight = FontWeight.SemiBold),
        headlineLarge = headlineLarge.copy(fontWeight = FontWeight.SemiBold),
        headlineMedium = headlineMedium.copy(fontWeight = FontWeight.SemiBold),
        headlineSmall = headlineSmall.copy(fontWeight = FontWeight.SemiBold),
        titleLarge = titleLarge.copy(fontWeight = FontWeight.SemiBold, fontSize = 22.sp, lineHeight = 28.sp),
        titleMedium = titleMedium.copy(fontWeight = FontWeight.Medium),
        labelLarge = TextStyle(
            fontWeight = FontWeight.SemiBold,
            fontSize = 15.sp,
            lineHeight = 20.sp,
            letterSpacing = 0.1.sp,
        ),
    )
}
