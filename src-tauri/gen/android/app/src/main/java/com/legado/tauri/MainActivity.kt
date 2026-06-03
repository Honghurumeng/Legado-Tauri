package com.legado.tauri

import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {
  private var readerImmersiveModeEnabled = false

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.addJavascriptInterface(LegadoAndroidInputBridge(), "LegadoAndroidInput")
    applyReaderImmersiveMode()
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) {
      applyReaderImmersiveMode()
    }
  }

  private fun applyReaderImmersiveMode() {
    val controller = WindowInsetsControllerCompat(window, window.decorView)
    controller.systemBarsBehavior =
      WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE

    if (readerImmersiveModeEnabled) {
      WindowCompat.setDecorFitsSystemWindows(window, false)
      controller.hide(WindowInsetsCompat.Type.systemBars())
    } else {
      WindowCompat.setDecorFitsSystemWindows(window, true)
      controller.show(WindowInsetsCompat.Type.systemBars())
    }
  }

  private inner class LegadoAndroidInputBridge {
    @JavascriptInterface
    fun setReaderImmersiveModeEnabled(enabled: Boolean) {
      runOnUiThread {
        readerImmersiveModeEnabled = enabled
        applyReaderImmersiveMode()
      }
    }
  }
}
