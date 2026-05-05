package com.example.kmpstarter.ui.screens.login

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

data class LoginUiState(
    val email: String = "",
    val password: String = "",
    val emailError: String? = null,
    val passwordError: String? = null,
    val serverStatus: ServerStatus = ServerStatus.Unknown,
    val isLoading: Boolean = false,
    val snackbar: String? = null,
)

enum class ServerStatus {
    Unknown,
    Checking,
    Online,
    Offline,
}

class LoginViewModel(
    private val authRepository: AuthRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow(LoginUiState())
    val uiState: StateFlow<LoginUiState> = _uiState.asStateFlow()

    fun onEmailChange(email: String) {
        _uiState.update { it.copy(email = email, emailError = null) }
    }

    fun onPasswordChange(password: String) {
        _uiState.update { it.copy(password = password, passwordError = null) }
    }

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun refreshServerStatus() {
        if (_uiState.value.serverStatus == ServerStatus.Checking) return

        viewModelScope.launch {
            _uiState.update { it.copy(serverStatus = ServerStatus.Checking) }
            val reachable = authRepository.checkServerReachable()
            _uiState.update {
                it.copy(serverStatus = if (reachable) ServerStatus.Online else ServerStatus.Offline)
            }
        }
    }

    fun onSubmit() {
        val state = _uiState.value
        if (state.isLoading) return

        val emailError = Validators.emailError(state.email)
        val passwordError = Validators.passwordError(state.password)
        if (emailError != null || passwordError != null) {
            _uiState.update { it.copy(emailError = emailError, passwordError = passwordError) }
            return
        }

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, snackbar = null) }
            val result = authRepository.login(state.email.trim(), state.password)
            result.onFailure { handleError(it) }
            _uiState.update { it.copy(isLoading = false) }
            // On success: AuthRepository.state flips to Authenticated; the
            // outer App composable swaps NavHosts, so this screen unmounts.
        }
    }

    private fun handleError(error: Throwable) {
        when (error) {
            ApiError.InvalidCredentials ->
                _uiState.update { it.copy(snackbar = "Invalid email or password") }
            is ApiError.AccountLocked ->
                _uiState.update { it.copy(snackbar = error.message ?: "Account locked. Try again later.") }
            is ApiError.RateLimited ->
                _uiState.update { it.copy(snackbar = "Too many attempts. Wait a bit and try again.") }
            is ApiError.Validation -> {
                val emailErr = error.fields["email"]
                    ?.let { Validators.apiFieldErrorMessage("email", it) }
                val pwErr = error.fields["password"]
                    ?.let { Validators.apiFieldErrorMessage("password", it) }
                _uiState.update {
                    it.copy(
                        emailError = emailErr ?: it.emailError,
                        passwordError = pwErr ?: it.passwordError,
                        snackbar = if (emailErr == null && pwErr == null) {
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
