package com.example.kmpstarter.ui.navigation

/** External-link intents the app needs to react to. */
sealed interface DeepLinkAction {
    /** kmpstarter://verify#token=... */
    data class VerifyEmail(val token: String) : DeepLinkAction

    /** kmpstarter://reset#token=... */
    data class ResetPassword(val token: String) : DeepLinkAction
}
