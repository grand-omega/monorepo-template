package com.example.kmpstarter.ui.screens.login

import androidx.compose.foundation.Canvas
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.MailOutline
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.example.kmpstarter.ui.components.AppScaffold
import com.example.kmpstarter.ui.components.AppTextField
import com.example.kmpstarter.ui.components.LinkText
import com.example.kmpstarter.ui.components.PasswordTextField
import com.example.kmpstarter.ui.components.PrimaryButton
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun LoginScreen(
    onNavigateToRegister: () -> Unit,
    onNavigateToForgotPassword: () -> Unit,
    viewModel: LoginViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(Unit) {
        viewModel.refreshServerStatus()
    }

    LaunchedEffect(state.snackbar) {
        val msg = state.snackbar ?: return@LaunchedEffect
        snackbarHostState.showSnackbar(msg)
        viewModel.consumeSnackbar()
    }

    AppScaffold(snackbarHostState = snackbarHostState) { padding ->
        LoginContent(
            state = state,
            modifier = Modifier.fillMaxSize().padding(padding),
            onEmailChange = viewModel::onEmailChange,
            onPasswordChange = viewModel::onPasswordChange,
            onSubmit = viewModel::onSubmit,
            onNavigateToRegister = onNavigateToRegister,
            onNavigateToForgotPassword = onNavigateToForgotPassword,
        )
    }
}

/** Stateless render of the login form — used in previews and tests. */
@Composable
fun LoginContent(
    state: LoginUiState,
    onEmailChange: (String) -> Unit,
    onPasswordChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onNavigateToRegister: () -> Unit,
    onNavigateToForgotPassword: () -> Unit,
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
                text = "Welcome back",
                style = MaterialTheme.typography.headlineLarge,
            )
            Text(
                text = "Sign in to continue.",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            ServerStatusIndicator(
                status = state.serverStatus,
            )

            Spacer(Modifier.height(8.dp))

            AppTextField(
                value = state.email,
                onValueChange = onEmailChange,
                label = "Email",
                error = state.emailError,
                leadingIcon = Icons.Outlined.MailOutline,
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Email,
                    imeAction = ImeAction.Next,
                    capitalization = KeyboardCapitalization.None,
                    autoCorrectEnabled = false,
                ),
                enabled = !state.isLoading,
            )

            PasswordTextField(
                value = state.password,
                onValueChange = onPasswordChange,
                label = "Password",
                error = state.passwordError,
                imeAction = ImeAction.Done,
                keyboardActions = KeyboardActions(onDone = { onSubmit() }),
                enabled = !state.isLoading,
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                LinkText(
                    text = "Forgot password?",
                    onClick = onNavigateToForgotPassword,
                    enabled = !state.isLoading,
                )
            }

            Spacer(Modifier.height(8.dp))

            PrimaryButton(
                text = "Sign in",
                onClick = onSubmit,
                isLoading = state.isLoading,
            )

            Spacer(Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "New here?",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
                LinkText(
                    text = "Create account",
                    onClick = onNavigateToRegister,
                    enabled = !state.isLoading,
                )
            }
        }
    }
}

@Composable
private fun ServerStatusIndicator(
    status: ServerStatus,
) {
    val color = when (status) {
        ServerStatus.Online -> Color(0xFF2E7D32)
        ServerStatus.Offline -> MaterialTheme.colorScheme.error
        ServerStatus.Checking -> MaterialTheme.colorScheme.tertiary
        ServerStatus.Unknown -> MaterialTheme.colorScheme.outline
    }
    val label = when (status) {
        ServerStatus.Online -> "Server online"
        ServerStatus.Offline -> "Server offline"
        ServerStatus.Checking -> "Checking server"
        ServerStatus.Unknown -> "Server status unknown"
    }

    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Canvas(Modifier.size(10.dp)) {
            drawCircle(color)
        }
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
