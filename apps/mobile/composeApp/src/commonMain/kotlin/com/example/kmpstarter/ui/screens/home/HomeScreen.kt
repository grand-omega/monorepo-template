package com.example.kmpstarter.ui.screens.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AccountCircle
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.kmpstarter.domain.User
import com.example.kmpstarter.ui.components.AppScaffold
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun HomeScreen(
    user: User,
    onOpenProfile: () -> Unit,
    onVerifyEmail: () -> Unit,
    viewModel: HomeViewModel = koinViewModel(),
) {
    val state by viewModel.uiState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(Unit) { viewModel.refresh() }
    LaunchedEffect(state.snackbar) {
        val msg = state.snackbar ?: return@LaunchedEffect
        snackbarHostState.showSnackbar(msg)
        viewModel.consumeSnackbar()
    }

    AppScaffold(snackbarHostState = snackbarHostState) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column {
                    Text("Home", style = MaterialTheme.typography.headlineLarge)
                    Text(user.displayName ?: user.email, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                IconButton(onClick = viewModel::refresh, enabled = !state.isRefreshing) {
                    Icon(Icons.Outlined.Refresh, contentDescription = "Refresh")
                }
            }

            if (!user.emailVerified) {
                Surface(
                    color = MaterialTheme.colorScheme.secondaryContainer,
                    shape = MaterialTheme.shapes.medium,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Text("Email verification needed", style = MaterialTheme.typography.titleMedium)
                        Text("Verify your email to unlock the full account lifecycle.")
                        OutlinedButton(onClick = onVerifyEmail) { Text("Verify email") }
                    }
                }
            }

            Surface(
                tonalElevation = 1.dp,
                shape = MaterialTheme.shapes.medium,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Icon(Icons.Outlined.AccountCircle, contentDescription = null)
                        Text("Account", style = MaterialTheme.typography.titleMedium)
                    }
                    Text("Email: ${user.email}")
                    AssistChip(
                        onClick = {},
                        label = { Text(if (user.emailVerified) "Verified" else "Unverified") },
                    )
                }
            }

            Spacer(Modifier.height(8.dp))
            Button(onClick = onOpenProfile, modifier = Modifier.fillMaxWidth()) { Text("Profile settings") }
            OutlinedButton(onClick = viewModel::logout, modifier = Modifier.fillMaxWidth()) { Text("Sign out") }
        }
    }
}
