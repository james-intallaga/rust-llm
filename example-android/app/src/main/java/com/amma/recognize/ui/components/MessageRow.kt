package com.amma.recognize.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Description
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import com.amma.recognize.data.ChatMessageUI
import com.amma.recognize.data.ChatRole

@Composable
fun MessageRow(
    message: ChatMessageUI,
    onImageTap: ((String) -> Unit)? = null
) {
    val isUser = message.role == ChatRole.USER

    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = if (isUser) Alignment.End else Alignment.Start,
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        // Images
        message.images.forEach { uri ->
            AsyncImage(
                model = uri,
                contentDescription = "Attached image",
                modifier = Modifier
                    .size(120.dp)
                    .clip(RoundedCornerShape(16.dp))
                    .clickable { onImageTap?.invoke(uri) },
                contentScale = ContentScale.Crop
            )
        }

        // File attachments
        message.files.forEach { fileName ->
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = if (isUser) Color(0xFF1A4334) else Color(0x72DFDAD1)
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        Icons.Default.Description,
                        contentDescription = null,
                        tint = if (isUser) Color.White else Color(0xFF1A4334),
                        modifier = Modifier.size(16.dp)
                    )
                    Text(
                        text = fileName,
                        fontSize = 14.sp,
                        color = if (isUser) Color.White else Color.Black,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                }
            }
        }

        // Text
        val textToShow = message.text
        val isPlaceholder = textToShow == "[Image attached]" ||
                           textToShow == "[File attached]" ||
                           textToShow.startsWith("[") && textToShow.endsWith("attached]")

        if (textToShow.isNotBlank() && !isPlaceholder) {
            Surface(
                shape = RoundedCornerShape(18.dp),
                color = if (isUser) Color(0xFF1A4334) else Color.Transparent
            ) {
                Text(
                    text = textToShow,
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                    fontSize = 16.sp,
                    color = if (isUser) Color.White else Color(0xFF171A17)
                )
            }
        }
    }
}
