package com.example.kmpstarter

import android.app.Application
import com.example.kmpstarter.di.androidModule
import com.example.kmpstarter.di.appModule
import org.koin.android.ext.koin.androidContext
import org.koin.android.ext.koin.androidLogger
import org.koin.core.context.startKoin
import org.koin.core.logger.Level

class KmpStarterApp : Application() {
    override fun onCreate() {
        super.onCreate()
        startKoin {
            androidLogger(Level.INFO)
            androidContext(this@KmpStarterApp)
            modules(appModule, androidModule)
        }
    }
}
