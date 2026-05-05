package com.example.kmpstarter.ui.screens.forgotpassword

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.Validators
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ForgotPasswordUiState(
    val email: String = "",
    val emailError: String? = null,
    val isLoading: Boolean = false,
    val sent: Boolean = false,
    val snackbar: String? = null,
)

class ForgotPasswordViewModel(
    private val authRepository: AuthRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(ForgotPasswordUiState())
    val uiState: StateFlow<ForgotPasswordUiState> = _uiState.asStateFlow()

    fun onEmailChange(email: String) {
        _uiState.update { it.copy(email = email, emailError = null) }
    }

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun onSubmit() {
        val state = _uiState.value
        if (state.isLoading) return

        val emailError = Validators.emailError(state.email)
        if (emailError != null) {
            _uiState.update { it.copy(emailError = emailError) }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, snackbar = null) }
            authRepository.passwordResetRequest(state.email.trim()).fold(
                onSuccess = { _uiState.update { it.copy(sent = true, isLoading = false) } },
                onFailure = { error ->
                    _uiState.update { it.copy(isLoading = false) }
                    handleError(error)
                },
            )
        }
    }

    private fun handleError(error: Throwable) {
        when (error) {
            is ApiError.Validation -> {
                val emailErr = error.fields["email"]
                    ?.let { Validators.apiFieldErrorMessage("email", it) }
                _uiState.update { it.copy(emailError = emailErr, snackbar = if (emailErr == null) error.message else null) }
            }
            is ApiError.RateLimited ->
                _uiState.update { it.copy(snackbar = "Too many attempts. Wait a bit and try again.") }
            is ApiError.Network ->
                _uiState.update { it.copy(snackbar = "Connection error. Check your network.") }
            else ->
                _uiState.update { it.copy(snackbar = error.message ?: "Something went wrong.") }
        }
    }
}
