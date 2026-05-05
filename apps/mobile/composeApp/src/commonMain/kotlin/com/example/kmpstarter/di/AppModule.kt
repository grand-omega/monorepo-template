package com.example.kmpstarter.di

import com.example.kmpstarter.apiBaseUrl
import com.example.kmpstarter.data.auth.AuthRepository
import com.example.kmpstarter.data.network.AuthApi
import com.example.kmpstarter.data.network.UserApi
import com.example.kmpstarter.data.network.buildHttpClient
import com.example.kmpstarter.data.user.UserRepository
import com.example.kmpstarter.ui.screens.login.LoginViewModel
import com.example.kmpstarter.ui.screens.forgotpassword.ForgotPasswordViewModel
import com.example.kmpstarter.ui.screens.home.HomeViewModel
import com.example.kmpstarter.ui.screens.profile.ProfileViewModel
import com.example.kmpstarter.ui.screens.register.RegisterViewModel
import com.example.kmpstarter.ui.screens.resetpassword.ResetPasswordViewModel
import com.example.kmpstarter.ui.screens.verifyemail.VerifyEmailViewModel
import io.ktor.client.HttpClient
import org.koin.core.module.dsl.viewModelOf
import org.koin.core.qualifier.named
import org.koin.dsl.module

const val PUBLIC_HTTP_CLIENT = "public_http_client"
const val AUTHED_HTTP_CLIENT = "authed_http_client"

/**
 * Common Koin module. Platform modules (androidModule / iosModule) must
 * supply a [com.example.kmpstarter.data.auth.TokenStorage] binding.
 */
val appModule = module {
    single<HttpClient>(named(PUBLIC_HTTP_CLIENT)) {
        buildHttpClient(baseUrl = apiBaseUrl)
    }
    single { AuthApi(get(named(PUBLIC_HTTP_CLIENT))) }
    single { AuthRepository(get(), get()) }

    single<HttpClient>(named(AUTHED_HTTP_CLIENT)) {
        buildHttpClient(
            baseUrl = apiBaseUrl,
            tokens = get<AuthRepository>(),
        )
    }
    single { UserApi(get(named(AUTHED_HTTP_CLIENT))) }
    single { UserRepository(get(), get()) }

    viewModelOf(::LoginViewModel)
    viewModelOf(::RegisterViewModel)
    viewModelOf(::VerifyEmailViewModel)
    viewModelOf(::ForgotPasswordViewModel)
    viewModelOf(::ResetPasswordViewModel)
    viewModelOf(::HomeViewModel)
    viewModelOf(::ProfileViewModel)
}
