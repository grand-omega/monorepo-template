package com.example.kmpstarter.ui.screens.verifyemail

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.MarkEmailRead
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.example.kmpstarter.ui.components.AppScaffold
import com.example.kmpstarter.ui.components.AppTextField
import com.example.kmpstarter.ui.components.LinkText
import com.example.kmpstarter.ui.components.PrimaryButton
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun VerifyEmailScreen(
    email: String?,
    prefilledToken: String?,
    onVerified: () -> Unit,
    onSkipToLogin: () -> Unit,
    viewModel: VerifyEmailViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(email, prefilledToken) {
        viewModel.applyArgs(email, prefilledToken)
    }

    LaunchedEffect(state.snackbar) {
        val msg = state.snackbar ?: return@LaunchedEffect
        snackbarHostState.showSnackbar(msg)
        viewModel.consumeSnackbar()
    }

    LaunchedEffect(Unit) {
        viewModel.events.collect { event ->
            when (event) {
                VerifyEmailEvent.Verified -> onVerified()
            }
        }
    }

    AppScaffold(snackbarHostState = snackbarHostState) { padding ->
        VerifyEmailContent(
            state = state,
            modifier = Modifier.fillMaxSize().padding(padding),
            onTokenChange = viewModel::onTokenChange,
            onSubmit = viewModel::onSubmit,
            onResend = viewModel::onResend,
            onSkipToLogin = onSkipToLogin,
        )
    }
}

@Composable
fun VerifyEmailContent(
    state: VerifyEmailUiState,
    onTokenChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onResend: () -> Unit,
    onSkipToLogin: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(modifier = modifier, contentAlignment = Alignment.TopCenter) {
        Column(
            modifier = Modifier
                .widthIn(max = 480.dp)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(horizontal = 24.dp, vertical = 32.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Spacer(Modifier.height(40.dp))

            Text(
                text = "Verify your email",
                style = MaterialTheme.typography.headlineLarge,
            )

            Text(
                text = if (state.email.isNotBlank()) {
                    "We sent a verification link to ${state.email}. Open it on this device, " +
                        "or paste the token below."
                } else {
                    "We sent a verification link to your email. Open it on this device, " +
                        "or paste the token below."
                },
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Spacer(Modifier.height(8.dp))

            AppTextField(
                value = state.token,
                onValueChange = onTokenChange,
                label = "Verification token",
                error = state.tokenError,
                leadingIcon = Icons.Outlined.MarkEmailRead,
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Ascii,
                    imeAction = ImeAction.Done,
                    autoCorrectEnabled = false,
                ),
                keyboardActions = KeyboardActions(onDone = { onSubmit() }),
                enabled = !state.isVerifying,
                singleLine = false,
            )

            Spacer(Modifier.height(8.dp))

            PrimaryButton(
                text = "Verify",
                onClick = onSubmit,
                isLoading = state.isVerifying,
            )

            Spacer(Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TextButton(
                    onClick = onResend,
                    enabled = !state.isResending && state.resendCooldownSeconds == 0 && !state.isVerifying,
                ) {
                    val label = when {
                        state.isResending -> "Sending…"
                        state.resendCooldownSeconds > 0 -> "Resend in ${state.resendCooldownSeconds}s"
                        else -> "Resend email"
                    }
                    Text(label)
                }

                LinkText(
                    text = "Back to sign in",
                    onClick = onSkipToLogin,
                    enabled = !state.isVerifying,
                )
            }
        }
    }
}
