package com.example.kmpstarter.ui.screens.register

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
import androidx.compose.material.icons.outlined.MailOutline
import androidx.compose.material.icons.outlined.PersonOutline
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
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.example.kmpstarter.domain.Validators
import com.example.kmpstarter.ui.components.AppScaffold
import com.example.kmpstarter.ui.components.AppTextField
import com.example.kmpstarter.ui.components.LinkText
import com.example.kmpstarter.ui.components.PasswordTextField
import com.example.kmpstarter.ui.components.PrimaryButton
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun RegisterScreen(
    onRegistered: (email: String) -> Unit,
    onBackToLogin: () -> Unit,
    viewModel: RegisterViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(state.snackbar) {
        val msg = state.snackbar ?: return@LaunchedEffect
        snackbarHostState.showSnackbar(msg)
        viewModel.consumeSnackbar()
    }

    LaunchedEffect(Unit) {
        viewModel.events.collect { event ->
            when (event) {
                is RegisterEvent.Registered -> onRegistered(event.email)
            }
        }
    }

    AppScaffold(snackbarHostState = snackbarHostState) { padding ->
        RegisterContent(
            state = state,
            modifier = Modifier.fillMaxSize().padding(padding),
            onEmailChange = viewModel::onEmailChange,
            onDisplayNameChange = viewModel::onDisplayNameChange,
            onPasswordChange = viewModel::onPasswordChange,
            onConfirmPasswordChange = viewModel::onConfirmPasswordChange,
            onSubmit = viewModel::onSubmit,
            onBackToLogin = onBackToLogin,
        )
    }
}

@Composable
fun RegisterContent(
    state: RegisterUiState,
    onEmailChange: (String) -> Unit,
    onDisplayNameChange: (String) -> Unit,
    onPasswordChange: (String) -> Unit,
    onConfirmPasswordChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onBackToLogin: () -> Unit,
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
            Spacer(Modifier.height(24.dp))

            Text(
                text = "Create your account",
                style = MaterialTheme.typography.headlineLarge,
            )
            Text(
                text = "It only takes a minute.",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
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

            AppTextField(
                value = state.displayName,
                onValueChange = onDisplayNameChange,
                label = "Display name (optional)",
                error = state.displayNameError,
                leadingIcon = Icons.Outlined.PersonOutline,
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Text,
                    imeAction = ImeAction.Next,
                    capitalization = KeyboardCapitalization.Words,
                ),
                enabled = !state.isLoading,
            )

            PasswordTextField(
                value = state.password,
                onValueChange = onPasswordChange,
                label = "Password",
                error = state.passwordError,
                supportingText = "At least ${Validators.PASSWORD_MIN_LENGTH} characters",
                imeAction = ImeAction.Next,
                enabled = !state.isLoading,
            )

            PasswordTextField(
                value = state.confirmPassword,
                onValueChange = onConfirmPasswordChange,
                label = "Confirm password",
                error = state.confirmPasswordError,
                imeAction = ImeAction.Done,
                keyboardActions = KeyboardActions(onDone = { onSubmit() }),
                enabled = !state.isLoading,
            )

            Spacer(Modifier.height(8.dp))

            PrimaryButton(
                text = "Create account",
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
                    text = "Already have an account?",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                LinkText(
                    text = "Sign in",
                    onClick = onBackToLogin,
                    enabled = !state.isLoading,
                )
            }
        }
    }
}
