@file:Suppress("DEPRECATION") // androidx.security.crypto is the standard EncryptedSharedPreferences API.

package com.example.kmpstarter.di

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.example.kmpstarter.data.auth.SettingsTokenStorage
import com.example.kmpstarter.data.auth.TokenStorage
import com.russhwolf.settings.Settings
import com.russhwolf.settings.SharedPreferencesSettings
import org.koin.android.ext.koin.androidContext
import org.koin.dsl.module

private const val SECURE_PREFS_NAME = "kmpstarter_secure_prefs"

val androidModule = module {
    single<Settings> { createEncryptedSettings(androidContext()) }
    single<TokenStorage> { SettingsTokenStorage(get()) }
}

private fun createEncryptedSettings(context: Context): Settings {
    val masterKey = MasterKey.Builder(context, MasterKey.DEFAULT_MASTER_KEY_ALIAS)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()
    val prefs = EncryptedSharedPreferences.create(
        context,
        SECURE_PREFS_NAME,
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )
    return SharedPreferencesSettings(prefs)
}
