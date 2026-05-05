package com.example.kmpstarter.ui.screens.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AccountCircle
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.unit.dp
import com.example.kmpstarter.domain.User
import com.example.kmpstarter.ui.components.AppScaffold
import com.example.kmpstarter.ui.components.LinkText
import com.example.kmpstarter.ui.components.PasswordTextField
import com.example.kmpstarter.ui.components.PrimaryButton
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun ProfileScreen(
    user: User,
    onBack: () -> Unit,
    viewModel: ProfileViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(state.snackbar) {
        val msg = state.snackbar ?: return@LaunchedEffect
        snackbarHostState.showSnackbar(msg)
        viewModel.consumeSnackbar()
    }

    AppScaffold(snackbarHostState = snackbarHostState) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text("Profile", style = MaterialTheme.typography.headlineLarge)
                LinkText("Back", onClick = onBack)
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier
                        .size(72.dp)
                        .background(MaterialTheme.colorScheme.surfaceVariant, CircleShape),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Outlined.AccountCircle,
                        contentDescription = "Profile avatar",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(48.dp),
                    )
                }
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        text = user.displayName?.takeIf { it.isNotBlank() } ?: "No display name",
                        style = MaterialTheme.typography.titleLarge,
                    )
                    Text(
                        text = user.email,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            HorizontalDivider()
            Text("Change password", style = MaterialTheme.typography.titleLarge)
            PasswordTextField(
                value = state.currentPassword,
                onValueChange = viewModel::onCurrentPasswordChange,
                label = "Current password",
                error = state.currentPasswordError,
                imeAction = ImeAction.Next,
                enabled = !state.changingPassword,
            )
            PasswordTextField(
                value = state.newPassword,
                onValueChange = viewModel::onNewPasswordChange,
                label = "New password",
                error = state.newPasswordError,
                imeAction = ImeAction.Done,
                enabled = !state.changingPassword,
            )
            PrimaryButton("Change password", onClick = viewModel::changePassword, isLoading = state.changingPassword)

            HorizontalDivider()
            Text("Session", style = MaterialTheme.typography.titleLarge)
            OutlinedButton(onClick = { viewModel.logout(false) }, modifier = Modifier.fillMaxWidth(), enabled = !state.loggingOut) {
                Text("Sign out")
            }
            Button(onClick = { viewModel.logout(true) }, modifier = Modifier.fillMaxWidth(), enabled = !state.loggingOut) {
                Text("Sign out everywhere")
            }

            HorizontalDivider()
            Text("Delete account", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.error)
            PasswordTextField(
                value = state.deletePassword,
                onValueChange = viewModel::onDeletePasswordChange,
                label = "Password",
                error = state.deletePasswordError,
                imeAction = ImeAction.Done,
                enabled = !state.deleting,
            )
            PrimaryButton("Delete account", onClick = viewModel::deleteAccount, isLoading = state.deleting)
            Spacer(Modifier.height(8.dp))
        }
    }
}
