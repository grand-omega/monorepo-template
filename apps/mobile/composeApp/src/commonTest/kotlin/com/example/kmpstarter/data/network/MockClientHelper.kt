package com.example.kmpstarter.data.network

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.MockRequestHandleScope
import io.ktor.client.engine.mock.respond
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf

internal const val TEST_BASE_URL = "http://localhost:1"

/** Build a test client that uses MockEngine but otherwise matches production wiring. */
internal fun mockClient(
    tokens: BearerTokenProvider? = null,
    handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData,
): HttpClient {
    val engine = MockEngine(handler)
    return buildHttpClient(baseUrl = TEST_BASE_URL, tokens = tokens, engine = engine)
}

internal fun MockRequestHandleScope.respondJson(
    status: HttpStatusCode = HttpStatusCode.OK,
    body: String,
): HttpResponseData = respond(
    content = body,
    status = status,
    headers = headersOf(HttpHeaders.ContentType, ContentType.Application.Json.toString()),
)
