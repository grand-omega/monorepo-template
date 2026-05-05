package com.example.kmpstarter.domain

/** Client-side input validation. The server is the source of truth — these are early hints. */
object Validators {
    private val emailRegex = Regex("^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$")

    const val PASSWORD_MIN_LENGTH = 8

    fun emailError(email: String): String? = when {
        email.isBlank() -> "Email is required"
        !emailRegex.matches(email.trim()) -> "Enter a valid email"
        else -> null
    }

    fun passwordError(password: String): String? = when {
        password.isBlank() -> "Password is required"
        password.length < PASSWORD_MIN_LENGTH -> "Use at least $PASSWORD_MIN_LENGTH characters"
        else -> null
    }

    fun passwordMatchError(password: String, confirm: String): String? = when {
        confirm.isBlank() -> "Re-enter the password"
        password != confirm -> "Passwords don't match"
        else -> null
    }

    fun displayNameError(displayName: String): String? = when {
        displayName.length > 64 -> "Display name is too long"
        else -> null
    }

    fun verificationTokenError(token: String): String? = when {
        token.isBlank() -> "Paste the token from the email"
        else -> null
    }

    /** Map a server-side field error code to a user-facing message. */
    fun apiFieldErrorMessage(field: String, code: String): String = when ("$field:$code") {
        "email:format", "email:invalid" -> "Enter a valid email"
        "email:taken", "email:duplicate", "email:already_exists" -> "Email is already in use"
        "password:too_short" -> "Password is too short"
        "password:too_weak" -> "Choose a stronger password"
        "password:mismatch", "password:wrong" -> "Wrong password"
        "current_password:wrong" -> "Current password is incorrect"
        "display_name:too_long" -> "Display name is too long"
        "token:invalid", "token:expired" -> "This link has expired — request a new one"
        else -> "${field.replaceFirstChar { it.uppercase() }}: $code"
    }
}
