package com.example.kmpstarter.ui.screens.verifyemail

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.Validators
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlin.time.Duration.Companion.seconds

data class VerifyEmailUiState(
    val email: String = "",
    val token: String = "",
    val tokenError: String? = null,
    val isVerifying: Boolean = false,
    val isResending: Boolean = false,
    val resendCooldownSeconds: Int = 0,
    val snackbar: String? = null,
)

sealed interface VerifyEmailEvent {
    data object Verified : VerifyEmailEvent
}

class VerifyEmailViewModel(
    private val authRepository: AuthRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow(VerifyEmailUiState())
    val uiState: StateFlow<VerifyEmailUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<VerifyEmailEvent>(
        replay = 0,
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    val events: SharedFlow<VerifyEmailEvent> = _events.asSharedFlow()

    private var cooldownJob: Job? = null
    private var argsApplied = false

    /**
     * Called once when the screen mounts to apply navigation arguments.
     * If a [prefilledToken] is supplied (deep link), submit immediately.
     */
    fun applyArgs(email: String?, prefilledToken: String?) {
        if (argsApplied) return
        argsApplied = true
        _uiState.update {
            it.copy(
                email = email.orEmpty(),
                token = prefilledToken.orEmpty(),
            )
        }
        if (!prefilledToken.isNullOrBlank()) {
            onSubmit()
        }
    }

    fun onTokenChange(value: String) {
        _uiState.update { it.copy(token = value, tokenError = null) }
    }

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun onSubmit() {
        val state = _uiState.value
        if (state.isVerifying) return

        val tokenError = Validators.verificationTokenError(state.token)
        if (tokenError != null) {
            _uiState.update { it.copy(tokenError = tokenError) }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isVerifying = true, snackbar = null) }
            val result = authRepository.verifyEmail(state.token.trim())
            result.fold(
                onSuccess = {
                    _uiState.update { it.copy(isVerifying = false) }
                    _events.tryEmit(VerifyEmailEvent.Verified)
                },
                onFailure = { error ->
                    handleVerifyError(error)
                    _uiState.update { it.copy(isVerifying = false) }
                },
            )
        }
    }

    fun onResend() {
        val state = _uiState.value
        if (state.isResending || state.resendCooldownSeconds > 0) return

        val emailError = Validators.emailError(state.email)
        if (emailError != null) {
            _uiState.update { it.copy(snackbar = "Add your email at the top to resend") }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isResending = true, snackbar = null) }
            val result = authRepository.resendVerification(state.email.trim())
            result.fold(
                onSuccess = {
                    _uiState.update { it.copy(isResending = false, snackbar = "Sent — check your inbox") }
                    startCooldown(60)
                },
                onFailure = { error ->
                    handleResendError(error)
                    _uiState.update { it.copy(isResending = false) }
                },
            )
        }
    }

    private fun startCooldown(seconds: Int) {
        cooldownJob?.cancel()
        // Set the initial value synchronously so the UI can disable the
        // resend button immediately. The tick loop continues asynchronously.
        _uiState.update { it.copy(resendCooldownSeconds = seconds) }
        cooldownJob = viewModelScope.launch {
            for (s in (seconds - 1) downTo 0) {
                delay(1.seconds)
                _uiState.update { it.copy(resendCooldownSeconds = s) }
            }
        }
    }

    private fun handleVerifyError(error: Throwable) {
        when (error) {
            ApiError.Unauthorized ->
                _uiState.update {
                    it.copy(tokenError = "This link has expired — request a new one.")
                }
            is ApiError.Validation -> {
                val tokenErr = error.fields["token"]
                    ?.let { Validators.apiFieldErrorMessage("token", it) }
                _uiState.update { it.copy(tokenError = tokenErr ?: "Token couldn't be read.") }
            }
            is ApiError.Network ->
                _uiState.update { it.copy(snackbar = "Connection error. Check your network.") }
            else ->
                _uiState.update { it.copy(snackbar = error.message ?: "Something went wrong.") }
        }
    }

    private fun handleResendError(error: Throwable) {
        when (error) {
            is ApiError.RateLimited ->
                _uiState.update { it.copy(snackbar = "Already sent. Try again in a minute.") }
            is ApiError.Network ->
                _uiState.update { it.copy(snackbar = "Connection error. Check your network.") }
            else ->
                _uiState.update { it.copy(snackbar = error.message ?: "Couldn't send the email.") }
        }
    }
}
