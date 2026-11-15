package org.fontes.app

/** Pauses are measured after the preceding primitive ends, not from its onset. */
internal object VerseArrivalPattern {
    data class Tick(val pauseMs: Int, val scale: Float)

    val ticks = listOf(
        Tick(0, .55f),
        Tick(60, .44f),
        Tick(95, .34f),
        Tick(145, .25f),
        Tick(210, .17f),
        Tick(285, .11f),
        Tick(375, .065f),
    )

    fun durationMs(primitiveMs: Int): Long =
        ticks.sumOf { it.pauseMs.toLong() + primitiveMs }

    fun startsMs(primitiveMs: Int): List<Long> {
        var elapsed = 0L
        return ticks.map {
            elapsed += it.pauseMs
            val start = elapsed
            elapsed += primitiveMs
            start
        }
    }
}
