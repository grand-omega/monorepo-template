package com.example.kmpstarter.di

import com.example.kmpstarter.data.auth.SettingsTokenStorage
import com.example.kmpstarter.data.auth.TokenStorage
import com.russhwolf.settings.NSUserDefaultsSettings
import org.koin.dsl.module
import platform.Foundation.NSUserDefaults

private const val DEFAULTS_SUITE_NAME = "kmpstarter_secure_prefs"

@Suppress("unused")
val iosModule = module {
    // TODO: swap NSUserDefaults for a Keychain-backed Settings before shipping iOS.
    single<TokenStorage> {
        val defaults = NSUserDefaults(suiteName = DEFAULTS_SUITE_NAME)
        SettingsTokenStorage(NSUserDefaultsSettings(defaults))
    }
}
