package com.example.kmpstarter.ui.screens.resetpassword

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.Validators
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ResetPasswordUiState(
    val token: String = "",
    val newPassword: String = "",
    val confirmPassword: String = "",
    val tokenError: String? = null,
    val newPasswordError: String? = null,
    val confirmPasswordError: String? = null,
    val isLoading: Boolean = false,
    val snackbar: String? = null,
)

sealed interface ResetPasswordEvent {
    data object ResetComplete : ResetPasswordEvent
}

class ResetPasswordViewModel(
    private val authRepository: AuthRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(ResetPasswordUiState())
    val uiState: StateFlow<ResetPasswordUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<ResetPasswordEvent>(
        replay = 0,
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    val events: SharedFlow<ResetPasswordEvent> = _events.asSharedFlow()

    private var argsApplied = false

    fun applyArgs(prefilledToken: String?) {
        if (argsApplied) return
        argsApplied = true
        if (!prefilledToken.isNullOrBlank()) {
            _uiState.update { it.copy(token = prefilledToken) }
        }
    }

    fun onTokenChange(token: String) {
        _uiState.update { it.copy(token = token, tokenError = null) }
    }

    fun onNewPasswordChange(password: String) {
        _uiState.update { it.copy(newPassword = password, newPasswordError = null, confirmPasswordError = null) }
    }

    fun onConfirmPasswordChange(password: String) {
        _uiState.update { it.copy(confirmPassword = password, confirmPasswordError = null) }
    }

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun onSubmit() {
        val state = _uiState.value
        if (state.isLoading) return

        val tokenError = Validators.verificationTokenError(state.token)
        val passwordError = Validators.passwordError(state.newPassword)
        val matchError = Validators.passwordMatchError(state.newPassword, state.confirmPassword)
        if (tokenError != null || passwordError != null || matchError != null) {
            _uiState.update {
                it.copy(
                    tokenError = tokenError,
                    newPasswordError = passwordError,
                    confirmPasswordError = matchError,
                )
            }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, snackbar = null) }
            authRepository.passwordResetConfirm(state.token.trim(), state.newPassword).fold(
                onSuccess = {
                    _uiState.update { it.copy(isLoading = false) }
                    _events.tryEmit(ResetPasswordEvent.ResetComplete)
                },
                onFailure = { error ->
                    _uiState.update { it.copy(isLoading = false) }
                    handleError(error)
                },
            )
        }
    }

    private fun handleError(error: Throwable) {
        when (error) {
            ApiError.Unauthorized ->
                _uiState.update { it.copy(tokenError = "This link has expired — request a new one.") }
            is ApiError.Validation -> {
                val tokenErr = error.fields["token"]?.let { Validators.apiFieldErrorMessage("token", it) }
                val passwordErr = error.fields["new_password"]?.let { Validators.apiFieldErrorMessage("password", it) }
                _uiState.update {
                    it.copy(
                        tokenError = tokenErr,
                        newPasswordError = passwordErr,
                        snackbar = if (tokenErr == null && passwordErr == null) error.message else null,
                    )
                }
            }
            is ApiError.Network ->
                _uiState.update { it.copy(snackbar = "Connection error. Check your network.") }
            else ->
                _uiState.update { it.copy(snackbar = error.message ?: "Something went wrong.") }
        }
    }
}
