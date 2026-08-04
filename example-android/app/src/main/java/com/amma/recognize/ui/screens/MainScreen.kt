package com.amma.recognize.ui.screens

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.FileProvider
import java.io.File
import com.amma.recognize.data.*
import com.amma.recognize.ui.components.*
import com.amma.recognize.viewmodel.VisionViewModel
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainScreen(viewModel: VisionViewModel) {
    val isLoading by viewModel.isLoading.collectAsState()
    val loadingProgress by viewModel.loadingProgress.collectAsState()
    val loadingStage by viewModel.loadingStage.collectAsState()
    val messages by viewModel.messages.collectAsState()
    val currentResponse by viewModel.currentResponse.collectAsState()
    val isGenerating by viewModel.isGenerating.collectAsState()
    val isRecording by viewModel.isRecording.collectAsState()
    val transcript by viewModel.transcript.collectAsState()
    val pendingImages by viewModel.pendingImages.collectAsState()
    val pendingFile by viewModel.pendingFile.collectAsState()
    val sessions by viewModel.sessions.collectAsState()
    val currentSessionId by viewModel.currentSessionId.collectAsState()
    val searchText by viewModel.searchText.collectAsState()
    val filteredSessions by viewModel.filteredSessions.collectAsState(emptyList())

    var showSidebar by remember { mutableStateOf(false) }
    var showAttachmentMenu by remember { mutableStateOf(false) }
    var showSettings by remember { mutableStateOf(false) }
    var showRenameDialog by remember { mutableStateOf(false) }
    var renameText by remember { mutableStateOf("") }
    var inputText by remember { mutableStateOf("") }
    var showChatMenu by remember { mutableStateOf(false) }
    var selectedImageForViewing by remember { mutableStateOf<String?>(null) }

    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val focusManager = LocalFocusManager.current

    // Create camera URI
    val cameraUri = remember {
        val file = File(context.cacheDir, "images").apply { mkdirs() }
            .let { File(it, "camera_photo.jpg") }
        FileProvider.getUriForFile(
            context,
            "${context.packageName}.fileprovider",
            file
        )
    }

    val hasUserMessages = messages.any { it.role == ChatRole.USER }

    // Photo picker launcher
    val photoPickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetMultipleContents()
    ) { uris: List<Uri> ->
        if (uris.isNotEmpty()) {
            viewModel.setPendingImages(uris)
        }
    }

    // Camera launcher
    val cameraLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.TakePicture()
    ) { success ->
        if (success) {
            viewModel.addPendingImage(cameraUri)
        }
    }

    // File picker launcher
    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        uri?.let { viewModel.processFile(it) }
    }

    // Auto-scroll
    LaunchedEffect(messages.size, currentResponse) {
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(messages.size - 1)
        }
    }

    Box(modifier = Modifier.fillMaxSize()) {
        // Main content
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(Color(0xFFF7F5F0))
                .statusBarsPadding()
                .navigationBarsPadding()
        ) {
            // Header
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Color(0xFFF7F5F0)
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 8.dp)
                        .height(50.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Menu button
                    IconButton(onClick = {
                        focusManager.clearFocus()
                        showSidebar = true
                    }) {
                        Icon(
                            Icons.Default.Menu,
                            contentDescription = "Menu",
                            tint = Color(0xFF6B6B6B)
                        )
                    }

                    Spacer(modifier = Modifier.weight(1f))

                    // Title
                    Text(
                        text = "Amma",
                        fontWeight = FontWeight.Medium,
                        fontFamily = androidx.compose.ui.text.font.FontFamily.Serif,
                        fontSize = 19.sp,
                        color = Color(0xFF171A17)
                    )

                    Spacer(modifier = Modifier.weight(1f))

                    // Right buttons
                    if (hasUserMessages) {
                        IconButton(onClick = { viewModel.createNewChat() }) {
                            Icon(
                                Icons.Default.Edit,
                                contentDescription = "New Chat",
                                tint = Color(0xFF6B6B6B)
                            )
                        }

                        Box {
                            IconButton(onClick = { showChatMenu = true }) {
                                Icon(
                                    Icons.Default.MoreVert,
                                    contentDescription = "More",
                                    tint = Color(0xFF6B6B6B)
                                )
                            }

                            DropdownMenu(
                                expanded = showChatMenu,
                                onDismissRequest = { showChatMenu = false }
                            ) {
                                DropdownMenuItem(
                                    text = { Text("Rename") },
                                    leadingIcon = { Icon(Icons.Default.Edit, null) },
                                    onClick = {
                                        showChatMenu = false
                                        sessions.find { it.id == currentSessionId }?.let {
                                            renameText = it.title
                                            showRenameDialog = true
                                        }
                                    }
                                )
                                DropdownMenuItem(
                                    text = { Text("Delete", color = Color.Red) },
                                    leadingIcon = { Icon(Icons.Default.Delete, null, tint = Color.Red) },
                                    onClick = {
                                        showChatMenu = false
                                        sessions.find { it.id == currentSessionId }?.let {
                                            viewModel.deleteSession(it)
                                        }
                                    }
                                )
                            }
                        }
                    } else {
                        IconButton(onClick = { showSettings = true }) {
                            Icon(
                                Icons.Default.Settings,
                                contentDescription = "Settings",
                                tint = Color(0xFF6B6B6B)
                            )
                        }
                    }
                }
            }

            HorizontalDivider(color = Color(0xFFDFDAD1), thickness = 0.5.dp)

            // Chat area
            LazyColumn(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp),
                state = listState,
                contentPadding = PaddingValues(top = 8.dp, bottom = 10.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                // Welcome view if no messages
                if (messages.isEmpty() && pendingImages.isEmpty() && !isLoading) {
                    item {
                        WelcomeView(modifier = Modifier.padding(top = 40.dp))
                    }
                }

                // Messages
                items(messages, key = { it.id }) { message ->
                    MessageRow(
                        message = message,
                        onImageTap = { uri -> selectedImageForViewing = uri }
                    )
                }

                // Streaming response
                if (currentResponse.isNotBlank()) {
                    item(key = "streaming") {
                        MessageRow(
                            message = ChatMessageUI(
                                role = ChatRole.ASSISTANT,
                                text = currentResponse
                            )
                        )
                    }
                }

                // Analyzing indicator
                if (isGenerating && currentResponse.isBlank()) {
                    item(key = "thinking") {
                        AnalyzingView()
                    }
                }
            }

            // Input bar
            InputBar(
                text = inputText,
                onTextChange = { inputText = it },
                isRecording = isRecording,
                isGenerating = isGenerating,
                isDisabled = isLoading || isGenerating,
                pendingImageUris = pendingImages,
                pendingFile = pendingFile,
                onRemoveImage = { index -> viewModel.removePendingImage(index) },
                onRemoveFile = { viewModel.setPendingFile(null) },
                onSend = {
                    val text = inputText
                    if (viewModel.sendFollowUp(text)) {
                        inputText = ""
                        focusManager.clearFocus()
                        if (isRecording) {
                            viewModel.stopListening()
                        }
                    }
                },
                onAttachment = { showAttachmentMenu = true },
                onMicTap = {
                    if (isRecording) {
                        viewModel.stopListening()
                    } else {
                        // In a real app, we should check permission here
                        // For this example, we'll assume it's granted or handled in ViewModel
                        inputText = ""
                        viewModel.startListening()
                    }
                },
                onStopGeneration = { viewModel.stopGeneration() }
            )
        }

        // Update input text with transcript when recording
        LaunchedEffect(transcript) {
            if (isRecording) {
                inputText = transcript
            }
        }

        // Attachment menu overlay
        AnimatedVisibility(
            visible = showAttachmentMenu,
            enter = fadeIn(),
            exit = fadeOut()
        ) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.4f))
                    .clickable { showAttachmentMenu = false }
            ) {
                AttachmentMenuView(
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = 100.dp),
                    onCamera = {
                        showAttachmentMenu = false
                        cameraLauncher.launch(cameraUri)
                    },
                    onPhotos = {
                        showAttachmentMenu = false
                        photoPickerLauncher.launch("image/*")
                    },
                    onFiles = {
                        showAttachmentMenu = false
                        filePickerLauncher.launch("*/*")
                    }
                )
            }
        }

        // Sidebar overlay
        AnimatedVisibility(
            visible = showSidebar,
            enter = fadeIn(),
            exit = fadeOut()
        ) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.4f))
                    .clickable { showSidebar = false }
            )
        }

        AnimatedVisibility(
            visible = showSidebar,
            enter = slideInHorizontally { -it },
            exit = slideOutHorizontally { -it }
        ) {
            SidebarView(
                sessions = filteredSessions,
                currentSessionId = currentSessionId,
                searchText = searchText,
                onSearchChange = { viewModel.setSearchText(it) },
                onNewChat = {
                    viewModel.createNewChat()
                    showSidebar = false
                },
                onSessionSelect = { session ->
                    viewModel.loadSession(session)
                    showSidebar = false
                },
                onSessionRename = { session ->
                    renameText = session.title
                    showRenameDialog = true
                },
                onSessionDelete = { session ->
                    viewModel.deleteSession(session)
                },
                onClose = { showSidebar = false }
            )
        }

        // Loading overlay
        AnimatedVisibility(
            visible = isLoading,
            enter = fadeIn(),
            exit = fadeOut()
        ) {
            LoadingOverlay(
                progress = loadingProgress,
                stage = loadingStage,
                onRetry = viewModel::loadModel
            )
        }

        // Settings dialog
        if (showSettings) {
            SettingsDialog(
                userName = viewModel.getUserName(),
                userBirthday = viewModel.getUserBirthday(),
                onUserNameChange = { viewModel.setUserName(it) },
                onUserBirthdayChange = { viewModel.setUserBirthday(it) },
                onDismiss = { showSettings = false }
            )
        }

        // Rename dialog
        if (showRenameDialog) {
            AlertDialog(
                onDismissRequest = { showRenameDialog = false },
                title = { Text("Rename Chat") },
                text = {
                    OutlinedTextField(
                        value = renameText,
                        onValueChange = { renameText = it },
                        label = { Text("Chat title") },
                        singleLine = true
                    )
                },
                confirmButton = {
                    TextButton(onClick = {
                        sessions.find { it.id == currentSessionId }?.let {
                            viewModel.renameSession(it, renameText)
                        }
                        showRenameDialog = false
                    }) {
                        Text("Save")
                    }
                },
                dismissButton = {
                    TextButton(onClick = { showRenameDialog = false }) {
                        Text("Cancel")
                    }
                }
            )
        }

        // Image viewer
        selectedImageForViewing?.let { uri ->
            ImageViewerDialog(
                imageUri = uri,
                onDismiss = { selectedImageForViewing = null }
            )
        }
    }
}
