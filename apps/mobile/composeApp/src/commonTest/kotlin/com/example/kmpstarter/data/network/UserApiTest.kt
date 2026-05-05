package com.example.kmpstarter.data.network

import com.example.kmpstarter.domain.ApiError
import io.ktor.client.engine.mock.MockRequestHandleScope
import io.ktor.client.plugins.auth.providers.BearerTokens
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class UserApiTest {

    private val userJson = """
        {"id":"u-1","email":"u@example.com","display_name":"Alice","avatar_url":"https://cdn.example.com/u-1.png","email_verified":true}
    """.trimIndent()

    private fun authedClient(handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData) =
        mockClient(
            tokens = object : BearerTokenProvider {
                override suspend fun load(): BearerTokens = BearerTokens("ax", "rx")
                override suspend fun refresh(): BearerTokens? = null
            },
            handler = handler,
        )

    @Test
    fun `getMe attaches bearer token automatically`() = runTest {
        var authHeader: String? = null
        val client = authedClient { request ->
            authHeader = request.headers[HttpHeaders.Authorization]
            respondJson(body = userJson)
        }

        val u = UserApi(client).getMe().getOrThrow()
        assertEquals("Alice", u.displayName)
        assertEquals("https://cdn.example.com/u-1.png", u.avatarUrl)
        assertEquals("Bearer ax", authHeader)
    }

    @Test
    fun `patchMe sends display_name and returns updated user`() = runTest {
        val client = authedClient { _ -> respondJson(body = userJson) }
        val u = UserApi(client).patchMe(displayName = "Bob").getOrThrow()
        assertEquals("u-1", u.id)
    }

    @Test
    fun `getMe 401 with no refresh maps to Unauthorized`() = runTest {
        val client = authedClient { _ ->
            respondJson(
                status = HttpStatusCode.Unauthorized,
                body = """{"code":"unauthorized","message":"go away"}""",
            )
        }

        val err = UserApi(client).getMe().exceptionOrNull()
        assertEquals(ApiError.Unauthorized, err)
    }

    @Test
    fun `changePassword returns new TokenPair`() = runTest {
        val client = authedClient { _ ->
            respondJson(
                body = """
                    {"access_token":"ax2","refresh_token":"rx2","token_type":"Bearer",
                     "access_expires_in":900,
                     "user":{"id":"u-1","email":"u@example.com","display_name":null,"email_verified":true}}
                """.trimIndent(),
            )
        }

        val pair = UserApi(client).changePassword("oldpw", "newpw").getOrThrow()
        assertEquals("ax2", pair.accessToken)
    }
}
