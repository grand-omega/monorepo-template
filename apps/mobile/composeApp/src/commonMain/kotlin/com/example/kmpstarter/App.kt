package com.example.kmpstarter

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.domain.AuthState
import com.example.kmpstarter.ui.navigation.AuthNavHost
import com.example.kmpstarter.ui.navigation.DeepLinkAction
import com.example.kmpstarter.ui.navigation.MainNavHost
import com.example.kmpstarter.ui.screens.AppSplashScreen
import com.example.kmpstarter.ui.theme.AppTheme
import org.koin.compose.koinInject

@Composable
fun App(
    deepLink: MutableState<DeepLinkAction?> = remember { mutableStateOf(null) },
) {
    AppTheme {
        Surface(modifier = Modifier.fillMaxSize(), color = androidx.compose.material3.MaterialTheme.colorScheme.background) {
            val authRepository: AuthRepository = koinInject()
            val authState by authRepository.state.collectAsState()

            LaunchedEffect(Unit) { authRepository.bootstrap() }

            AnimatedContent(
                targetState = authState,
                transitionSpec = { fadeIn(animationSpec = androidx.compose.animation.core.tween(220)) togetherWith
                    fadeOut(animationSpec = androidx.compose.animation.core.tween(180)) },
                contentKey = { state ->
                    when (state) {
                        AuthState.Loading -> 0
                        AuthState.Unauthenticated -> 1
                        is AuthState.Authenticated -> 2
                    }
                },
                label = "auth-state",
            ) { state ->
                when (state) {
                    AuthState.Loading -> AppSplashScreen()
                    AuthState.Unauthenticated -> AuthNavHost(deepLink = deepLink)
                    is AuthState.Authenticated -> MainNavHost(user = state.user, deepLink = deepLink)
                }
            }
        }
    }
}
