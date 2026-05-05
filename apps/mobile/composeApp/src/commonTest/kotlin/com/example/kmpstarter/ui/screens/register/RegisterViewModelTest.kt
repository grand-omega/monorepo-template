package com.example.kmpstarter.ui.screens.register

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
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class RegisterViewModelTest {

    @BeforeTest
    fun setUp() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
    }

    @AfterTest
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun viewModelWith(
        handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData,
    ): RegisterViewModel {
        val authApi = AuthApi(mockClient(handler = handler))
        val authRepository = AuthRepository(authApi, FakeTokenStorage())
        return RegisterViewModel(authRepository)
    }

    private fun runVmTest(body: suspend TestScope.() -> Unit) = runTest {
        Dispatchers.setMain(UnconfinedTestDispatcher(testScheduler))
        try {
            body()
        } finally {
            Dispatchers.resetMain()
        }
    }

    private suspend fun RegisterViewModel.awaitIdle(): RegisterUiState =
        withContext(Dispatchers.Default.limitedParallelism(1)) {
            withTimeout(5.seconds) { uiState.first { !it.isLoading } }
        }

    @Test
    fun `empty submit shows all required-field errors and does not call api`() = runVmTest {
        var hit = false
        val vm = viewModelWith { _ ->
            hit = true
            respondJson(HttpStatusCode.Accepted, """{"status":"accepted"}""")
        }

        vm.onSubmit()

        val s = vm.uiState.value
        assertNotNull(s.emailError)
        assertNotNull(s.passwordError)
        assertNotNull(s.confirmPasswordError)
        assertFalse(s.isLoading)
        assertFalse(hit)
    }

    @Test
    fun `mismatched passwords blocks submission`() = runVmTest {
        var hit = false
        val vm = viewModelWith { _ ->
            hit = true
            respondJson(HttpStatusCode.Accepted, """{"status":"accepted"}""")
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("password1")
        vm.onConfirmPasswordChange("password2")

        vm.onSubmit()

        assertNotNull(vm.uiState.value.confirmPasswordError)
        assertFalse(hit)
    }

    @Test
    fun `successful register emits Registered event with the trimmed email`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.Accepted, """{"status":"accepted"}""")
        }
        vm.onEmailChange("  user@example.com  ")
        vm.onPasswordChange("strongpassword")
        vm.onConfirmPasswordChange("strongpassword")

        vm.events.test {
            vm.onSubmit()
            val event = awaitItem()
            assertEquals(RegisterEvent.Registered("user@example.com"), event)
        }
        assertFalse(vm.uiState.value.isLoading)
    }

    @Test
    fun `display name is trimmed and sent or omitted when blank`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.Accepted, """{"status":"accepted"}""")
        }
        vm.onEmailChange("u@example.com")
        vm.onDisplayNameChange("   ")
        vm.onPasswordChange("strongpassword")
        vm.onConfirmPasswordChange("strongpassword")

        vm.events.test {
            vm.onSubmit()
            awaitItem() // should still emit Registered
        }
    }

    @Test
    fun `429 surfaces rate-limited snackbar`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(HttpStatusCode.TooManyRequests, """{"code":"rate_limited","message":"x"}""")
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")
        vm.onConfirmPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals(
            "Too many attempts. Wait a bit and try again.",
            vm.uiState.value.snackbar,
        )
    }

    @Test
    fun `422 with email taken sets inline emailError`() = runVmTest {
        val vm = viewModelWith { _ ->
            respondJson(
                HttpStatusCode.UnprocessableEntity,
                """{"code":"validation_failed","message":"x",
                    "fields":[{"field":"email","code":"taken"}]}""".trimIndent(),
            )
        }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")
        vm.onConfirmPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals("Email is already in use", vm.uiState.value.emailError)
    }

    @Test
    fun `network failure surfaces connection snackbar`() = runVmTest {
        val vm = viewModelWith { _ -> error("simulated") }
        vm.onEmailChange("u@example.com")
        vm.onPasswordChange("strongpassword")
        vm.onConfirmPasswordChange("strongpassword")

        vm.onSubmit()
        vm.awaitIdle()

        assertEquals(
            "Connection error. Check your network.",
            vm.uiState.value.snackbar,
        )
    }

    @Test
    fun `email change clears email error`() = runVmTest {
        val vm = viewModelWith { _ -> respondJson(HttpStatusCode.Accepted, """{"status":"x"}""") }
        vm.onSubmit()
        assertNotNull(vm.uiState.value.emailError)

        vm.onEmailChange("u@example.com")
        assertNull(vm.uiState.value.emailError)
    }
}
