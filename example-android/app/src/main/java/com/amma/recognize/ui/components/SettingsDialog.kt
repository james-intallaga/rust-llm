package com.amma.recognize.ui.components

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.OpenInNew
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import java.text.SimpleDateFormat
import java.util.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsDialog(
    userName: String,
    userBirthday: String = "",
    onUserNameChange: (String) -> Unit,
    onUserBirthdayChange: (String) -> Unit = {},
    onDismiss: () -> Unit
) {
    var name by remember { mutableStateOf(userName) }
    var showDatePicker by remember { mutableStateOf(false) }
    var selectedDate by remember {
        mutableStateOf(
            if (userBirthday.isNotBlank()) {
                try {
                    SimpleDateFormat("MM/dd/yyyy", Locale.US).parse(userBirthday)?.time ?: System.currentTimeMillis()
                } catch (e: Exception) {
                    System.currentTimeMillis()
                }
            } else {
                // Default to 18 years ago
                Calendar.getInstance().apply { add(Calendar.YEAR, -18) }.timeInMillis
            }
        )
    }
    val context = LocalContext.current

    val dateFormatter = remember { SimpleDateFormat("MM/dd/yyyy", Locale.US) }
    val displayDate = dateFormatter.format(Date(selectedDate))

    // URLs
    val privacyUrl = "https://privacy.amma.live"
    val termsUrl = "https://term.amma.live"
    val playStoreUrl = "https://play.google.com/store/apps/details?id=com.amma.recognize"

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false)
    ) {
        Surface(
            modifier = Modifier
                .fillMaxSize()
                .padding(top = 48.dp),
            color = Color(0xFFF2F2F7)
        ) {
            Column(modifier = Modifier.fillMaxSize()) {
                // Header
                CenterAlignedTopAppBar(
                    title = { Text("Settings", fontWeight = FontWeight.SemiBold) },
                    navigationIcon = { },
                    actions = {
                        TextButton(onClick = {
                            onUserNameChange(name)
                            onUserBirthdayChange(dateFormatter.format(Date(selectedDate)))
                            onDismiss()
                        }) {
                            Text("Done", fontWeight = FontWeight.SemiBold)
                        }
                    },
                    colors = TopAppBarDefaults.centerAlignedTopAppBarColors(
                        containerColor = Color(0xFFF2F2F7)
                    )
                )

                LazyColumn(
                    modifier = Modifier.fillMaxSize()
                ) {
                    // Profile Section
                    item {
                        SettingsSection(title = "Profile") {
                            SettingsTextField(
                                label = "Name",
                                value = name,
                                onValueChange = { name = it }
                            )
                            HorizontalDivider(modifier = Modifier.padding(start = 16.dp))
                            SettingsClickableRow(
                                label = "Birthday",
                                value = displayDate,
                                onClick = { showDatePicker = true }
                            )
                        }
                    }

                    // Share Section
                    item {
                        SettingsSection(title = "Share") {
                            SettingsActionRow(
                                icon = Icons.Default.People,
                                iconTint = Color(0xFF007AFF),
                                title = "Recommend to Friends",
                                subtitle = "Share Amma with your friends",
                                trailingIcon = Icons.Default.Share,
                                onClick = { shareWithFriends(context, playStoreUrl) }
                            )
                        }
                    }

                    // About Section
                    item {
                        SettingsSection(title = "About Amma") {
                            SettingsInfoRow(
                                icon = Icons.Default.AttachMoney,
                                iconTint = Color(0xFF34C759),
                                title = "$1 USD Forever",
                                subtitle = "One-time purchase, lifetime access"
                            )
                            HorizontalDivider(modifier = Modifier.padding(start = 56.dp))
                            SettingsInfoRow(
                                icon = Icons.Default.PhoneAndroid,
                                iconTint = Color(0xFF007AFF),
                                title = "All Data on Your Phone",
                                subtitle = "100% privacy protected"
                            )
                            HorizontalDivider(modifier = Modifier.padding(start = 56.dp))
                            SettingsInfoRow(
                                icon = Icons.Default.AutoAwesome,
                                iconTint = Color(0xFF34C759),
                                title = "Personal Assistant",
                                subtitle = "Writing, planning, images and documents"
                            )
                        }
                    }

                    // Support Section
                    item {
                        SettingsSection(title = "Support") {
                            SettingsActionRow(
                                icon = Icons.Default.Star,
                                iconTint = Color(0xFFFFCC00),
                                title = "Rate Amma ⭐⭐⭐⭐⭐",
                                subtitle = "Love Amma? Give us 5 stars!",
                                trailingIcon = Icons.AutoMirrored.Filled.OpenInNew,
                                onClick = { openUrl(context, playStoreUrl) }
                            )
                        }
                    }

                    // Legal Section
                    item {
                        SettingsSection(title = "Legal") {
                            SettingsLinkRow(
                                icon = Icons.Default.PrivacyTip,
                                iconTint = Color(0xFF9C27B0),
                                title = "Privacy Policy",
                                onClick = { openUrl(context, privacyUrl) }
                            )
                            HorizontalDivider(modifier = Modifier.padding(start = 56.dp))
                            SettingsLinkRow(
                                icon = Icons.Default.Description,
                                iconTint = Color(0xFFFF9800),
                                title = "Terms of Use",
                                onClick = { openUrl(context, termsUrl) }
                            )
                        }
                    }

                    // Version Section
                    item {
                        SettingsSection(title = null) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 16.dp, vertical = 12.dp),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text(
                                    text = "Version",
                                    color = Color(0xFF6B6B6B)
                                )
                                Text(
                                    text = "1.0.0 (Rust)",
                                    color = Color(0xFF6B6B6B)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(32.dp))
                    }
                }
            }
        }
    }

    // Date Picker Dialog
    if (showDatePicker) {
        val datePickerState = rememberDatePickerState(
            initialSelectedDateMillis = selectedDate
        )

        DatePickerDialog(
            onDismissRequest = { showDatePicker = false },
            confirmButton = {
                TextButton(onClick = {
                    datePickerState.selectedDateMillis?.let { selectedDate = it }
                    showDatePicker = false
                }) {
                    Text("OK")
                }
            },
            dismissButton = {
                TextButton(onClick = { showDatePicker = false }) {
                    Text("Cancel")
                }
            }
        ) {
            DatePicker(state = datePickerState)
        }
    }
}

