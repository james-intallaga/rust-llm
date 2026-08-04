package com.amma.recognize.ui.components

import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import com.amma.recognize.data.PendingFile

@Composable
fun InputBar(
    text: String,
    onTextChange: (String) -> Unit,
    isRecording: Boolean = false,
    isGenerating: Boolean,
    isDisabled: Boolean,
    pendingImageUris: List<Uri>,
    pendingFile: PendingFile?,
    onRemoveImage: (Int) -> Unit,
    onRemoveFile: () -> Unit,
    onSend: () -> Unit,
    onAttachment: () -> Unit,
    onMicTap: () -> Unit = {},
    onStopGeneration: () -> Unit
) {
    val showSend = text.isNotBlank() || pendingImageUris.isNotEmpty() || pendingFile != null

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(Color(0xFFF7F5F0))
    ) {
        // Pending attachments preview
        if (pendingImageUris.isNotEmpty() || pendingFile != null) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState())
                    .padding(horizontal = 12.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                // Image previews
                pendingImageUris.forEachIndexed { index, uri ->
                    Box {
                        AsyncImage(
                            model = uri,
                            contentDescription = "Pending image",
                            modifier = Modifier
                                .size(60.dp)
                                .clip(RoundedCornerShape(8.dp)),
                            contentScale = ContentScale.Crop
                        )

                        IconButton(
                            onClick = { onRemoveImage(index) },
                            modifier = Modifier
                                .align(Alignment.TopEnd)
                                .offset(x = 6.dp, y = (-6).dp)
                                .size(24.dp)
                                .background(Color.Black.copy(alpha = 0.6f), CircleShape)
                        ) {
                            Icon(
                                Icons.Default.Close,
                                contentDescription = "Remove",
                                modifier = Modifier.size(14.dp),
                                tint = Color.White
                            )
                        }
                    }
                }

                // File preview
                pendingFile?.let { file ->
                    Box {
                        Surface(
                            modifier = Modifier.size(60.dp),
                            shape = RoundedCornerShape(8.dp),
                            color = Color(0xFFE5E5EA)
                        ) {
                            Column(
                                modifier = Modifier.fillMaxSize(),
                                horizontalAlignment = Alignment.CenterHorizontally,
                                verticalArrangement = Arrangement.Center
                            ) {
                                Icon(
                                    Icons.Default.Description,
                                    contentDescription = null,
                                    tint = Color(0xFF007AFF),
                                    modifier = Modifier.size(24.dp)
                                )
                                Text(
                                    text = file.fileName,
                                    fontSize = 10.sp,
                                    maxLines = 1,
                                    modifier = Modifier.padding(horizontal = 4.dp)
                                )
                            }
                        }

                        IconButton(
                            onClick = onRemoveFile,
                            modifier = Modifier
                                .align(Alignment.TopEnd)
                                .offset(x = 6.dp, y = (-6).dp)
                                .size(24.dp)
                                .background(Color.Black.copy(alpha = 0.6f), CircleShape)
                        ) {
                            Icon(
                                Icons.Default.Close,
                                contentDescription = "Remove",
                                modifier = Modifier.size(14.dp),
                                tint = Color.White
                            )
                        }
                    }
                }
            }
        }

        // Input row
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            // + button
            IconButton(
                onClick = onAttachment,
                enabled = !isDisabled,
                modifier = Modifier
                    .size(36.dp)
                    .background(Color(0x72DFDAD1), CircleShape)
            ) {
                Icon(
                    Icons.Default.Add,
                    contentDescription = "Add attachment",
                    tint = if (isDisabled) Color(0xFF6B6E66).copy(alpha = 0.4f) else Color(0xFF6B6E66)
                )
            }

            // Text field
            Surface(
                modifier = Modifier.weight(1f),
                shape = RoundedCornerShape(22.dp),
                color = Color.White.copy(alpha = 0.72f),
                border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFFDFDAD1))
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 14.dp, end = 10.dp, top = 8.dp, bottom = 8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    BasicTextField(
                        value = text,
                        onValueChange = onTextChange,
                        modifier = Modifier.weight(1f),
                        enabled = !isDisabled && !isRecording,
                        textStyle = TextStyle(fontSize = 16.sp, color = Color(0xFF171A17)),
                        cursorBrush = SolidColor(Color(0xFF1A4334)),
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                        keyboardActions = KeyboardActions(onSend = {
                            if (showSend) onSend()
                        }),
                        decorationBox = { innerTextField ->
                            Box {
                                if (text.isEmpty()) {
                                    Text(
                                        text = if (isRecording) "Listening..." else "Ask anything",
                                        fontSize = 16.sp,
                                        color = if (isRecording) Color(0xFFFF3B30) else Color(0xFF8E8E93)
                                    )
                                }
                                innerTextField()
                            }
                        }
                    )

                    Spacer(modifier = Modifier.width(8.dp))

                    // Mic button - show when not generating
                    if (!isGenerating) {
                        IconButton(
                            onClick = onMicTap,
                            modifier = Modifier.size(28.dp)
                        ) {
                            Icon(
                                if (isRecording) Icons.Default.StopCircle else Icons.Default.Mic,
                                contentDescription = if (isRecording) "Stop recording" else "Start recording",
                                modifier = Modifier.size(18.dp),
                                tint = if (isRecording) Color(0xFFFF3B30) else Color(0xFF6B6B6B)
                            )
                        }
                    }

                    // Generation status or send button
                    if (isGenerating) {
                        Surface(
                            shape = RoundedCornerShape(12.dp),
                            color = Color(0xFFE5E5EA),
                            modifier = Modifier.clickable { onStopGeneration() }
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(4.dp)
                            ) {
                                CircularProgressIndicator(
                                    modifier = Modifier.size(12.dp),
                                    strokeWidth = 2.dp,
                                    color = Color(0xFF6B6B6B)
                                )
                                Text(
                                    text = "Generating",
                                    fontSize = 12.sp,
                                    color = Color(0xFF6B6B6B)
                                )
                            }
                        }
                    } else if (showSend) {
                        IconButton(
                            onClick = onSend,
                            enabled = !isDisabled,
                            modifier = Modifier.size(28.dp)
                        ) {
                            Icon(
                                Icons.Default.ArrowUpward,
                                contentDescription = "Send",
                                    modifier = Modifier
                                    .size(28.dp)
                                    .background(Color(0xFF1A4334), CircleShape)
                                    .padding(4.dp),
                                tint = Color.White
                            )
                        }
                    }
                }
            }
        }
    }
}
