package com.example.kmpstarter.data.network

import com.example.kmpstarter.data.network.dto.AcceptedResponseDto
import com.example.kmpstarter.data.network.dto.ChangePasswordRequestDto
import com.example.kmpstarter.data.network.dto.DeleteMeRequestDto
import com.example.kmpstarter.data.network.dto.PatchMeRequestDto
import com.example.kmpstarter.data.network.dto.TokenPairDto
import com.example.kmpstarter.data.network.dto.UserSummaryDto
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.request.delete
import io.ktor.client.request.get
import io.ktor.client.request.patch
import io.ktor.client.request.setBody

/** Wraps the authenticated /v1/me endpoints. Caller must supply a client whose Auth bearer is wired up. */
class UserApi(private val client: HttpClient) {

    suspend fun getMe(): Result<UserSummaryDto> =
        runCatching { client.get("/v1/me").body<UserSummaryDto>() }
            .recoverCatching { throw mapToApiError(it) }

    suspend fun patchMe(displayName: String?): Result<UserSummaryDto> =
        runCatching {
            client.patch("/v1/me") {
                setBody(PatchMeRequestDto(displayName = displayName))
            }.body<UserSummaryDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun deleteMe(password: String): Result<AcceptedResponseDto> =
        runCatching {
            client.delete("/v1/me") {
                setBody(DeleteMeRequestDto(password = password))
            }.body<AcceptedResponseDto>()
        }.recoverCatching { throw mapToApiError(it) }

    suspend fun changePassword(currentPassword: String, newPassword: String): Result<TokenPairDto> =
        runCatching {
            client.patch("/v1/me/password") {
                setBody(ChangePasswordRequestDto(currentPassword = currentPassword, newPassword = newPassword))
            }.body<TokenPairDto>()
        }.recoverCatching { throw mapToApiError(it) }
}
