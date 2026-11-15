package org.fontes.app

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.util.Base64
import androidx.core.content.FileProvider
import java.io.File
import java.util.UUID

// Only app-created PNG bytes cross this bridge; callers cannot supply a path or recipient.
object VerseImageShare {
  fun open(activity: Activity, dataUrl: String, caption: String): String {
    return try {
      require(dataUrl.startsWith("data:image/png;base64,") && dataUrl.length <= 8_000_000) { "Invalid or oversized share image." }
      require(caption.length <= 20_000) { "Share caption is too long." }
      val bytes = Base64.decode(dataUrl.substringAfter(','), Base64.DEFAULT)
      val signature = byteArrayOf(137.toByte(), 80, 78, 71, 13, 10, 26, 10)
      require(bytes.size >= 24 && bytes.take(8).toByteArray().contentEquals(signature)) { "Invalid PNG image." }
      val directory = File(activity.cacheDir, "share-cards").apply { mkdirs() }
      // Keep recent files for receiving apps; remove only expired generated cards.
      directory.listFiles()?.filter { it.name.endsWith(".png") && it.lastModified() < System.currentTimeMillis() - 7L * 24 * 60 * 60 * 1000 }?.forEach { it.delete() }
      val image = File(directory, "fontes-${UUID.randomUUID()}.png")
      image.writeBytes(bytes)
      val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", image)
      val send = Intent(Intent.ACTION_SEND).apply {
        type = "image/png"
        putExtra(Intent.EXTRA_STREAM, uri)
        putExtra(Intent.EXTRA_TEXT, caption)
        putExtra(Intent.EXTRA_TITLE, "Share passage")
        clipData = ClipData.newUri(activity.contentResolver, "Scripture card", uri)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      }
      activity.startActivity(Intent.createChooser(send, "Share passage"))
      ""
    } catch (error: Exception) {
      error.message ?: "Could not open Android share options."
    }
  }
}
