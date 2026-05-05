package com.example.kmpstarter.domain

/** Top-level session state observed by the navigation host. */
sealed interface AuthState {
    /** App is restoring a session from storage; show splash. */
    data object Loading : AuthState

    /** No valid refresh token — user must log in or register. */
    data object Unauthenticated : AuthState

    /** Active session. */
    data class Authenticated(val user: User) : AuthState
}
