package com.example.kmpstarter.data.network

import co.touchlab.kermit.Logger
import io.ktor.client.HttpClient
import io.ktor.client.HttpClientConfig
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.plugins.DefaultRequest
import io.ktor.client.plugins.HttpTimeout
import io.ktor.client.plugins.auth.Auth
import io.ktor.client.plugins.auth.providers.BearerTokens
import io.ktor.client.plugins.auth.providers.bearer
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logging
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.URLBuilder
import io.ktor.http.contentType
import io.ktor.http.takeFrom
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.json.Json

/** JSON config tuned for the API: ignore unknown keys, encode defaults. */
val ApiJson: Json = Json {
    ignoreUnknownKeys = true
    encodeDefaults = true
    explicitNulls = false
    isLenient = false
}

/** Hooks the Ktor Auth bearer plugin into an external token provider/refresher. */
interface BearerTokenProvider {
    /** Read tokens currently stored. Return null when the user is not signed in. */
    suspend fun load(): BearerTokens?

    /** Called by Ktor on 401 to obtain a fresh token pair. Return null to give up. */
    suspend fun refresh(): BearerTokens?
}

/**
 * Build a single Ktor HttpClient configured for the API.
 *
 * @param baseUrl  the API root (e.g. http://localhost:8080)
 * @param tokens   optional bearer-token provider — when null, the client makes
 *                 unauthenticated calls only. When non-null, /v1 protected
 *                 calls automatically attach the access token and on 401 the
 *                 refresh hook is invoked once.
 * @param engine   override the platform engine (used by tests with MockEngine).
 * @param logger   Kermit logger for outbound traffic.
 */
fun buildHttpClient(
    baseUrl: String,
    tokens: BearerTokenProvider? = null,
    engine: HttpClientEngine? = null,
    logger: Logger = Logger.withTag("HTTP"),
): HttpClient {
    val configure: HttpClientConfig<*>.() -> Unit = {
        expectSuccess = true

        install(ContentNegotiation) { json(ApiJson) }

        install(HttpTimeout) {
            connectTimeoutMillis = 15_000
            requestTimeoutMillis = 30_000
            socketTimeoutMillis = 30_000
        }

        install(Logging) {
            this.logger = object : io.ktor.client.plugins.logging.Logger {
                override fun log(message: String) {
                    logger.d { redactTokens(message) }
                }
            }
            level = LogLevel.INFO
        }

        if (tokens != null) {
            install(Auth) {
                bearer {
                    loadTokens { tokens.load() }
                    refreshTokens { tokens.refresh() }
                    sendWithoutRequest { request ->
                        // Don't preemptively send tokens to public auth endpoints.
                        val url = request.url.buildString()
                        !url.contains("/v1/auth/")
                    }
                }
            }
        }

        install(DefaultRequest) {
            url.takeFrom(URLBuilder().takeFrom(baseUrl))
            headers.append(HttpHeaders.Accept, ContentType.Application.Json.toString())
            contentType(ContentType.Application.Json)
        }
    }

    return if (engine != null) HttpClient(engine, configure) else HttpClient(httpClientEngine(), configure)
}

private val tokenLineRegex = Regex("(?i)(\"(?:access|refresh)_token\"\\s*:\\s*\")([^\"]+)(\")")
private val authHeaderRegex = Regex("(?i)(Authorization:\\s*Bearer\\s+)(\\S+)")

/** Strip token values out of log lines. */
internal fun redactTokens(line: String): String =
    line
        .replace(tokenLineRegex) { "${it.groupValues[1]}<redacted>${it.groupValues[3]}" }
        .replace(authHeaderRegex) { "${it.groupValues[1]}<redacted>" }
