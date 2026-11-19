package com.postal2.app

import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import java.io.File
import java.io.FilterInputStream
import java.io.InputStream
import java.util.zip.ZipInputStream
import kotlin.concurrent.thread

/// Pantalla de inicio: elegir el ZIP con los datos de tu copia de POSTAL 2 y jugar.
/// El ZIP se extrae en el almacenamiento privado de la app y se busca la carpeta que contiene
/// System/, Maps/ y Textures/. Nada del juego viene incluido en la app.
class MainActivity : ComponentActivity() {

    private lateinit var status: TextView
    private lateinit var progress: ProgressBar
    private lateinit var progressText: TextView
    private lateinit var playButton: Button
    private lateinit var pickButton: Button
    private lateinit var editButton: Button

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
        progress = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply {
            max = 1000
            progress = 0
            visibility = View.GONE
        }
        progressText = TextView(this).apply {
            textSize = 14f
            setTextColor(Color.LTGRAY)
            gravity = Gravity.CENTER
            visibility = View.GONE
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

        editButton = Button(this).apply {
            text = "Editar controles táctiles"
            setOnClickListener { startActivity(Intent(this@MainActivity, EditorActivity::class.java)) }
        }

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setBackgroundColor(Color.BLACK)
            setPadding(64, 64, 64, 64)
            addView(status)
            addView(progress)
            addView(progressText)
            addView(pickButton)
            addView(playButton)
            addView(editButton)
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

    /// Muestra u oculta la barra de progreso y bloquea los botones mientras se extrae.
    private fun setBusy(busy: Boolean) {
        pickButton.isEnabled = !busy
        playButton.isEnabled = !busy && gameRootFile().exists()
        progress.visibility = if (busy) View.VISIBLE else View.GONE
        progressText.visibility = if (busy) View.VISIBLE else View.GONE
        progress.progress = 0
        progressText.text = ""
    }

    private fun reportProgress(fraction: Float) {
        runOnUiThread {
            val pct = (fraction * 100).toInt().coerceIn(0, 100)
            progress.progress = pct * 10
            progressText.text = "Extrayendo… $pct %"
        }
    }

    private fun importZip(uri: Uri) {
        status.text = "Extrayendo… no cierres la app."
        setBusy(true)
        thread {
            try {
                val total = contentResolver.openFileDescriptor(uri, "r")?.use { it.statSize } ?: -1L
                val dest = extractDir()
                if (dest.exists()) dest.deleteRecursively()
                dest.mkdirs()
                val destPath = dest.canonicalPath + File.separator

                val input = contentResolver.openInputStream(uri)
                    ?: throw IllegalStateException("No se pudo abrir el ZIP")
                // Cuenta los bytes del ZIP que se van leyendo: así el progreso es sobre el archivo real.
                val counted = CountingStream(input)
                var lastPct = -1
                ZipInputStream(counted).use { zin ->
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

                        if (total > 0) {
                            val pct = (counted.count * 100 / total).toInt().coerceIn(0, 100)
                            if (pct != lastPct) {
                                lastPct = pct
                                reportProgress(counted.count.toFloat() / total)
                            }
                        }
                    }
                }

                val root = findGameRoot(dest)
                    ?: throw IllegalStateException("El ZIP no contiene las carpetas System/ y Maps/")
                gameRootFile().writeText(root.absolutePath)
                runOnUiThread {
                    setBusy(false)
                    refresh("Datos importados.")
                }
            } catch (e: Exception) {
                runOnUiThread {
                    setBusy(false)
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

/// Stream que cuenta los bytes leídos. Sirve para calcular el progreso de la extracción.
private class CountingStream(input: InputStream) : FilterInputStream(input) {
    var count: Long = 0
        private set

    override fun read(): Int {
        val r = super.read()
        if (r >= 0) count++
        return r
    }

    override fun read(b: ByteArray, off: Int, len: Int): Int {
        val r = super.read(b, off, len)
        if (r > 0) count += r
        return r
    }
}
