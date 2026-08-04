package com.amma.recognize.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun AttachmentMenuView(
    modifier: Modifier = Modifier,
    onCamera: () -> Unit,
    onPhotos: () -> Unit,
    onFiles: () -> Unit
) {
    Surface(
        modifier = modifier.padding(horizontal = 20.dp),
        shape = RoundedCornerShape(16.dp),
        color = Color.White.copy(alpha = 0.95f),
        shadowElevation = 8.dp
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 24.dp, vertical = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(20.dp)
        ) {
            AttachmentButton(
                icon = Icons.Default.CameraAlt,
                title = "Camera",
                onClick = onCamera
            )

            AttachmentButton(
                icon = Icons.Default.Photo,
                title = "Photos",
                onClick = onPhotos
            )

            AttachmentButton(
                icon = Icons.Default.AttachFile,
                title = "Files",
                onClick = onFiles
            )
        }
    }
}

@Composable
private fun AttachmentButton(
    icon: ImageVector,
    title: String,
    onClick: () -> Unit
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.clickable { onClick() }
    ) {
        Surface(
            modifier = Modifier.size(70.dp),
            shape = RoundedCornerShape(14.dp),
            color = Color(0xFFF2F2F7)
        ) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    icon,
                    contentDescription = title,
                    modifier = Modifier.size(32.dp),
                    tint = Color.Black
                )
            }
        }

        Spacer(modifier = Modifier.height(10.dp))

        Text(
            text = title,
            fontSize = 15.sp
        )
    }
}
