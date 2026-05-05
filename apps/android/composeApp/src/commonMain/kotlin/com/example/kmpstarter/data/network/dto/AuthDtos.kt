package com.example.kmpstarter.data.network.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class RegisterRequestDto(
    val email: String,
    val password: String,
    @SerialName("display_name") val displayName: String? = null,
)

@Serializable
data class LoginRequestDto(
    val email: String,
    val password: String,
)

@Serializable
data class RefreshRequestDto(
    @SerialName("refresh_token") val refreshToken: String,
)

@Serializable
data class LogoutRequestDto(
    @SerialName("refresh_token") val refreshToken: String,
)

@Serializable
data class VerifyEmailRequestDto(
    val token: String,
)

@Serializable
data class ResendVerificationRequestDto(
    val email: String,
)

@Serializable
data class PasswordResetRequestDto(
    val email: String,
)

@Serializable
data class PasswordResetConfirmRequestDto(
    val token: String,
    @SerialName("new_password") val newPassword: String,
)

@Serializable
data class TokenPairDto(
    @SerialName("access_token") val accessToken: String,
    @SerialName("refresh_token") val refreshToken: String,
    @SerialName("token_type") val tokenType: String,
    @SerialName("access_expires_in") val accessExpiresInSeconds: Long,
    val user: UserSummaryDto,
)

@Serializable
data class AcceptedResponseDto(
    val status: String,
)
