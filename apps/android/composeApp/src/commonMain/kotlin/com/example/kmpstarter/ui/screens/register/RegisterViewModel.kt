package com.example.kmpstarter.ui.screens.register

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

data class RegisterUiState(
    val email: String = "",
    val displayName: String = "",
    val password: String = "",
    val confirmPassword: String = "",
    val emailError: String? = null,
    val displayNameError: String? = null,
    val passwordError: String? = null,
    val confirmPasswordError: String? = null,
    val isLoading: Boolean = false,
    val snackbar: String? = null,
)

sealed interface RegisterEvent {
    /** Server accepted the registration; UI should jump to verify-email with this email. */
    data class Registered(val email: String) : RegisterEvent
}

class RegisterViewModel(
    private val authRepository: AuthRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow(RegisterUiState())
    val uiState: StateFlow<RegisterUiState> = _uiState.asStateFlow()

    private val _events = MutableSharedFlow<RegisterEvent>(
        replay = 0,
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    val events: SharedFlow<RegisterEvent> = _events.asSharedFlow()

    fun onEmailChange(value: String) {
        _uiState.update { it.copy(email = value, emailError = null) }
    }

    fun onDisplayNameChange(value: String) {
        _uiState.update { it.copy(displayName = value, displayNameError = null) }
    }

    fun onPasswordChange(value: String) {
        _uiState.update {
            it.copy(
                password = value,
                passwordError = null,
                // Re-validate match if confirm was already typed.
                confirmPasswordError = if (it.confirmPassword.isEmpty()) null else it.confirmPasswordError,
            )
        }
    }

    fun onConfirmPasswordChange(value: String) {
        _uiState.update { it.copy(confirmPassword = value, confirmPasswordError = null) }
    }

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun onSubmit() {
        val state = _uiState.value
        if (state.isLoading) return

        val emailError = Validators.emailError(state.email)
        val displayNameError = Validators.displayNameError(state.displayName)
        val passwordError = Validators.passwordError(state.password)
        val confirmError = Validators.passwordMatchError(state.password, state.confirmPassword)

        if (emailError != null || displayNameError != null || passwordError != null || confirmError != null) {
            _uiState.update {
                it.copy(
                    emailError = emailError,
                    displayNameError = displayNameError,
                    passwordError = passwordError,
                    confirmPasswordError = confirmError,
                )
            }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, snackbar = null) }

            val email = state.email.trim()
            val displayName = state.displayName.trim().takeIf { it.isNotEmpty() }
            val result = authRepository.register(email, state.password, displayName)

            result.fold(
                onSuccess = {
                    _uiState.update { it.copy(isLoading = false) }
                    _events.tryEmit(RegisterEvent.Registered(email))
                },
                onFailure = { error ->
                    handleError(error)
                    _uiState.update { it.copy(isLoading = false) }
                },
            )
        }
    }

    private fun handleError(error: Throwable) {
        when (error) {
            is ApiError.RateLimited ->
                _uiState.update { it.copy(snackbar = "Too many attempts. Wait a bit and try again.") }
            is ApiError.Validation -> {
                val emailErr = error.fields["email"]
                    ?.let { Validators.apiFieldErrorMessage("email", it) }
                val pwErr = error.fields["password"]
                    ?.let { Validators.apiFieldErrorMessage("password", it) }
                val nameErr = error.fields["display_name"]
                    ?.let { Validators.apiFieldErrorMessage("display_name", it) }
                _uiState.update {
                    it.copy(
                        emailError = emailErr ?: it.emailError,
                        passwordError = pwErr ?: it.passwordError,
                        displayNameError = nameErr ?: it.displayNameError,
                        snackbar = if (emailErr == null && pwErr == null && nameErr == null) {
                            error.message
                        } else {
                            null
                        },
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
