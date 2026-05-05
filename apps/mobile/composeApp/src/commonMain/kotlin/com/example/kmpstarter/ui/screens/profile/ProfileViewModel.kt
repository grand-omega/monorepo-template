package com.example.kmpstarter.ui.screens.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.user.UserRepository
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.Validators
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ProfileUiState(
    val currentPassword: String = "",
    val newPassword: String = "",
    val deletePassword: String = "",
    val currentPasswordError: String? = null,
    val newPasswordError: String? = null,
    val deletePasswordError: String? = null,
    val changingPassword: Boolean = false,
    val loggingOut: Boolean = false,
    val deleting: Boolean = false,
    val snackbar: String? = null,
)

class ProfileViewModel(
    private val userRepository: UserRepository,
    private val authRepository: AuthRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(ProfileUiState())
    val uiState: StateFlow<ProfileUiState> = _uiState.asStateFlow()

    fun onCurrentPasswordChange(value: String) {
        _uiState.update { it.copy(currentPassword = value, currentPasswordError = null) }
    }

    fun onNewPasswordChange(value: String) {
        _uiState.update { it.copy(newPassword = value, newPasswordError = null) }
    }

    fun onDeletePasswordChange(value: String) {
        _uiState.update { it.copy(deletePassword = value, deletePasswordError = null) }
    }

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun changePassword() {
        val state = _uiState.value
        val currentError = Validators.passwordError(state.currentPassword)
        val newError = Validators.passwordError(state.newPassword)
        if (currentError != null || newError != null) {
            _uiState.update { it.copy(currentPasswordError = currentError, newPasswordError = newError) }
            return
        }
        viewModelScope.launch {
            _uiState.update { it.copy(changingPassword = true, snackbar = null) }
            userRepository.changePassword(state.currentPassword, state.newPassword).fold(
                onSuccess = {
                    _uiState.update {
                        it.copy(
                            changingPassword = false,
                            currentPassword = "",
                            newPassword = "",
                            snackbar = "Password changed",
                        )
                    }
                },
                onFailure = { error ->
                    _uiState.update { it.copy(changingPassword = false) }
                    handlePasswordError(error)
                },
            )
        }
    }

    fun logout(allDevices: Boolean) {
        viewModelScope.launch {
            _uiState.update { it.copy(loggingOut = true) }
            authRepository.logout(allDevices)
        }
    }

    fun deleteAccount() {
        val state = _uiState.value
        val passwordError = Validators.passwordError(state.deletePassword)
        if (passwordError != null) {
            _uiState.update { it.copy(deletePasswordError = passwordError) }
            return
        }
        viewModelScope.launch {
            _uiState.update { it.copy(deleting = true, snackbar = null) }
            userRepository.deleteAccount(state.deletePassword).fold(
                onSuccess = { },
                onFailure = { error ->
                    _uiState.update { it.copy(deleting = false) }
                    handleDeleteError(error)
                },
            )
        }
    }

    private fun handlePasswordError(error: Throwable) {
        when (error) {
            is ApiError.Validation -> {
                val currentErr = error.fields["current_password"]?.let { Validators.apiFieldErrorMessage("current_password", it) }
                val newErr = error.fields["new_password"]?.let { Validators.apiFieldErrorMessage("password", it) }
                _uiState.update {
                    it.copy(
                        currentPasswordError = currentErr,
                        newPasswordError = newErr,
                        snackbar = if (currentErr == null && newErr == null) error.message else null,
                    )
                }
            }
            is ApiError.Network -> _uiState.update { it.copy(snackbar = "Connection error. Check your network.") }
            else -> _uiState.update { it.copy(snackbar = error.message ?: "Couldn't change password.") }
        }
    }

    private fun handleDeleteError(error: Throwable) {
        when (error) {
            is ApiError.Validation -> {
                val passwordErr = error.fields["password"]?.let { Validators.apiFieldErrorMessage("password", it) }
                _uiState.update { it.copy(deletePasswordError = passwordErr, snackbar = if (passwordErr == null) error.message else null) }
            }
            is ApiError.Network -> _uiState.update { it.copy(snackbar = "Connection error. Check your network.") }
            else -> _uiState.update { it.copy(snackbar = error.message ?: "Couldn't delete account.") }
        }
    }
}
