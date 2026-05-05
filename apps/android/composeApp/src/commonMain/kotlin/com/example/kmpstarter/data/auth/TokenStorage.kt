package com.example.kmpstarter.data.auth

/** Persistent (encrypted on device) storage for the long-lived refresh token. */
interface TokenStorage {
    suspend fun saveRefreshToken(token: String)
    suspend fun readRefreshToken(): String?
    suspend fun clear()
}
