package com.example.kmpstarter.data.user

import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.auth.FakeTokenStorage
import com.example.kmpstarter.data.network.AuthApi
import com.example.kmpstarter.data.network.BearerTokenProvider
import com.example.kmpstarter.data.network.UserApi
import com.example.kmpstarter.data.network.mockClient
import com.example.kmpstarter.data.network.respondJson
import com.example.kmpstarter.domain.AuthState
import io.ktor.client.engine.mock.MockRequestHandleScope
import io.ktor.client.plugins.auth.providers.BearerTokens
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.HttpStatusCode
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class UserRepositoryTest {

    private val tokenPairJson = """
        {"access_token":"ax","refresh_token":"rx","token_type":"Bearer","access_expires_in":900,
         "user":{"id":"u-1","email":"u@example.com","display_name":null,"email_verified":true}}
    """.trimIndent()

    private val updatedUserJson = """
        {"id":"u-1","email":"u@example.com","display_name":"Bob","email_verified":true}
    """.trimIndent()

    private val rotatedTokenPairJson = """
        {"access_token":"ax2","refresh_token":"rx2","token_type":"Bearer","access_expires_in":900,
         "user":{"id":"u-1","email":"u@example.com","display_name":null,"email_verified":true}}
    """.trimIndent()

    private suspend fun setup(
        authedHandler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData,
    ): Triple<AuthRepository, UserRepository, FakeTokenStorage> {
        val storage = FakeTokenStorage(initial = "stored-rt")
        val authApi = AuthApi(mockClient { _ -> respondJson(body = tokenPairJson) })
        val authRepo = AuthRepository(authApi, storage)
        authRepo.bootstrap()

        // Authed client provides bearer from the live AuthRepository.
        val authedClient = mockClient(
            tokens = object : BearerTokenProvider {
                override suspend fun load(): BearerTokens? = authRepo.load()
                override suspend fun refresh(): BearerTokens? = authRepo.refresh()
            },
            handler = authedHandler,
        )
        val userApi = UserApi(authedClient)
        val userRepo = UserRepository(userApi, authRepo)
        return Triple(authRepo, userRepo, storage)
    }

    @Test
    fun `fetchMe returns user and updates AuthRepository state`() = runTest {
        val (authRepo, userRepo, _) = setup { _ -> respondJson(body = updatedUserJson) }

        val result = userRepo.fetchMe()
        val user = result.getOrThrow()

        assertEquals("Bob", user.displayName)
        val state = assertIs<AuthState.Authenticated>(authRepo.state.value)
        assertEquals("Bob", state.user.displayName)
    }

    @Test
    fun `updateDisplayName updates state user`() = runTest {
        val (authRepo, userRepo, _) = setup { _ -> respondJson(body = updatedUserJson) }

        userRepo.updateDisplayName("Bob").getOrThrow()
        val state = assertIs<AuthState.Authenticated>(authRepo.state.value)
        assertEquals("Bob", state.user.displayName)
    }

    @Test
    fun `changePassword swaps in the new TokenPair`() = runTest {
        val (authRepo, userRepo, storage) = setup { _ -> respondJson(body = rotatedTokenPairJson) }

        userRepo.changePassword("oldpw", "newpw").getOrThrow()

        // New refresh token persisted, in-memory bearer rotated.
        assertEquals("rx2", storage.stored)
        val tokens = authRepo.load()
        assertEquals("ax2", tokens?.accessToken)
        assertIs<AuthState.Authenticated>(authRepo.state.value)
    }

    @Test
    fun `deleteAccount clears local session`() = runTest {
        val (authRepo, userRepo, storage) = setup { _ ->
            respondJson(body = """{"status":"deleted"}""")
        }

        userRepo.deleteAccount("pw").getOrThrow()

        assertEquals(AuthState.Unauthenticated, authRepo.state.value)
        assertNull(storage.stored)
    }

    @Test
    fun `fetchMe 401 propagates failure`() = runTest {
        val (_, userRepo, _) = setup { _ ->
            respondJson(HttpStatusCode.Unauthorized, """{"code":"unauthorized","message":"x"}""")
        }
        // The Auth plugin will try refresh, which our mock public client will succeed at,
        // but the authed mock returns 401 again on retry — so we expect failure.
        val result = userRepo.fetchMe()
        assertTrue(result.isFailure)
    }
}
