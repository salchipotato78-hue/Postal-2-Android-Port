package com.postal2.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.os.Bundle
import android.os.SystemClock
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.SeekBar
import android.widget.Switch
import android.widget.TextView
import androidx.activity.ComponentActivity
import java.io.File
import java.util.Locale
import kotlin.math.hypot
import kotlin.math.min

/// Editor de controles táctiles. Arrastra cada control en la vista previa para moverlo; los
/// deslizadores cambian tamaño y opacidad. Los cambios se guardan en `touch_layout.txt`, que el
/// juego vuelve a leer solo, así que se ven en vivo al volver al juego.
class EditorActivity : ComponentActivity() {

    private val header = "# Postal 2 touch layout v3"
    private val file by lazy { File(filesDir, "touch_layout.txt") }
    private var items: MutableList<Item> = mutableListOf()
    private var lastSave = 0L

    /// Un control. `key` es el nombre que usa el archivo (stick, fire, alt_fire...).
    data class Item(
        val key: String,
        val title: String,
        var x: Float,
        var y: Float,
        var size: Float,
        var opacity: Float,
        var visible: Boolean,
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        items = load()

        val preview = LayoutPreview(
            this,
            { items },
            { saveThrottled() },
            { saveAll() },
        )
        val previewHeight = (resources.displayMetrics.heightPixels * 0.42f).toInt()

        val controls = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        for (item in items) controls.addView(section(item, preview))

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.BLACK)
            setPadding(16, 16, 16, 16)
        }
        root.addView(preview, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, previewHeight))
        root.addView(TextView(this).apply {
            text = "Arrastra un control para moverlo. Al volver al juego se aplican los cambios."
            setTextColor(Color.WHITE)
            textSize = 14f
        })
        root.addView(Button(this).apply {
            text = "Restablecer diseño"
            setOnClickListener {
                items = defaults()
                saveAll()
                recreate()
            }
        })
        root.addView(
            ScrollView(this).apply { addView(controls) },
            LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f),
        )
        setContentView(root)
    }

    private fun section(item: Item, preview: View): LinearLayout {
        val box = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 16, 0, 8)
        }
        box.addView(TextView(this).apply {
            text = item.title
            setTextColor(Color.rgb(240, 190, 80))
            textSize = 16f
        })
        box.addView(Switch(this).apply {
            text = "Visible"
            setTextColor(Color.WHITE)
            isChecked = item.visible
            setOnCheckedChangeListener { _, checked ->
                item.visible = checked
                saveAll()
                preview.invalidate()
            }
        })
        box.addView(slider("Tamaño", item.size, 0.04f, 0.5f) {
            item.size = it
            saveThrottled()
            preview.invalidate()
        })
        box.addView(slider("Opacidad", item.opacity, 0f, 1f) {
            item.opacity = it
            saveThrottled()
            preview.invalidate()
        })
        return box
    }

    private fun slider(name: String, value: Float, lo: Float, hi: Float, onChange: (Float) -> Unit): LinearLayout {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        val label = TextView(this).apply {
            setTextColor(Color.WHITE)
            text = "$name ${"%.2f".format(Locale.US, value)}"
            textSize = 14f
            minWidth = 240
        }
        val seek = SeekBar(this).apply {
            max = 1000
            progress = ((value - lo) / (hi - lo) * 1000f).toInt().coerceIn(0, 1000)
            setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                override fun onProgressChanged(bar: SeekBar, p: Int, fromUser: Boolean) {
                    if (!fromUser) return
                    val v = lo + (hi - lo) * p / 1000f
                    label.text = "$name ${"%.2f".format(Locale.US, v)}"
                    onChange(v)
                }
                override fun onStartTrackingTouch(bar: SeekBar) {}
                override fun onStopTrackingTouch(bar: SeekBar) {}
            })
        }
        row.addView(label)
        row.addView(seek, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        return row
    }

    /// Diseño por defecto. Debe coincidir con `Layout::default()` en crates/p2touch/src/lib.rs.
    private fun defaults(): MutableList<Item> = mutableListOf(
        Item("stick", "Joystick (moverse)", 0.106f, 0.794f, 0.235f, 0.8f, true),
        Item("fire", "Disparar", 0.869f, 0.805f, 0.20f, 0.8f, true),
        Item("alt_fire", "Alt (disparo alternativo)", 0.832f, 0.923f, 0.07f, 0.8f, true),
        Item("reload", "Recargar", 0.051f, 0.601f, 0.17f, 0.8f, true),
        Item("jump", "Saltar", 0.920f, 0.695f, 0.14f, 0.8f, true),
        Item("crouch", "Agacharse", 0.934f, 0.884f, 0.125f, 0.8f, true),
        Item("use", "Usar", 0.818f, 0.704f, 0.14f, 0.8f, true),
        Item("kick", "Patear", 0.791f, 0.843f, 0.14f, 0.8f, true),
        Item("next_weapon", "Arma siguiente", 0.973f, 0.573f, 0.11f, 0.8f, true),
        Item("pause", "Pausa", 0.964f, 0.058f, 0.075f, 0.8f, true),
        Item("holster", "Guardar arma (sin icono)", 0.60f, 0.90f, 0.08f, 0.8f, false),
        Item("prev_weapon", "Arma anterior (sin icono)", 0.55f, 0.90f, 0.08f, 0.8f, false),
    ).toMutableList()

    /// Lee el archivo si es de esta versión; si no, usa el diseño por defecto.
    private fun load(): MutableList<Item> {
        val base = defaults()
        if (!file.exists()) return base
        val text = file.readText()
        if (!text.startsWith(header)) return base
        for (line in text.lines()) {
            val p = line.trim().split(Regex("\\s+"))
            val item = when (p.getOrNull(0)) {
                "stick" -> base.firstOrNull { it.key == "stick" }
                "button" -> base.firstOrNull { it.key == p.getOrNull(1) }
                else -> null
            } ?: continue
            val off = if (p[0] == "stick") 1 else 2
            val x = p.getOrNull(off)?.toFloatOrNull() ?: continue
            val y = p.getOrNull(off + 1)?.toFloatOrNull() ?: continue
            val size = p.getOrNull(off + 2)?.toFloatOrNull() ?: continue
            val opacity = p.getOrNull(off + 3)?.toFloatOrNull() ?: continue
            item.x = x.coerceIn(0f, 1f)
            item.y = y.coerceIn(0f, 1f)
            item.size = size.coerceIn(0.04f, 0.5f)
            item.opacity = opacity.coerceIn(0f, 1f)
            item.visible = p.getOrNull(off + 4) == "1"
        }
        return base
    }

    private fun saveAll() {
        val sb = StringBuilder(header).append('\n')
        for (item in items) {
            val vis = if (item.visible) 1 else 0
            val nums = String.format(Locale.US, "%.3f %.3f %.3f %.3f %d", item.x, item.y, item.size, item.opacity, vis)
            if (item.key == "stick") sb.append("stick ").append(nums).append('\n')
            else sb.append("button ").append(item.key).append(' ').append(nums).append('\n')
        }
        sb.append("look 1.000\n")
        file.writeText(sb.toString())
        lastSave = SystemClock.uptimeMillis()
    }

    /// Guarda mientras se arrastra, como mucho cada 150 ms.
    private fun saveThrottled() {
        if (SystemClock.uptimeMillis() - lastSave >= 150) saveAll()
    }
}

