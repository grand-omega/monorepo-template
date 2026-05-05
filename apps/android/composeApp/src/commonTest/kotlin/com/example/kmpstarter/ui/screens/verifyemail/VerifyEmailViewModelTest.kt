package com.example.kmpstarter.ui.screens.verifyemail

import app.cash.turbine.test
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.auth.FakeTokenStorage
import com.example.kmpstarter.data.network.AuthApi
import com.example.kmpstarter.data.network.mockClient
import com.example.kmpstarter.data.network.respondJson
import io.ktor.client.engine.mock.MockRequestHandleScope
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.HttpStatusCode
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class VerifyEmailViewModelTest {

    private fun viewModelWith(
        handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData,
    ): VerifyEmailViewModel {
        val authApi = AuthApi(mockClient(handler = handler))
        val authRepository = AuthRepository(authApi, FakeTokenStorage())
        return VerifyEmailViewModel(authRepository)
    }

    private fun runVmTest(body: suspend TestScope.() -> Unit) = runTest {
        Dispatchers.setMain(UnconfinedTestDispatcher(testScheduler))
        try {
            body()
        } finally {
            Dispatchers.resetMain()
        }
    }

    private suspend fun VerifyEmailViewModel.awaitVerifyDone(): VerifyEmailUiState =
        withContext(Dispatchers.Default.limitedParallelism(1)) {
            withTimeout(5.seconds) { uiState.first { !it.isVerifying && it.tokenError != null } }
        }

    private suspend fun VerifyEmailViewModel.awaitIdle(): VerifyEmailUiState =
        withContext(Dispatchers.Default.limitedParallelism(1)) {
            withTimeout(5.seconds) { uiState.first { !it.isVerifying && !it.isResending } }
        }

    private suspend fun VerifyEmailViewModel.awaitResendDone(): VerifyEmailUiState =
        withContext(Dispatchers.Default.limitedParallelism(1)) {
            withTimeout(5.seconds) {
                uiState.first { !it.isResending && (it.resendCooldownSeconds > 0 || it.snackbar != null) }
            }
        }

    @Test
    fun `applyArgs sets email and does not auto-submit when token is null`() = runVmTest {
        var hits = 0
        val vm = viewModelWith { _ ->
            hits++
            respondJson(body = """{"status":"ok"}""")
        }

        vm.applyArgs(email = "u@example.com", prefilledToken = null)

        assertEquals("u@example.com", vm.uiState.value.email)
        assertEquals(0, hits)
    }

    @Test
    fun `applyArgs with prefilled token auto-submits and emits Verified`() = runVmTest {
        val vm = viewModelWith { request ->
            assertEquals("/v1/auth/verify-email", request.url.encodedPath)
            respondJson(body = """{"status":"ok"}""")
        }

        vm.events.test {
            vm.applyArgs(email = "u@example.com", prefilledToken = "tok-123")
            assertEquals(VerifyEmailEvent.Verified, awaitItem())
        }
    }

    @Test
    fun `applyArgs is idempotent across reconfigurations`() = runVmTest {
        var hits = 0
        val vm = viewModelWith { _ ->
            hits++
            respondJson(body = """{"status":"ok"}""")
        }

        vm.events.test {
            vm.applyArgs(email = "u@example.com", prefilledToken = "tok-123")
            awaitItem()
        }
        // Second call (e.g. from a re-composition) should not re-submit.
        vm.applyArgs(email = "u@example.com", prefilledToken = "tok-123")
        vm.awaitIdle()

        assertEquals(1, hits)
    }

    @Test
    fun `submit with empty token sets tokenError`() = runVmTest {
        val vm = viewModelWith { _ -> respondJson(body = """{"status":"ok"}""") }
        vm.onSubmit()
        assertNotNull(vm.uiState.value.tokenError)
    }

    @Test
    fun `submit with 401 marks token as expired`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.Unauthorized, """{"code":"invalid_token","message":"x"}""")
        }
        vm.onTokenChange("bad-token")
        vm.onSubmit()
        vm.awaitVerifyDone()
        val msg = vm.uiState.value.tokenError
        assertNotNull(msg)
        assertTrue(msg.contains("expired", ignoreCase = true))
    }

    @Test
    fun `resend without email surfaces a hint snackbar`() = runVmTest {
        var hits = 0
        val vm = viewModelWith { _ ->
            hits++
            respondJson(HttpStatusCode.Accepted, """{"status":"ok"}""")
        }
        vm.applyArgs(email = null, prefilledToken = null)

        vm.onResend()

        assertEquals(0, hits)
        assertNotNull(vm.uiState.value.snackbar)
    }

    @Test
    fun `resend with email + 202 starts cooldown`() = runVmTest {
        val vm = viewModelWith { request ->
            assertEquals("/v1/auth/resend-verification", request.url.encodedPath)
            respondJson(HttpStatusCode.Accepted, """{"status":"ok"}""")
        }
        vm.applyArgs(email = "u@example.com", prefilledToken = null)

        vm.onResend()
        vm.awaitResendDone()

        assertTrue(vm.uiState.value.resendCooldownSeconds > 0)
        advanceTimeBy(60.seconds)
        runCurrent()
    }

    @Test
    fun `resend with 429 surfaces rate-limited copy`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.TooManyRequests, """{"code":"rate_limited","message":"x"}""")
        }
        vm.applyArgs(email = "u@example.com", prefilledToken = null)

        vm.onResend()
        vm.awaitResendDone()

        assertEquals("Already sent. Try again in a minute.", vm.uiState.value.snackbar)
    }

    @Test
    fun `onTokenChange clears token error`() = runVmTest {
        val vm = viewModelWith { _ -> respondJson(body = """{"status":"ok"}""") }
        vm.onSubmit()
        assertNotNull(vm.uiState.value.tokenError)

        vm.onTokenChange("anything")
        assertEquals(null, vm.uiState.value.tokenError)
    }
}
