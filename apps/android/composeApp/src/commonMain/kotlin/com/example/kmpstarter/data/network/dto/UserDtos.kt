package com.example.kmpstarter.data.network.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class UserSummaryDto(
    val id: String,
    val email: String,
    @SerialName("display_name") val displayName: String? = null,
    @SerialName("email_verified") val emailVerified: Boolean,
)

@Serializable
data class PatchMeRequestDto(
    @SerialName("display_name") val displayName: String? = null,
)

@Serializable
data class ChangePasswordRequestDto(
    @SerialName("current_password") val currentPassword: String,
    @SerialName("new_password") val newPassword: String,
)

@Serializable
data class DeleteMeRequestDto(
    val password: String,
)
