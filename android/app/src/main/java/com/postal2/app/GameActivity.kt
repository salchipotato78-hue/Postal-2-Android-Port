package com.postal2.app

import android.app.NativeActivity

/// Actividad nativa (Bevy/winit vía android-activity). Lee la ruta de datos desde
/// `filesDir/game_root.txt`, que escribe MainActivity al importar el ZIP.
class GameActivity : NativeActivity()
