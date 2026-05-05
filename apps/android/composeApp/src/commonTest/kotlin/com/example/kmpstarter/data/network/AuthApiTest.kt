package com.example.kmpstarter.data.network

import com.example.kmpstarter.domain.ApiError
import io.ktor.http.HttpStatusCode
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlin.test.fail

class AuthApiTest {

    private val tokenPairJson = """
        {
          "access_token": "ax",
          "refresh_token": "rx",
          "token_type": "Bearer",
          "access_expires_in": 900,
          "user": {
            "id": "u-1",
            "email": "u@example.com",
            "display_name": null,
            "email_verified": false
          }
        }
    """.trimIndent()

    @Test
    fun `login success returns TokenPair`() = runTest {
        val client = mockClient { request ->
            assertEquals("/v1/auth/login", request.url.encodedPath)
            respondJson(body = tokenPairJson)
        }

        val result = AuthApi(client).login("u@example.com", "secret")

        val pair = result.getOrThrow()
        assertEquals("ax", pair.accessToken)
        assertEquals("u-1", pair.user.id)
    }

    @Test
    fun `login 401 maps to InvalidCredentials`() = runTest {
        val client = mockClient { _ ->
            respondJson(
                status = HttpStatusCode.Unauthorized,
                body = """{"code":"invalid_credentials","message":"bad creds"}""",
            )
        }

        val result = AuthApi(client).login("u@example.com", "wrong")
        val err = result.exceptionOrNull() ?: fail("expected failure")
        assertEquals(ApiError.InvalidCredentials, err)
    }

    @Test
    fun `login 422 maps to Validation with field errors`() = runTest {
        val client = mockClient { _ ->
            respondJson(
                status = HttpStatusCode.UnprocessableEntity,
                body = """
                    {"code":"validation_failed","message":"bad",
                     "fields":[{"field":"email","code":"format"}]}
                """.trimIndent(),
            )
        }

        val err = AuthApi(client).login("notanemail", "pw").exceptionOrNull()
        val v = assertIs<ApiError.Validation>(err)
        assertEquals("format", v.fields["email"])
    }

    @Test
    fun `login 423 maps to AccountLocked`() = runTest {
        val client = mockClient { _ ->
            respondJson(
                status = HttpStatusCode.Locked,
                body = """{"code":"locked","message":"locked"}""",
            )
        }

        val err = AuthApi(client).login("u", "p").exceptionOrNull()
        assertIs<ApiError.AccountLocked>(err)
    }

    @Test
    fun `login 429 maps to RateLimited`() = runTest {
        val client = mockClient { _ ->
            respondJson(
                status = HttpStatusCode.TooManyRequests,
                body = """{"code":"rate_limited","message":"slow"}""",
            )
        }

        val err = AuthApi(client).login("u", "p").exceptionOrNull()
        assertIs<ApiError.RateLimited>(err)
    }

    @Test
    fun `register hits register endpoint and returns Accepted`() = runTest {
        var seenPath = ""
        val client = mockClient { request ->
            seenPath = request.url.encodedPath
            respondJson(
                status = HttpStatusCode.Accepted,
                body = """{"status":"accepted"}""",
            )
        }

        val ok = AuthApi(client).register("u@example.com", "secret123", displayName = "Alice").getOrThrow()
        assertEquals("/v1/auth/register", seenPath)
        assertEquals("accepted", ok.status)
    }

    @Test
    fun `refresh 401 maps to Unauthorized (not InvalidCredentials)`() = runTest {
        val client = mockClient { _ ->
            respondJson(
                status = HttpStatusCode.Unauthorized,
                body = """{"code":"invalid_token","message":"nope"}""",
            )
        }

        val err = AuthApi(client).refresh("bad").exceptionOrNull()
        assertEquals(ApiError.Unauthorized, err)
    }

    @Test
    fun `verifyEmail returns success on 200`() = runTest {
        val client = mockClient { request ->
            assertEquals("/v1/auth/verify-email", request.url.encodedPath)
            respondJson(body = """{"status":"ok"}""")
        }

        val ok = AuthApi(client).verifyEmail("tok").getOrThrow()
        assertEquals("ok", ok.status)
    }

    @Test
    fun `passwordResetRequest returns 202`() = runTest {
        val client = mockClient { request ->
            assertEquals("/v1/auth/password-reset/request", request.url.encodedPath)
            respondJson(status = HttpStatusCode.Accepted, body = """{"status":"accepted"}""")
        }
        val ok = AuthApi(client).passwordResetRequest("u@example.com").getOrThrow()
        assertEquals("accepted", ok.status)
    }

    @Test
    fun `passwordResetConfirm 401 maps to Unauthorized`() = runTest {
        val client = mockClient { _ ->
            respondJson(
                status = HttpStatusCode.Unauthorized,
                body = """{"code":"invalid_token","message":"expired"}""",
            )
        }
        val err = AuthApi(client).passwordResetConfirm("bad", "newpw1234").exceptionOrNull()
        assertEquals(ApiError.Unauthorized, err)
    }

    @Test
    fun `network failure maps to Network`() = runTest {
        val client = mockClient { _ -> error("simulated socket close") }
        val err = AuthApi(client).login("u", "p").exceptionOrNull()
        val net = assertIs<ApiError.Network>(err)
        assertTrue(net.message?.contains("simulated") == true)
    }

    @Test
    fun `malformed success body maps to Unknown`() = runTest {
        val client = mockClient { _ -> respondJson(body = "{not-json:}") }
        val err = AuthApi(client).login("u", "p").exceptionOrNull()
        val unk = assertIs<ApiError.Unknown>(err)
        assertEquals("serialization", unk.code)
    }
}
