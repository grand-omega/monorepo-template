package com.example.kmpstarter.data.network

import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.engine.darwin.Darwin

actual fun httpClientEngine(): HttpClientEngine = Darwin.create()
