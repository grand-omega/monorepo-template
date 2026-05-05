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
import com.example.kmpstarter.ui.screens.forgotpassword.ForgotPasswordScreen
import com.example.kmpstarter.ui.screens.home.HomeScreen
import com.example.kmpstarter.ui.screens.login.LoginScreen
import com.example.kmpstarter.ui.screens.profile.ProfileScreen
import com.example.kmpstarter.ui.screens.register.RegisterScreen
import com.example.kmpstarter.ui.screens.resetpassword.ResetPasswordScreen
import com.example.kmpstarter.ui.screens.verifyemail.VerifyEmailScreen

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
            VerifyEmailScreen(
                email = args.email,
                prefilledToken = args.prefilledToken,
                onVerified = {
                    navController.navigate(AuthRoute.Login) {
                        popUpTo(AuthRoute.Login) { inclusive = true }
                    }
                },
                onSkipToLogin = {
                    navController.navigate(AuthRoute.Login) {
                        popUpTo(AuthRoute.Login) { inclusive = true }
                    }
                },
            )
        }
        composable<AuthRoute.ForgotPassword> {
            ForgotPasswordScreen(
                onBackToLogin = { navController.popBackStack() },
                onOpenReset = { navController.navigate(AuthRoute.ResetPassword()) },
            )
        }
        composable<AuthRoute.ResetPassword> { entry ->
            val args = entry.toRoute<AuthRoute.ResetPassword>()
            ResetPasswordScreen(
                prefilledToken = args.prefilledToken,
                onResetComplete = {
                    navController.navigate(AuthRoute.Login) {
                        popUpTo(AuthRoute.Login) { inclusive = true }
                    }
                },
                onBackToLogin = {
                    navController.navigate(AuthRoute.Login) {
                        popUpTo(AuthRoute.Login) { inclusive = true }
                    }
                },
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
        startDestination = MainRoute.Home,
        modifier = modifier,
    ) {
        composable<MainRoute.Home> {
            HomeScreen(
                user = user,
                onOpenProfile = { navController.navigate(MainRoute.Profile) },
                onVerifyEmail = { navController.navigate(AuthRoute.VerifyEmail(email = user.email)) },
            )
        }
        composable<MainRoute.Profile> {
            ProfileScreen(
                user = user,
                onBack = { navController.popBackStack() },
            )
        }
        composable<AuthRoute.VerifyEmail> { entry ->
            val args = entry.toRoute<AuthRoute.VerifyEmail>()
            VerifyEmailScreen(
                email = args.email,
                prefilledToken = args.prefilledToken,
                onVerified = {
                    navController.navigate(MainRoute.Home) {
                        popUpTo(MainRoute.Home) { inclusive = true }
                    }
                },
                onSkipToLogin = { navController.popBackStack() },
            )
        }
        composable<AuthRoute.ResetPassword> { entry ->
            val args = entry.toRoute<AuthRoute.ResetPassword>()
            ResetPasswordScreen(
                prefilledToken = args.prefilledToken,
                onResetComplete = { navController.popBackStack() },
                onBackToLogin = { navController.popBackStack() },
            )
        }
    }
}
