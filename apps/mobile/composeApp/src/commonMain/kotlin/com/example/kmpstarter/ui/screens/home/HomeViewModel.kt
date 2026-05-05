package com.example.kmpstarter.ui.screens.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.user.UserRepository
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.User
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class HomeUiState(
    val isRefreshing: Boolean = false,
    val snackbar: String? = null,
)

class HomeViewModel(
    private val userRepository: UserRepository,
    private val authRepository: AuthRepository,
) : ViewModel() {
    private val _uiState = MutableStateFlow(HomeUiState())
    val uiState: StateFlow<HomeUiState> = _uiState.asStateFlow()

    fun consumeSnackbar() {
        _uiState.update { it.copy(snackbar = null) }
    }

    fun refresh() {
        if (_uiState.value.isRefreshing) return
        viewModelScope.launch {
            _uiState.update { it.copy(isRefreshing = true, snackbar = null) }
            userRepository.fetchMe().onFailure { error ->
                _uiState.update {
                    it.copy(snackbar = if (error is ApiError.Network) "Connection error. Check your network." else error.message)
                }
            }
            _uiState.update { it.copy(isRefreshing = false) }
        }
    }

    fun logout() {
        viewModelScope.launch {
            authRepository.logout(allDevices = false)
        }
    }
}
