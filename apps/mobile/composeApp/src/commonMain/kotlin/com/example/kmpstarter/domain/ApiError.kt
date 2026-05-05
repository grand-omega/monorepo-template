package com.example.kmpstarter.domain

/** Typed errors surfaced from API calls. Subclasses kotlin Throwable so they fit Result<T>. */
sealed class ApiError(message: String, cause: Throwable? = null) : Exception(message, cause) {

    /** No connection / DNS / timeouts — anything before we got an HTTP status. */
    class Network(message: String = "Network error", cause: Throwable? = null) : ApiError(message, cause)

    /** 422 validation failure with per-field error codes. */
    class Validation(
        val fields: Map<String, String>,
        message: String = "Validation failed",
    ) : ApiError(message)

    /** 401 specifically for the login endpoint. */
    data object InvalidCredentials : ApiError("Invalid email or password")

    /** 423 — account locked after too many failed attempts. */
    class AccountLocked(message: String = "Account temporarily locked") : ApiError(message)

    /** 429 — too many requests. */
    class RateLimited(message: String = "Too many requests, slow down") : ApiError(message)

    /** 401 on a non-login endpoint — generic "session expired / not authorized". */
    data object Unauthorized : ApiError("Session expired")

    /** 404. */
    data object NotFound : ApiError("Not found")

    /** Anything else — surface the raw code+message so the UI can show something. */
    class Unknown(
        val httpCode: Int,
        val code: String,
        message: String = "Unexpected error",
    ) : ApiError(message)
}

/** Project-wide alias for convenience. */
typealias ApiResult<T> = Result<T>
