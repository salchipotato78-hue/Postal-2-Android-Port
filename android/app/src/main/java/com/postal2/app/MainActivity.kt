package com.postal2.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import java.io.File
import java.util.zip.ZipInputStream
import kotlin.concurrent.thread

/// Pantalla de inicio: elegir el ZIP con los datos de tu copia de POSTAL 2 y jugar.
/// El ZIP se extrae en el almacenamiento privado de la app y se busca la carpeta que contiene
/// System/, Maps/ y Textures/. Nada del juego viene incluido en la app.
class MainActivity : ComponentActivity() {

    private lateinit var status: TextView
    private lateinit var playButton: Button
    private lateinit var pickButton: Button

    private val pickZip = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) importZip(uri)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        status = TextView(this).apply {
            textSize = 16f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
        }
        pickButton = Button(this).apply {
            text = "Elegir ZIP de datos de POSTAL 2"
            setOnClickListener {
                pickZip.launch(arrayOf("application/zip", "application/x-zip-compressed", "application/octet-stream"))
            }
        }
        playButton = Button(this).apply {
            text = "Jugar"
            setOnClickListener {
                startActivity(Intent(this@MainActivity, GameActivity::class.java))
            }
        }

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setBackgroundColor(Color.BLACK)
            setPadding(64, 64, 64, 64)
            addView(status)
            addView(pickButton)
            addView(playButton)
        }
        setContentView(root)
        refresh()
    }

    private fun gameRootFile() = File(filesDir, "game_root.txt")
    private fun extractDir() = File(filesDir, "postal2")

    private fun refresh(message: String? = null) {
        val ready = gameRootFile().exists() && File(gameRootFile().readText()).isDirectory
        status.text = message ?: if (ready) "Datos listos." else "Sin datos. Elige el ZIP con tu copia del juego."
        playButton.isEnabled = ready
    }

    private fun importZip(uri: android.net.Uri) {
        status.text = "Extrayendo… no cierres la app."
        pickButton.isEnabled = false
        playButton.isEnabled = false
        thread {
            try {
                val dest = extractDir()
                if (dest.exists()) dest.deleteRecursively()
                dest.mkdirs()
                val destPath = dest.canonicalPath + File.separator

                contentResolver.openInputStream(uri)?.use { input ->
                    ZipInputStream(input).use { zin ->
                        while (true) {
                            val entry = zin.nextEntry ?: break
                            val out = File(dest, entry.name).canonicalFile
                            // Protección contra "zip slip": nada puede salir de la carpeta de destino.
                            if (out.path.startsWith(destPath)) {
                                if (entry.isDirectory) {
                                    out.mkdirs()
                                } else {
                                    out.parentFile?.mkdirs()
                                    out.outputStream().use { zin.copyTo(it) }
                                }
                            }
                            zin.closeEntry()
                        }
                    }
                } ?: throw IllegalStateException("No se pudo abrir el ZIP")

                val root = findGameRoot(dest)
                    ?: throw IllegalStateException("El ZIP no contiene las carpetas System/ y Maps/")
                gameRootFile().writeText(root.absolutePath)
                runOnUiThread {
                    pickButton.isEnabled = true
                    refresh("Datos importados.")
                }
            } catch (e: Exception) {
                runOnUiThread {
                    pickButton.isEnabled = true
                    refresh("Error: ${e.message}")
                }
            }
        }
    }

    /// Busca la carpeta que contiene System/ y Maps/ (hasta 4 niveles, porque el ZIP suele traer una carpeta envolvente).
    private fun findGameRoot(dir: File): File? =
        dir.walkTopDown().maxDepth(4).firstOrNull {
            it.isDirectory && File(it, "System").isDirectory && File(it, "Maps").isDirectory
        }
}
