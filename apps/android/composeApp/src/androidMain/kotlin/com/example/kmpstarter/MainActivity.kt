package com.example.kmpstarter

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import com.example.kmpstarter.ui.navigation.DeepLinkAction

class MainActivity : ComponentActivity() {

    private val deepLink: MutableState<DeepLinkAction?> = mutableStateOf(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        consumeIntent(intent)
        setContent { App(deepLink = deepLink) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        consumeIntent(intent)
    }

    private fun consumeIntent(intent: Intent?) {
        val data: Uri = intent?.data ?: return
        if (data.scheme != APP_SCHEME) return
        val token = data.getQueryParameter("token") ?: return
        deepLink.value = when (data.host) {
            HOST_VERIFY -> DeepLinkAction.VerifyEmail(token)
            HOST_RESET -> DeepLinkAction.ResetPassword(token)
            else -> return
        }
    }

    private companion object {
        const val APP_SCHEME = "kmpstarter"
        const val HOST_VERIFY = "verify"
        const val HOST_RESET = "reset"
    }
}
