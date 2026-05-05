package com.example.kmpstarter.ui.screens.login

import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.auth.FakeTokenStorage
import com.example.kmpstarter.data.network.AuthApi
import com.example.kmpstarter.data.network.mockClient
import com.example.kmpstarter.data.network.respondJson
import io.ktor.client.engine.mock.MockRequestHandleScope
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.HttpStatusCode
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlin.time.Duration.Companion.seconds
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull

@OptIn(ExperimentalCoroutinesApi::class)
class LoginViewModelTest {

    private val tokenPairJson = """
        {"access_token":"ax","refresh_token":"rx","token_type":"Bearer","access_expires_in":900,
         "user":{"id":"u-1","email":"u@example.com","display_name":null,"email_verified":true}}
    """.trimIndent()

    private fun viewModelWith(
        handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData,
    ): LoginViewModel {
        val authApi = AuthApi(mockClient(handler = handler))
        val authRepository = AuthRepository(authApi, FakeTokenStorage())
        return LoginViewModel(authRepository)
    }

    /** Run a test where viewModelScope's Dispatchers.Main is driven by runTest's scheduler. */
    private fun runVmTest(body: suspend TestScope.() -> Unit) = runTest {
        val dispatcher: CoroutineDispatcher = UnconfinedTestDispatcher(testScheduler)
        Dispatchers.setMain(dispatcher)
        try {
            body()
        } finally {
            Dispatchers.resetMain()
        }
    }

    /**
     * Wait for [LoginViewModel.onSubmit]'s launched coroutine to finish.
     *
     * Ktor MockEngine runs on Dispatchers.IO, which is invisible to
     * runTest's virtual scheduler — `withTimeout` on the test scheduler
     * would tick instantly. Bridge to real time by hopping off the test
     * dispatcher.
     */
    private suspend fun LoginViewModel.awaitIdle(): LoginUiState =
        withContext(Dispatchers.Default.limitedParallelism(1)) {
            withTimeout(5.seconds) { uiState.first { !it.isLoading } }
        }

    @Test
    fun `initial state is blank`() = runVmTest {
        val vm = viewModelWith { _ -> respondJson(body = tokenPairJson) }
        val s = vm.uiState.value
        assertEquals("", s.email)
        assertEquals("", s.password)
        assertNull(s.emailError)
        assertNull(s.passwordError)
        assertFalse(s.isLoading)
        assertNull(s.snackbar)
    }

    @Test
    fun `onEmailChange updates email and clears error`() = runVmTest {
        val vm = viewModelWith { _ -> respondJson(body = tokenPairJson) }
        vm.onSubmit() // empty fields → validation errors, no coroutine launched
        assertNotNull(vm.uiState.value.emailError)

        vm.onEmailChange("u@example.com")
        assertEquals("u@example.com", vm.uiState.value.email)
        assertNull(vm.uiState.value.emailError)
    }

    @Test
    fun `onSubmit with empty fields shows validation errors and does not call api`() = runVmTest {
        var called = false
        val vm = viewModelWith { _ ->
            called = true
            respondJson(body = tokenPairJson)
        }

        vm.onSubmit()
        vm.awaitIdle()

        val s = vm.uiState.value
        assertNotNull(s.emailError)
        assertNotNull(s.passwordError)
        assertFalse(s.isLoading)
        assertFalse(called)
    }

    @Test
    fun `successful login leaves screen in non-loading clean state`() = runVmTest {
        val vm = viewModelWith { _ -> respondJson(body = tokenPairJson) }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        val s = vm.uiState.value
        assertFalse(s.isLoading)
        assertNull(s.snackbar)
        assertNull(s.emailError)
        assertNull(s.passwordError)
    }

    @Test
    fun `401 surfaces InvalidCredentials snackbar`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(
                HttpStatusCode.Unauthorized,
                """{"code":"invalid_credentials","message":"x"}""",
            )
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals("Invalid email or password", vm.uiState.value.snackbar)
    }

    @Test
    fun `422 with email field error sets inline emailError`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(
                HttpStatusCode.UnprocessableEntity,
                """{"code":"validation_failed","message":"x",
                    "fields":[{"field":"email","code":"format"}]}""".trimIndent(),
            )
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals("Enter a valid email", vm.uiState.value.emailError)
    }

    @Test
    fun `423 surfaces account-locked snackbar`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.Locked, """{"code":"locked","message":"locked!"}""")
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals("locked!", vm.uiState.value.snackbar)
    }

    @Test
    fun `429 surfaces rate-limited snackbar`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.TooManyRequests, """{"code":"rate_limited","message":"x"}""")
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals(
            "Too many attempts. Wait a bit and try again.",
            vm.uiState.value.snackbar,
        )
    }

    @Test
    fun `network failure surfaces connection snackbar`() = runVmTest {
        val vm = viewModelWith { _ -> error("simulated socket close") }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals(
            "Connection error. Check your network.",
            vm.uiState.value.snackbar,
        )
    }

    @Test
    fun `consumeSnackbar clears snackbar`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.Unauthorized, """{"code":"x","message":"x"}""")
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")
        vm.onSubmit()
        vm.awaitIdle()
        assertNotNull(vm.uiState.value.snackbar)

        vm.consumeSnackbar()
        assertNull(vm.uiState.value.snackbar)
    }
}
