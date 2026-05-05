package com.example.kmpstarter.ui.screens.profile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Person
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.example.kmpstarter.domain.User
import com.example.kmpstarter.ui.components.AppScaffold
import com.example.kmpstarter.ui.components.AppTextField
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

    LaunchedEffect(user.id) { viewModel.applyUser(user) }
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
            Text(user.email, color = MaterialTheme.colorScheme.onSurfaceVariant)

            AppTextField(
                value = state.displayName,
                onValueChange = viewModel::onDisplayNameChange,
                label = "Display name",
                error = state.displayNameError,
                leadingIcon = Icons.Outlined.Person,
                enabled = !state.savingProfile,
            )
            PrimaryButton("Save profile", onClick = viewModel::saveProfile, isLoading = state.savingProfile)

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
