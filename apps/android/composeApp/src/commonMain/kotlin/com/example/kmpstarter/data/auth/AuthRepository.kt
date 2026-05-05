package com.example.kmpstarter.data.auth

import com.example.kmpstarter.data.network.AuthApi
import com.example.kmpstarter.data.network.BearerTokenProvider
import com.example.kmpstarter.data.network.dto.TokenPairDto
import com.example.kmpstarter.domain.ApiError
import com.example.kmpstarter.domain.AuthState
import com.example.kmpstarter.domain.User
import io.ktor.client.plugins.auth.providers.BearerTokens
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * Owns the user's session state and the refresh-token rotation logic.
 *
 * Implements [BearerTokenProvider] so the authenticated Ktor client can
 * read the in-memory access token and trigger a refresh on 401.
 */
class AuthRepository(
    private val authApi: AuthApi,
    private val tokenStorage: TokenStorage,
) : BearerTokenProvider {

    private val _state = MutableStateFlow<AuthState>(AuthState.Loading)
    val state: StateFlow<AuthState> = _state.asStateFlow()

    private val mutex = Mutex()
    private var accessToken: String? = null
    private var refreshToken: String? = null
    private var currentUser: User? = null

    /**
     * Restore a session from disk on app start. Always transitions out of
     * [AuthState.Loading]: either to [AuthState.Authenticated] (refresh OK)
     * or [AuthState.Unauthenticated] (no token / refresh rejected).
     */
    suspend fun bootstrap() {
        val storedRefresh = tokenStorage.readRefreshToken()
        if (storedRefresh == null) {
            _state.value = AuthState.Unauthenticated
            return
        }
        mutex.withLock { refreshToken = storedRefresh }

        authApi.refresh(storedRefresh).fold(
            onSuccess = { applyTokenPair(it) },
            onFailure = { err ->
                if (err is ApiError.Unauthorized || err is ApiError.NotFound) {
                    clearLocalSession()
                }
                _state.value = AuthState.Unauthenticated
            },
        )
    }

    suspend fun register(email: String, password: String, displayName: String?): Result<Unit> =
        authApi.register(email, password, displayName).map { }

    suspend fun login(email: String, password: String): Result<User> =
        authApi.login(email, password).map { pair ->
            applyTokenPair(pair)
            pair.user.toUser()
        }

    suspend fun verifyEmail(token: String): Result<Unit> =
        authApi.verifyEmail(token).map { }.also {
            // If we are signed in, refresh the local user copy so the
            // "verify your email" banner disappears immediately.
            if (it.isSuccess) {
                currentUser?.let { u ->
                    val updated = u.copy(emailVerified = true)
                    currentUser = updated
                    _state.value = AuthState.Authenticated(updated)
                }
            }
        }

    suspend fun resendVerification(email: String): Result<Unit> =
        authApi.resendVerification(email).map { }

    suspend fun passwordResetRequest(email: String): Result<Unit> =
        authApi.passwordResetRequest(email).map { }

    suspend fun passwordResetConfirm(token: String, newPassword: String): Result<Unit> =
        authApi.passwordResetConfirm(token, newPassword).map { }

    /**
     * Log out. When [allDevices] is true, calls /v1/auth/logout-all (revokes
     * every refresh token family); otherwise revokes only this device's
     * refresh token. Local session is cleared regardless of the server result.
     */
    suspend fun logout(allDevices: Boolean): Result<Unit> {
        val rt = mutex.withLock { refreshToken }
        val outcome = if (allDevices) {
            authApi.logoutAll().map { }
        } else if (rt != null) {
            authApi.logout(rt).map { }
        } else {
            Result.success(Unit)
        }
        clearLocalSession()
        return outcome
    }

    /**
     * Replace the in-memory + persisted tokens after an endpoint that
     * returns a fresh [TokenPairDto] (e.g. PATCH /v1/me/password).
     */
    suspend fun replaceTokens(pair: TokenPairDto) {
        applyTokenPair(pair)
    }

    /** Wipe local session state and emit Unauthenticated. */
    suspend fun clearLocalSession() {
        mutex.withLock {
            accessToken = null
            refreshToken = null
            currentUser = null
        }
        tokenStorage.clear()
        _state.value = AuthState.Unauthenticated
    }

    /** Update the in-memory user (e.g. after PATCH /v1/me). */
    fun setUser(user: User) {
        currentUser = user
        if (_state.value is AuthState.Authenticated) {
            _state.value = AuthState.Authenticated(user)
        }
    }

    // --- BearerTokenProvider -------------------------------------------------

    override suspend fun load(): BearerTokens? = mutex.withLock {
        val a = accessToken ?: return null
        val r = refreshToken ?: return null
        BearerTokens(a, r)
    }

    override suspend fun refresh(): BearerTokens? {
        val rt = mutex.withLock { refreshToken } ?: return null
        return authApi.refresh(rt).fold(
            onSuccess = {
                applyTokenPair(it)
                BearerTokens(it.accessToken, it.refreshToken)
            },
            onFailure = { err ->
                if (err is ApiError.Unauthorized || err is ApiError.NotFound) {
                    clearLocalSession()
                }
                null
            },
        )
    }

    // --- internals -----------------------------------------------------------

    private suspend fun applyTokenPair(pair: TokenPairDto) {
        val user = pair.user.toUser()
        mutex.withLock {
            accessToken = pair.accessToken
            refreshToken = pair.refreshToken
            currentUser = user
        }
        tokenStorage.saveRefreshToken(pair.refreshToken)
        _state.value = AuthState.Authenticated(user)
    }
}
