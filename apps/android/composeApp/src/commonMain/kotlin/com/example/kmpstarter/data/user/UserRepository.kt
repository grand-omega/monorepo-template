package com.example.kmpstarter.data.user

import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.auth.toUser
import com.example.kmpstarter.data.network.UserApi
import com.example.kmpstarter.domain.User

/**
 * Operations on the signed-in user (/v1/me). All calls implicitly require an
 * authenticated client; on 401 the underlying Ktor client invokes
 * [AuthRepository.refresh].
 */
class UserRepository(
    private val userApi: UserApi,
    private val authRepository: AuthRepository,
) {
    suspend fun fetchMe(): Result<User> =
        userApi.getMe()
            .map { it.toUser() }
            .onSuccess(authRepository::setUser)

    suspend fun updateDisplayName(displayName: String?): Result<User> =
        userApi.patchMe(displayName)
            .map { it.toUser() }
            .onSuccess(authRepository::setUser)

    suspend fun changePassword(currentPassword: String, newPassword: String): Result<Unit> =
        userApi.changePassword(currentPassword, newPassword)
            .onSuccess { authRepository.replaceTokens(it) }
            .map { }

    suspend fun deleteAccount(password: String): Result<Unit> =
        userApi.deleteMe(password)
            .onSuccess { authRepository.clearLocalSession() }
            .map { }
}
