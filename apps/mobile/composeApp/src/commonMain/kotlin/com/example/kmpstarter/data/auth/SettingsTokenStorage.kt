package com.example.kmpstarter.data.auth

import com.russhwolf.settings.Settings
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** TokenStorage backed by a com.russhwolf:multiplatform-settings [Settings]. */
class SettingsTokenStorage(private val settings: Settings) : TokenStorage {

    override suspend fun saveRefreshToken(token: String) = withContext(Dispatchers.Default) {
        settings.putString(KEY_REFRESH_TOKEN, token)
    }

    override suspend fun readRefreshToken(): String? = withContext(Dispatchers.Default) {
        if (settings.hasKey(KEY_REFRESH_TOKEN)) {
            settings.getStringOrNull(KEY_REFRESH_TOKEN)
        } else {
            null
        }
    }

    override suspend fun clear() = withContext(Dispatchers.Default) {
        settings.remove(KEY_REFRESH_TOKEN)
    }

    private companion object {
        const val KEY_REFRESH_TOKEN = "refresh_token"
    }
}
