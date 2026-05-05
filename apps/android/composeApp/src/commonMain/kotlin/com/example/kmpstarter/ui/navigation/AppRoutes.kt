package com.example.kmpstarter.ui.navigation

import kotlinx.serialization.Serializable

/** Type-safe destinations for the auth (signed-out) navigation graph. */
sealed interface AuthRoute {
    @Serializable data object Login : AuthRoute
    @Serializable data object Register : AuthRoute

    /**
     * @param email pre-filled in the screen if non-null (set after register).
     * @param prefilledToken auto-submitted when arriving from a deep link.
     */
    @Serializable data class VerifyEmail(
        val email: String? = null,
        val prefilledToken: String? = null,
    ) : AuthRoute

    @Serializable data object ForgotPassword : AuthRoute

    @Serializable data class ResetPassword(
        val prefilledToken: String? = null,
    ) : AuthRoute
}

/** Type-safe destinations for the main (signed-in) navigation graph. */
sealed interface MainRoute {
    @Serializable data object Home : MainRoute
    @Serializable data object Profile : MainRoute
}
