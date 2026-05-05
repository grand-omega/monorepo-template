package com.example.kmpstarter.ui.screens.resetpassword

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
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
import com.example.kmpstarter.ui.components.PasswordTextField
import com.example.kmpstarter.ui.components.PrimaryButton
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun ResetPasswordScreen(
    prefilledToken: String?,
    onResetComplete: () -> Unit,
    onBackToLogin: () -> Unit,
    viewModel: ResetPasswordViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(prefilledToken) {
        viewModel.applyArgs(prefilledToken)
    }
    LaunchedEffect(state.snackbar) {
        val msg = state.snackbar ?: return@LaunchedEffect
        snackbarHostState.showSnackbar(msg)
        viewModel.consumeSnackbar()
    }
    LaunchedEffect(Unit) {
        viewModel.events.collect { event ->
            when (event) {
                ResetPasswordEvent.ResetComplete -> onResetComplete()
            }
        }
    }

    AppScaffold(snackbarHostState = snackbarHostState) { padding ->
        Box(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.TopCenter) {
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
                Text("Set a new password", style = MaterialTheme.typography.headlineLarge)
                Text(
                    "Paste the reset token from your email, then choose a new password.",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                AppTextField(
                    value = state.token,
                    onValueChange = viewModel::onTokenChange,
                    label = "Reset token",
                    error = state.tokenError,
                    leadingIcon = Icons.Outlined.Key,
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Ascii,
                        imeAction = ImeAction.Next,
                        autoCorrectEnabled = false,
                    ),
                    enabled = !state.isLoading,
                    singleLine = false,
                )
                PasswordTextField(
                    value = state.newPassword,
                    onValueChange = viewModel::onNewPasswordChange,
                    label = "New password",
                    error = state.newPasswordError,
                    imeAction = ImeAction.Next,
                    enabled = !state.isLoading,
                )
                PasswordTextField(
                    value = state.confirmPassword,
                    onValueChange = viewModel::onConfirmPasswordChange,
                    label = "Confirm password",
                    error = state.confirmPasswordError,
                    imeAction = ImeAction.Done,
                    keyboardActions = KeyboardActions(onDone = { viewModel.onSubmit() }),
                    enabled = !state.isLoading,
                )
                PrimaryButton("Reset password", onClick = viewModel::onSubmit, isLoading = state.isLoading)
                LinkText("Back to sign in", onClick = onBackToLogin, enabled = !state.isLoading)
            }
        }
    }
}
