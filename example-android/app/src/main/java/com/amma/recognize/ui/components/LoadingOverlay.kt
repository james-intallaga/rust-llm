package com.amma.recognize.ui.components

import androidx.compose.foundation.border
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.amma.recognize.data.LoadingStage

@Composable
fun LoadingOverlay(
    progress: Double,
    stage: LoadingStage,
    onRetry: () -> Unit
) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFFF7F5F0)),
        contentAlignment = Alignment.Center
    ) {
        Column(
            modifier = Modifier.offset(y = (-20).dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(28.dp)
        ) {
            Box(
                modifier = Modifier
                    .size(82.dp)
                    .border(1.dp, Color(0x381A4334), CircleShape),
                contentAlignment = Alignment.Center
            ) {
                Box(
                    modifier = Modifier
                        .size(59.dp)
                        .background(Color(0xFF1A4334), CircleShape),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "A",
                        fontSize = 32.sp,
                        fontWeight = FontWeight.Medium,
                        fontFamily = FontFamily.Serif,
                        color = Color.White
                    )
                }
            }

            Text(stage.displayText, fontSize = 16.sp, fontWeight = FontWeight.Medium, color = Color(0xFF171A17))

            if (stage == LoadingStage.DOWNLOADING || stage == LoadingStage.LOADING) {
                Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    LinearProgressIndicator(
                        progress = { progress.toFloat() },
                        modifier = Modifier.width(176.dp).height(3.dp),
                        color = Color(0xFF1A4334),
                        trackColor = Color(0xFFDFDAD1),
                        strokeCap = androidx.compose.ui.graphics.StrokeCap.Round
                    )
                    Text("${(progress * 100).toInt()}%", fontSize = 12.sp, color = Color(0xFF6B6E66))
                }
            } else if (stage == LoadingStage.ERROR) {
                Button(
                    onClick = onRetry,
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF1A4334)),
                    shape = RoundedCornerShape(24.dp)
                ) {
                    Text("Retry", modifier = Modifier.padding(horizontal = 12.dp, vertical = 2.dp))
                }
            } else {
                CircularProgressIndicator(color = Color(0xFF1A4334), modifier = Modifier.size(22.dp), strokeWidth = 2.dp)
            }
        }
    }
}
