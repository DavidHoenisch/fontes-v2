package org.fontes.app

import android.content.Context
import android.media.AudioAttributes
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.provider.Settings
import android.view.HapticFeedbackConstants
import android.view.View

/** UI-thread only. Finite feedback that yields to OS settings and activity lifecycle. */
internal class VerseArrivalHaptics(private val context: Context) {
    private val handler = Handler(Looper.getMainLooper())
    private var activeVibrator: Vibrator? = null

    fun cancel() {
        handler.removeCallbacksAndMessages(null)
        activeVibrator?.cancel()
        activeVibrator = null
    }

    private fun canPlay(view: View): Boolean =
        view.isShown && view.hasWindowFocus() && view.isHapticFeedbackEnabled &&
            (Build.VERSION.SDK_INT >= 33 || Settings.System.getInt(
                context.contentResolver, Settings.System.HAPTIC_FEEDBACK_ENABLED, 1
            ) != 0)

    fun play(view: View) {
        cancel() // A new destination replaces the old sequence, never overlaps it.
        if (!canPlay(view)) return
        try {
            if (Build.VERSION.SDK_INT >= 31) {
                val vibrator = context.getSystemService(VibratorManager::class.java)?.defaultVibrator
                if (vibrator != null && vibrator.hasVibrator()) {
                    val primitive = listOf(
                        VibrationEffect.Composition.PRIMITIVE_LOW_TICK,
                        VibrationEffect.Composition.PRIMITIVE_TICK,
                    ).firstOrNull { vibrator.areAllPrimitivesSupported(it) }
                    if (primitive != null) {
                        val primitiveMs = vibrator.getPrimitiveDurations(primitive)[0]
                        // Keep the whole sequence inside the 1.6-second visual arrival.
                        if (primitiveMs in 1..50) {
                            val composition = VibrationEffect.startComposition()
                            VerseArrivalPattern.ticks.forEach {
                                composition.addPrimitive(primitive, it.scale, it.pauseMs)
                            }
                            val effect = composition.compose()
                            if (Build.VERSION.SDK_INT >= 33) {
                                vibrator.vibrate(effect, VibrationAttributes.Builder()
                                    .setUsage(VibrationAttributes.USAGE_TOUCH).build())
                            } else {
                                @Suppress("DEPRECATION")
                                vibrator.vibrate(effect, AudioAttributes.Builder()
                                    .setUsage(AudioAttributes.USAGE_ASSISTANCE_SONIFICATION)
                                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
                            }
                            activeVibrator = vibrator
                            handler.postDelayed({ activeVibrator = null },
                                VerseArrivalPattern.durationMs(primitiveMs))
                            return
                        }
                    }
                }
            }
        } catch (_: RuntimeException) {
            // A missing service/permission/effect cannot interrupt navigation.
            cancel()
        }
        // Older hardware retains the slowing cadence; the OS owns tick strength.
        VerseArrivalPattern.startsMs(20).forEach { start ->
            handler.postDelayed({
                if (canPlay(view)) {
                    val tick = if (Build.VERSION.SDK_INT >= 34)
                        HapticFeedbackConstants.SEGMENT_FREQUENT_TICK
                    else HapticFeedbackConstants.CLOCK_TICK
                    view.performHapticFeedback(tick) // No ignore-settings flags.
                } else cancel()
            }, start)
        }
    }
}
