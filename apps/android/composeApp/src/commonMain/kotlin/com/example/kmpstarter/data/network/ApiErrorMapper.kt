package com.example.kmpstarter.data.network

import com.example.kmpstarter.data.network.dto.ErrorBodyDto
import com.example.kmpstarter.domain.ApiError
import io.ktor.client.call.body
import io.ktor.client.plugins.ClientRequestException
import io.ktor.client.plugins.ServerResponseException
import io.ktor.client.statement.HttpResponse
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.JsonConvertException
import kotlinx.serialization.SerializationException

/**
 * Maps an arbitrary thrown error from a Ktor call into a typed ApiError.
 *
 * @param onLoginEndpoint when true, 401 is interpreted as InvalidCredentials
 *  rather than a generic "session expired".
 */
suspend fun mapToApiError(throwable: Throwable, onLoginEndpoint: Boolean = false): ApiError =
    when (throwable) {
        is ApiError -> throwable
        is ClientRequestException -> mapHttpError(throwable.response, onLoginEndpoint)
        is ServerResponseException -> ApiError.Unknown(
            httpCode = throwable.response.status.value,
            code = "server_error",
            message = "Server error (${throwable.response.status.value})",
        )
        is JsonConvertException, is SerializationException -> ApiError.Unknown(
            httpCode = 0,
            code = "serialization",
            message = "Could not parse server response",
        )
        // Any other throwable is treated as a network/transport failure (timeouts,
        // connection refused, DNS, broken socket, etc.). Specific subclasses live in
        // platform-specific Ktor packages and aren't all exposed in commonMain.
        else -> ApiError.Network(throwable.message ?: "Network error", throwable)
    }

private suspend fun mapHttpError(response: HttpResponse, onLoginEndpoint: Boolean): ApiError {
    val body = runCatching { response.body<ErrorBodyDto>() }.getOrNull()
    return when (response.status) {
        HttpStatusCode.UnprocessableEntity -> ApiError.Validation(
            fields = body?.fields?.associate { it.field to it.code }.orEmpty(),
            message = body?.message ?: "Validation failed",
        )
        HttpStatusCode.Unauthorized -> if (onLoginEndpoint) {
            ApiError.InvalidCredentials
        } else {
            ApiError.Unauthorized
        }
        HttpStatusCode.Locked -> ApiError.AccountLocked(body?.message ?: "Account locked")
        HttpStatusCode.TooManyRequests -> ApiError.RateLimited(body?.message ?: "Too many requests")
        HttpStatusCode.NotFound -> ApiError.NotFound
        else -> ApiError.Unknown(
            httpCode = response.status.value,
            code = body?.code ?: "http_${response.status.value}",
            message = body?.message ?: response.status.description,
        )
    }
}