private fun shareWithFriends(context: Context, playStoreUrl: String) {
    val message = """
        Meet Amma, your private personal assistant.

        ✓ $1 USD forever
        ✓ All data stays on your phone
        ✓ Writing, planning, images and documents

        Download here: $playStoreUrl
    """.trimIndent()

    val intent = Intent(Intent.ACTION_SEND).apply {
        type = "text/plain"
        putExtra(Intent.EXTRA_TEXT, message)
    }
    context.startActivity(Intent.createChooser(intent, "Share Amma"))
}

private fun openUrl(context: Context, url: String) {
    val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
    context.startActivity(intent)
}

@Composable
private fun SettingsSection(
    title: String?,
    content: @Composable ColumnScope.() -> Unit
) {
    Column(modifier = Modifier.padding(top = 16.dp)) {
        if (title != null) {
            Text(
                text = title,
                fontSize = 13.sp,
                color = Color(0xFF6B6B6B),
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            )
        }

        Surface(
            color = Color.White,
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(content = content)
        }
    }
}

@Composable
private fun SettingsTextField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            color = Color(0xFF6B6B6B)
        )
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            modifier = Modifier.width(200.dp),
            singleLine = true,
            placeholder = { Text("Enter your name") },
            colors = OutlinedTextFieldDefaults.colors(
                unfocusedBorderColor = Color.Transparent,
                focusedBorderColor = Color.Transparent
            )
        )
    }
}

@Composable
private fun SettingsClickableRow(
    label: String,
    value: String,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            color = Color(0xFF6B6B6B)
        )
        Text(
            text = value,
            color = Color(0xFF007AFF)
        )
    }
}

@Composable
private fun SettingsInfoRow(
    icon: ImageVector,
    iconTint: Color,
    title: String,
    subtitle: String
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = iconTint,
            modifier = Modifier.size(28.dp)
        )
        Column {
            Text(
                text = title,
                fontWeight = FontWeight.Medium
            )
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = Color(0xFF6B6B6B)
            )
        }
    }
}

@Composable
private fun SettingsActionRow(
    icon: ImageVector,
    iconTint: Color,
    title: String,
    subtitle: String,
    trailingIcon: ImageVector,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = iconTint,
            modifier = Modifier.size(28.dp)
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                fontWeight = FontWeight.Medium
            )
            Text(
                text = subtitle,
                fontSize = 12.sp,
                color = Color(0xFF6B6B6B)
            )
        }
        Icon(
            trailingIcon,
            contentDescription = null,
            tint = Color(0xFF6B6B6B),
            modifier = Modifier.size(16.dp)
        )
    }
}

@Composable
private fun SettingsLinkRow(
    icon: ImageVector,
    iconTint: Color,
    title: String,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = iconTint,
            modifier = Modifier.size(28.dp)
        )
        Spacer(modifier = Modifier.width(12.dp))
        Text(
            text = title,
            modifier = Modifier.weight(1f)
        )
        Icon(
            Icons.AutoMirrored.Filled.OpenInNew,
            contentDescription = null,
            tint = Color(0xFF6B6B6B),
            modifier = Modifier.size(16.dp)
        )
    }
}
