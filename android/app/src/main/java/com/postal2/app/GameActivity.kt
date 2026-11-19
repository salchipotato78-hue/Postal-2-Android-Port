package com.postal2.app

import android.app.NativeActivity
import android.os.Build
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController

/// Actividad nativa (Bevy/winit vía android-activity). Lee la ruta de datos desde
/// `filesDir/game_root.txt`, que escribe MainActivity al importar el ZIP.
///
/// Modo inmersivo: se ocultan la barra de estado y los botones de navegación (atrás, inicio,
/// recientes). Reaparecen con un deslizamiento desde el borde y vuelven a ocultarse solos.
class GameActivity : NativeActivity() {

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) hideSystemBars()
    }

    private fun hideSystemBars() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            window.setDecorFitsSystemWindows(false)
            window.insetsController?.let { controller ->
                controller.hide(WindowInsets.Type.systemBars())
                controller.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            }
        } else {
            @Suppress("DEPRECATION")
            window.decorView.systemUiVisibility = (
                View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                    or View.SYSTEM_UI_FLAG_FULLSCREEN
                    or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                    or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                    or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                    or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                )
        }
    }
}
