package com.example.kmpstarter.di

import com.example.kmpstarter.data.auth.SettingsTokenStorage
import com.example.kmpstarter.data.auth.TokenStorage
import com.russhwolf.settings.ExperimentalSettingsImplementation
import com.russhwolf.settings.KeychainSettings
import org.koin.dsl.module

private const val KEYCHAIN_SERVICE_NAME = "com.example.kmpstarter.auth"

@Suppress("unused")
@OptIn(ExperimentalSettingsImplementation::class)
val iosModule = module {
    single<TokenStorage> { SettingsTokenStorage(KeychainSettings(KEYCHAIN_SERVICE_NAME)) }
}
