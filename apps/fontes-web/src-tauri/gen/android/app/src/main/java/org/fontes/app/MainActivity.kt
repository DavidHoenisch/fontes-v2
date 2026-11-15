package org.fontes.app

import android.os.Bundle
import android.view.View
import androidx.annotation.Keep
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  private val arrivalHaptics by lazy { VerseArrivalHaptics(this) }

  // Called through JNI; retain the name in optimized Android builds.
  @Keep
  fun performVerseArrivalHaptics(view: View) {
    arrivalHaptics.play(view)
  }

  @Keep
  fun shareVerseImage(dataUrl: String, caption: String): String = VerseImageShare.open(this, dataUrl, caption)

  override fun onPause() {
    arrivalHaptics.cancel()
    super.onPause()
  }

  override fun onDestroy() {
    arrivalHaptics.cancel()
    super.onDestroy()
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Protect the WebView's viewport, not just document padding: fixed web
    // overlays otherwise still extend underneath Android's system bars.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val types = WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      val safe = windowInsets.getInsets(types)
      view.setPadding(safe.left, safe.top, safe.right, safe.bottom)
      // These dimensions are already applied by the parent. Forward zeroes
      // instead of consuming everything, retaining IME/viewport notifications
      // and avoiding duplicate CSS safe-area padding in newer WebViews.
      WindowInsetsCompat.Builder(windowInsets)
        .setInsets(types, Insets.NONE)
        .build()
    }
    ViewCompat.requestApplyInsets(content)
  }
}
