package com.example.kmpstarter.data.auth

internal class FakeTokenStorage(initial: String? = null) : TokenStorage {
    var stored: String? = initial
        private set
    var saveCount = 0
        private set
    var clearCount = 0
        private set

    override suspend fun saveRefreshToken(token: String) {
        stored = token
        saveCount++
    }

    override suspend fun readRefreshToken(): String? = stored

    override suspend fun clear() {
        stored = null
        clearCount++
    }
}
