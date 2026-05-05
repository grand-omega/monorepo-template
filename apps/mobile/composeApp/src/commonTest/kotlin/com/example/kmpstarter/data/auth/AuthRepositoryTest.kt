package com.example.kmpstarter.data.auth

import app.cash.turbine.test
import com.example.kmpstarter.data.network.AuthApi
import com.example.kmpstarter.data.network.mockClient
import com.example.kmpstarter.data.network.respondJson
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.AuthState
import io.ktor.client.engine.mock.MockRequestHandleScope
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.HttpStatusCode
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AuthRepositoryTest {

    private val tokenPairJson = """
        {"access_token":"ax","refresh_token":"rx","token_type":"Bearer","access_expires_in":900,
         "user":{"id":"u-1","email":"u@example.com","display_name":null,"avatar_url":"https://cdn.example.com/u-1.png","email_verified":false}}
    """.trimIndent()

    private val rotatedTokenPairJson = """
        {"access_token":"ax2","refresh_token":"rx2","token_type":"Bearer","access_expires_in":900,
         "user":{"id":"u-1","email":"u@example.com","display_name":null,"avatar_url":null,"email_verified":false}}
    """.trimIndent()

    private fun authApi(handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData): AuthApi =
        AuthApi(mockClient(handler = handler))

    @Test
    fun `bootstrap with empty storage transitions to Unauthenticated`() = runTest {
        val repo = AuthRepository(
            authApi = authApi { respondJson(HttpStatusCode.NotFound, """{"code":"x","message":"x"}""") },
            tokenStorage = FakeTokenStorage(initial = null),
        )
        repo.state.test {
            assertEquals(AuthState.Loading, awaitItem())
            repo.bootstrap()
            assertEquals(AuthState.Unauthenticated, awaitItem())
            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test
    fun `bootstrap with stored refresh and 200 transitions to Authenticated`() = runTest {
        val storage = FakeTokenStorage(initial = "stored-rt")
        val repo = AuthRepository(
            authApi = authApi { respondJson(body = tokenPairJson) },
            tokenStorage = storage,
        )

        repo.state.test {
            assertEquals(AuthState.Loading, awaitItem())
            repo.bootstrap()
            val authed = assertIs<AuthState.Authenticated>(awaitItem())
            assertEquals("u@example.com", authed.user.email)
            assertEquals("https://cdn.example.com/u-1.png", authed.user.avatarUrl)
            cancelAndIgnoreRemainingEvents()
        }

        // The new refresh token from the rotation should have been persisted.
        assertEquals("rx", storage.stored)
    }

    @Test
    fun `bootstrap with stored refresh and 401 clears storage and Unauthenticates`() = runTest {
        val storage = FakeTokenStorage(initial = "expired-rt")
        val repo = AuthRepository(
            authApi = authApi {
                respondJson(HttpStatusCode.Unauthorized, """{"code":"invalid_token","message":"nope"}""")
            },
            tokenStorage = storage,
        )

        repo.state.test {
            assertEquals(AuthState.Loading, awaitItem())
            repo.bootstrap()
            assertEquals(AuthState.Unauthenticated, awaitItem())
            cancelAndIgnoreRemainingEvents()
        }
        assertNull(storage.stored)
        assertEquals(1, storage.clearCount)
    }

    @Test
    fun `login success emits Authenticated and persists refresh token`() = runTest {
        val storage = FakeTokenStorage()
        val repo = AuthRepository(
            authApi = authApi { request ->
                assertEquals("/v1/auth/login", request.url.encodedPath)
                respondJson(body = tokenPairJson)
            },
            tokenStorage = storage,
        )

        repo.state.test {
            assertEquals(AuthState.Loading, awaitItem())
            val result = repo.login("u@example.com", "pw")
            assertTrue(result.isSuccess)
            val authed = assertIs<AuthState.Authenticated>(awaitItem())
            assertEquals("u-1", authed.user.id)
            cancelAndIgnoreRemainingEvents()
        }
        assertEquals("rx", storage.stored)
    }

    @Test
    fun `login 401 returns InvalidCredentials and leaves state alone`() = runTest {
        val storage = FakeTokenStorage(initial = null)
        // Bootstrap first so we leave Loading state cleanly.
        val repo = AuthRepository(
            authApi = authApi { request ->
                when (request.url.encodedPath) {
                    "/v1/auth/login" -> respondJson(
                        HttpStatusCode.Unauthorized,
                        """{"code":"invalid_credentials","message":"bad"}""",
                    )
                    else -> respondJson(body = tokenPairJson)
                }
            },
            tokenStorage = storage,
        )
        repo.bootstrap()

        val result = repo.login("u@example.com", "wrong")
        assertEquals(ApiError.InvalidCredentials, result.exceptionOrNull())
        assertEquals(AuthState.Unauthenticated, repo.state.value)
        assertNull(storage.stored)
    }

    @Test
    fun `logout single-device clears local session`() = runTest {
        val storage = FakeTokenStorage(initial = "stored-rt")
        val repo = AuthRepository(
            authApi = authApi { request ->
                when (request.url.encodedPath) {
                    "/v1/auth/refresh" -> respondJson(body = tokenPairJson)
                    "/v1/auth/logout" -> respondJson(body = """{"status":"ok"}""")
                    else -> respondJson(HttpStatusCode.NotFound, """{"code":"x","message":"x"}""")
                }
            },
            tokenStorage = storage,
        )
        repo.bootstrap()
        assertIs<AuthState.Authenticated>(repo.state.value)

        val result = repo.logout(allDevices = false)
        assertTrue(result.isSuccess)
        assertEquals(AuthState.Unauthenticated, repo.state.value)
        assertNull(storage.stored)
    }

    @Test
    fun `verifyEmail flips emailVerified on the in-memory user`() = runTest {
        val storage = FakeTokenStorage(initial = "stored-rt")
        val repo = AuthRepository(
            authApi = authApi { request ->
                when (request.url.encodedPath) {
                    "/v1/auth/refresh" -> respondJson(body = tokenPairJson)
                    "/v1/auth/verify-email" -> respondJson(body = """{"status":"verified"}""")
                    else -> respondJson(HttpStatusCode.NotFound, """{"code":"x","message":"x"}""")
                }
            },
            tokenStorage = storage,
        )
        repo.bootstrap()
        val before = assertIs<AuthState.Authenticated>(repo.state.value)
        assertEquals(false, before.user.emailVerified)

        val result = repo.verifyEmail("token")
        assertTrue(result.isSuccess)
        val after = assertIs<AuthState.Authenticated>(repo.state.value)
        assertEquals(true, after.user.emailVerified)
    }

    @Test
    fun `refresh() rotates tokens and returns new BearerTokens`() = runTest {
        val storage = FakeTokenStorage(initial = "rt-old")
        var refreshHits = 0
        val repo = AuthRepository(
            authApi = authApi { _ ->
                refreshHits++
                respondJson(body = if (refreshHits == 1) tokenPairJson else rotatedTokenPairJson)
            },
            tokenStorage = storage,
        )
        repo.bootstrap()
        assertEquals("rx", storage.stored)

        val rotated = repo.refresh()
        assertEquals("ax2", rotated?.accessToken)
        assertEquals("rx2", storage.stored)
    }

    @Test
    fun `refresh() returns null and clears session on 401`() = runTest {
        val storage = FakeTokenStorage(initial = "rt-old")
        var refreshHits = 0
        val repo = AuthRepository(
            authApi = authApi { _ ->
                refreshHits++
                if (refreshHits == 1) {
                    respondJson(body = tokenPairJson)
                } else {
                    respondJson(HttpStatusCode.Unauthorized, """{"code":"invalid_token","message":"x"}""")
                }
            },
            tokenStorage = storage,
        )
        repo.bootstrap()
        assertIs<AuthState.Authenticated>(repo.state.value)

        val rotated = repo.refresh()
        assertNull(rotated)
        assertEquals(AuthState.Unauthenticated, repo.state.value)
        assertNull(storage.stored)
    }

    @Test
    fun `register returns success without changing auth state`() = runTest {
        val repo = AuthRepository(
            authApi = authApi { _ ->
                respondJson(HttpStatusCode.Accepted, """{"status":"accepted"}""")
            },
            tokenStorage = FakeTokenStorage(initial = null),
        )
        repo.bootstrap()
        val before = repo.state.value

        val result = repo.register("u@example.com", "secret123", displayName = null)

        assertTrue(result.isSuccess)
        assertEquals(before, repo.state.value)
    }

    @Test
    fun `passwordResetRequest returns success when server 202s`() = runTest {
        val repo = AuthRepository(
            authApi = authApi { _ ->
                respondJson(HttpStatusCode.Accepted, """{"status":"accepted"}""")
            },
            tokenStorage = FakeTokenStorage(initial = null),
        )
        val result = repo.passwordResetRequest("u@example.com")
        assertTrue(result.isSuccess)
    }
}
