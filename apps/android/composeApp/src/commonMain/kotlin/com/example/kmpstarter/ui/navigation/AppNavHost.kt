package com.example.kmpstarter.ui.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import com.example.kmpstarter.domain.User
import com.example.kmpstarter.ui.screens.PlaceholderScreen
import com.example.kmpstarter.ui.screens.login.LoginScreen
import com.example.kmpstarter.ui.screens.register.RegisterScreen

/** Auth (signed-out) graph. */
@Composable
fun AuthNavHost(
    deepLink: MutableState<DeepLinkAction?>,
    modifier: Modifier = Modifier,
    navController: NavHostController = rememberNavController(),
) {
    // Consume any pending deep link by jumping to the relevant destination.
    val pending by deepLink
    LaunchedEffect(pending) {
        when (val action = pending) {
            is DeepLinkAction.VerifyEmail -> {
                navController.navigate(AuthRoute.VerifyEmail(prefilledToken = action.token))
                deepLink.value = null
            }
            is DeepLinkAction.ResetPassword -> {
                navController.navigate(AuthRoute.ResetPassword(prefilledToken = action.token))
                deepLink.value = null
            }
            null -> Unit
        }
    }

    NavHost(
        navController = navController,
        startDestination = AuthRoute.Login,
        modifier = modifier,
    ) {
        composable<AuthRoute.Login> {
            LoginScreen(
                onNavigateToRegister = { navController.navigate(AuthRoute.Register) },
                onNavigateToForgotPassword = { navController.navigate(AuthRoute.ForgotPassword) },
            )
        }
        composable<AuthRoute.Register> {
            RegisterScreen(
                onRegistered = { email ->
                    navController.navigate(AuthRoute.VerifyEmail(email = email)) {
                        popUpTo(AuthRoute.Login) { inclusive = false }
                    }
                },
                onBackToLogin = { navController.popBackStack() },
            )
        }
        composable<AuthRoute.VerifyEmail> { entry ->
            val args = entry.toRoute<AuthRoute.VerifyEmail>()
            PlaceholderScreen(
                title = "Verify your email",
                body = "Email: ${args.email ?: "—"} · token: ${args.prefilledToken ?: "—"}",
            )
        }
        composable<AuthRoute.ForgotPassword> {
            PlaceholderScreen(title = "Forgot password", body = "Forgot-password screen.")
        }
        composable<AuthRoute.ResetPassword> { entry ->
            val args = entry.toRoute<AuthRoute.ResetPassword>()
            PlaceholderScreen(
                title = "Reset password",
                body = "Token: ${args.prefilledToken ?: "—"}",
            )
        }
    }
}

/** Main (signed-in) graph. */
@Composable
fun MainNavHost(
    user: User,
    deepLink: MutableState<DeepLinkAction?>,
    modifier: Modifier = Modifier,
    navController: NavHostController = rememberNavController(),
) {
    // While signed in, route the verify-email deep link to a future verify
    // destination. For now, surface the token as an event the screen can show.
    val pending by deepLink
    LaunchedEffect(pending) {
        if (pending is DeepLinkAction.VerifyEmail) {
            // Home screen will handle it once milestone 9 lands; just pass through.
        }
    }

    NavHost(
        navController = navController,
        startDestination = MainRoute.Home,
        modifier = modifier,
    ) {
        composable<MainRoute.Home> {
            PlaceholderScreen(
                title = "Welcome, ${user.displayName ?: user.email}",
                body = "Home screen lands here in milestone 9.",
            )
        }
        composable<MainRoute.Profile> {
            PlaceholderScreen(title = "Profile", body = "Profile screen.")
        }
    }
}
