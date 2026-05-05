package com.example.kmpstarter.data.network

import io.ktor.client.engine.HttpClientEngine

/** Per-platform HTTP engine factory (OkHttp on Android, Darwin on iOS). */
expect fun httpClientEngine(): HttpClientEngine
