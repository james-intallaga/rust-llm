package com.amma.recognize.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.amma.recognize.data.ChatSession

@Composable
fun SidebarView(
    sessions: List<ChatSession>,
    currentSessionId: String?,
    searchText: String,
    onSearchChange: (String) -> Unit,
    onNewChat: () -> Unit,
    onSessionSelect: (ChatSession) -> Unit,
    onSessionRename: (ChatSession) -> Unit,
    onSessionDelete: (ChatSession) -> Unit,
    onClose: () -> Unit
) {
    Surface(
        modifier = Modifier
            .fillMaxHeight()
            .width(300.dp),
        color = Color.White
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            // Header
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                Spacer(modifier = Modifier.height(24.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Search field
                    Surface(
                        modifier = Modifier.weight(1f),
                        shape = RoundedCornerShape(10.dp),
                        color = Color(0xFFF2F2F7)
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Icon(
                                Icons.Default.Search,
                                contentDescription = null,
                                modifier = Modifier.size(14.dp),
                                tint = Color(0xFF6B6B6B)
                            )

                            OutlinedTextField(
                                value = searchText,
                                onValueChange = onSearchChange,
                                modifier = Modifier.fillMaxWidth(),
                                placeholder = { Text("Search chats", fontSize = 16.sp) },
                                colors = OutlinedTextFieldDefaults.colors(
                                    unfocusedBorderColor = Color.Transparent,
                                    focusedBorderColor = Color.Transparent,
                                    unfocusedContainerColor = Color.Transparent,
                                    focusedContainerColor = Color.Transparent
                                ),
                                singleLine = true,
                                textStyle = LocalTextStyle.current.copy(fontSize = 16.sp)
                            )
                        }
                    }

                    // New chat button
                    IconButton(
                        onClick = onNewChat,
                        modifier = Modifier
                            .size(36.dp)
                            .background(Color(0xFFF2F2F7), CircleShape)
                    ) {
                        Icon(
                            Icons.Default.Edit,
                            contentDescription = "New Chat",
                            modifier = Modifier.size(18.dp)
                        )
                    }
                }
            }

            HorizontalDivider(color = Color(0xFFD1D1D6))

            // Session list
            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = PaddingValues(vertical = 10.dp)
            ) {
                item {
                    Text(
                        text = "Chat History",
                        fontSize = 13.sp,
                        fontWeight = FontWeight.Medium,
                        color = Color(0xFF6B6B6B),
                        modifier = Modifier.padding(horizontal = 20.dp, vertical = 10.dp)
                    )
                }

                if (sessions.isEmpty()) {
                    item {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(top = 40.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Icon(
                                if (searchText.isBlank()) Icons.Default.Forum else Icons.Default.Search,
                                contentDescription = null,
                                modifier = Modifier.size(32.dp),
                                tint = Color(0xFF6B6B6B).copy(alpha = 0.5f)
                            )
                            Text(
                                text = if (searchText.isBlank()) "No chats yet" else "No results found",
                                fontSize = 14.sp,
                                color = Color(0xFF6B6B6B)
                            )
                        }
                    }
                } else {
                    items(sessions, key = { it.id }) { session ->
                        SessionRow(
                            session = session,
                            isSelected = session.id == currentSessionId,
                            onTap = { onSessionSelect(session) },
                            onRename = { onSessionRename(session) },
                            onDelete = { onSessionDelete(session) }
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SessionRow(
    session: ChatSession,
    isSelected: Boolean,
    onTap: () -> Unit,
    onRename: () -> Unit,
    onDelete: () -> Unit
) {
    var showMenu by remember { mutableStateOf(false) }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onTap() },
        color = if (isSelected) Color(0xFFF2F2F7) else Color.Transparent
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp, vertical = 14.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = session.title,
                fontSize = 15.sp,
                color = if (isSelected) Color.Black else Color.Black.copy(alpha = 0.7f),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f)
            )

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                if (isSelected) {
                    Box(
                        modifier = Modifier
                            .size(6.dp)
                            .background(Color(0xFF007AFF), CircleShape)
                    )
                }

                Box {
                    IconButton(
                        onClick = { showMenu = true },
                        modifier = Modifier.size(24.dp)
                    ) {
                        Icon(
                            Icons.Default.MoreVert,
                            contentDescription = "More",
                            modifier = Modifier.size(16.dp),
                            tint = Color(0xFF6B6B6B)
                        )
                    }

                    DropdownMenu(
                        expanded = showMenu,
                        onDismissRequest = { showMenu = false }
                    ) {
                        DropdownMenuItem(
                            text = { Text("Rename") },
                            leadingIcon = { Icon(Icons.Default.Edit, null) },
                            onClick = {
                                showMenu = false
                                onRename()
                            }
                        )
                        DropdownMenuItem(
                            text = { Text("Delete", color = Color.Red) },
                            leadingIcon = { Icon(Icons.Default.Delete, null, tint = Color.Red) },
                            onClick = {
                                showMenu = false
                                onDelete()
                            }
                        )
                    }
                }
            }
        }
    }
}