/// Vista previa del diseño: dibuja los controles con sus iconos y deja arrastrarlos.
private class LayoutPreview(
    context: Context,
    private val items: () -> List<EditorActivity.Item>,
    private val onMoved: (EditorActivity.Item) -> Unit,
    private val onEnd: () -> Unit,
) : View(context) {

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val bitmaps = HashMap<String, Bitmap?>()
    private var dragging: EditorActivity.Item? = null
    private var grabX = 0f
    private var grabY = 0f

    private fun bitmap(name: String): Bitmap? = bitmaps.getOrPut(name) {
        try {
            context.assets.open("touch/$name").use { BitmapFactory.decodeStream(it) }
        } catch (e: Exception) {
            Log.w("Postal2", "no se pudo cargar touch/$name: ${e.message}")
            null
        }
    }

    /// Icono de cada control. Los que no tienen icono se dibujan como un círculo con su nombre.
    private fun iconFor(key: String): String? = when (key) {
        "stick" -> "joystick-base.png"
        "fire" -> "disparar.png"
        "alt_fire" -> "Alt.png"
        "reload" -> "Recargar.png"
        "jump" -> "saltar.png"
        "crouch" -> "agacharse.png"
        "use" -> "usar.png"
        "kick" -> "Patear.png"
        "next_weapon" -> "cambiar-arma.png"
        "pause" -> "pausa.png"
        else -> null
    }

    private fun drawIcon(c: Canvas, name: String, rect: RectF, alpha: Float) {
        val bmp = bitmap(name) ?: return
        paint.alpha = (alpha * 255).toInt()
        c.drawBitmap(bmp, null, rect, paint)
        paint.alpha = 255
    }

    override fun onDraw(c: Canvas) {
        super.onDraw(c)
        c.drawColor(Color.rgb(45, 45, 45))
        val w = width.toFloat()
        val h = height.toFloat()
        val m = min(w, h)
        for (item in items()) {
            val d = item.size * m
            val cx = item.x * w
            val cy = item.y * h
            val alpha = if (item.visible) item.opacity else 0.25f
            val rect = RectF(cx - d / 2, cy - d / 2, cx + d / 2, cy + d / 2)
            val bmp = iconFor(item.key)?.let { bitmap(it) }
            if (bmp != null) {
                drawBitmap(c, bmp, rect, alpha)
                if (item.key == "stick") {
                    val k = d * 0.5f
                    val knob = bitmap("joystick-adentro.png")
                    if (knob != null) drawBitmap(c, knob, RectF(cx - k / 2, cy - k / 2, cx + k / 2, cy + k / 2), alpha)
                }
            } else {
                // Sin icono (o no cargó): círculo con el nombre, para que siempre se vea dónde está.
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = 4f
                paint.color = Color.rgb(230, 140, 60)
                paint.alpha = (alpha * 255).toInt()
                c.drawCircle(cx, cy, d / 2, paint)
                paint.style = Paint.Style.FILL
                paint.color = Color.WHITE
                paint.textSize = d * 0.18f
                paint.textAlign = Paint.Align.CENTER
                c.drawText(item.title.split(' ').first(), cx, cy + d * 0.06f, paint)
                paint.alpha = 255
            }
        }
    }

    private fun drawBitmap(c: Canvas, bmp: Bitmap, rect: RectF, alpha: Float) {
        paint.alpha = (alpha * 255).toInt()
        c.drawBitmap(bmp, null, rect, paint)
        paint.alpha = 255
    }

    override fun onTouchEvent(e: MotionEvent): Boolean {
        val w = width.toFloat()
        val h = height.toFloat()
        val m = min(w, h)
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                dragging = items()
                    .filter { hypot(e.x - it.x * w, e.y - it.y * h) <= it.size * m * 0.5f }
                    .minByOrNull { hypot(e.x - it.x * w, e.y - it.y * h) }
                val d = dragging ?: return false
                grabX = e.x - d.x * w
                grabY = e.y - d.y * h
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                val d = dragging ?: return false
                // Mismo criterio que el juego: el control queda completo dentro de la vista.
                val rx = (d.size * m * 0.5f / w).coerceAtMost(0.5f)
                val ry = (d.size * m * 0.5f / h).coerceAtMost(0.5f)
                d.x = ((e.x - grabX) / w).coerceIn(rx, 1f - rx)
                d.y = ((e.y - grabY) / h).coerceIn(ry, 1f - ry)
                onMoved(d)
                invalidate()
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                if (dragging != null) {
                    dragging = null
                    onEnd()
                }
                return true
            }
        }
        return super.onTouchEvent(e)
    }
}
