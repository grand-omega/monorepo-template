package com.example.kmpstarter.data.network

import com.example.kmpstarter.data.network.dto.AcceptedResponseDto
import com.example.kmpstarter.data.network.dto.LoginRequestDto
import com.example.kmpstarter.data.network.dto.LogoutRequestDto
import com.example.kmpstarter.data.network.dto.PasswordResetConfirmRequestDto
import com.example.kmpstarter.data.network.dto.PasswordResetRequestDto
import com.example.kmpstarter.data.network.dto.RefreshRequestDto
import com.example.kmpstarter.data.network.dto.RegisterRequestDto
import com.example.kmpstarter.data.network.dto.ResendVerificationRequestDto
import com.example.kmpstarter.data.network.dto.TokenPairDto
import com.example.kmpstarter.data.network.dto.VerifyEmailRequestDto
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.ClientRequestException
import io.ktor.client.plugins.ServerResponseException
import io.ktor.client.request.get
import io.ktor.client.request.post
import io.ktor.client.request.setBody

/** Wraps the public auth endpoints under /v1/auth. Failures are [com.example.kmpstarter.domain.ApiError]. */
class AuthApi(private val client: HttpClient) {

    /**
     * Lightweight reachability probe for the configured API host.
     *
     * Any HTTP response means the server is reachable; transport failures
     * (connection refused, timeout, DNS) mean it is offline from the app.
     */
    suspend fun ping(): Result<Unit> =
        runCatching {
            client.get("/")
            Unit
        }.recoverCatching { error ->
            when (error) {
                is ClientRequestException, is ServerResponseException -> Unit
                else -> throw mapToApiError(error)
            }
        }

    suspend fun register(email: String, password: String, displayName: String?): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/register") {
                setBody(RegisterRequestDto(email = email, password = password, displayName = displayName))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun login(email: String, password: String): Result<TokenPairDto> =
        runCatching {
            client.post("/v1/auth/login") {
                setBody(LoginRequestDto(email = email, password = password))
            }.body<TokenPairDto>()
        }.recoverCatching { throw mapToApiError(it, onLoginEndpoint = true) }

    suspend fun refresh(refreshToken: String): Result<TokenPairDto> =
        runCatching {
            client.post("/v1/auth/refresh") {
                setBody(RefreshRequestDto(refreshToken = refreshToken))
            }.body<TokenPairDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun logout(refreshToken: String): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/logout") {
                setBody(LogoutRequestDto(refreshToken = refreshToken))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun logoutAll(): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/logout-all").body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun verifyEmail(token: String): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/verify-email") {
                setBody(VerifyEmailRequestDto(token = token))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun resendVerification(email: String): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/resend-verification") {
                setBody(ResendVerificationRequestDto(email = email))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun passwordResetRequest(email: String): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/password-reset/request") {
                setBody(PasswordResetRequestDto(email = email))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun passwordResetConfirm(token: String, newPassword: String): Result<AcceptedResponseDto> =
        runCatching {
            client.post("/v1/auth/password-reset/confirm") {
                setBody(PasswordResetConfirmRequestDto(token = token, newPassword = newPassword))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }
}
