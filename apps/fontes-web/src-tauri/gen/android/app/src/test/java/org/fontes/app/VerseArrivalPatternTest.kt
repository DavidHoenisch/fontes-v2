package org.fontes.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class VerseArrivalPatternTest {
    @Test fun ticksSlowDownAndFadeWithoutRepeating() {
        val ticks = VerseArrivalPattern.ticks
        assertTrue(ticks.size in 4..10) // Bounded, including the phone's 10-primitive limit.
        assertEquals(0, ticks.first().pauseMs)
        ticks.zipWithNext().forEach { (previous, next) ->
            assertTrue(next.pauseMs > previous.pauseMs)
            assertTrue(next.scale < previous.scale)
        }
        assertTrue(ticks.drop(1).all { it.pauseMs >= 50 })
        assertTrue(ticks.all { it.scale > 0f && it.scale <= .6f })
    }

    @Test fun supportedDurationsFitInsideVisualArrival() {
        for (primitiveMs in 1..50) {
            assertTrue(VerseArrivalPattern.durationMs(primitiveMs) in 1000L..1600L)
            val starts = VerseArrivalPattern.startsMs(primitiveMs)
            val gaps = starts.zipWithNext { previous, next -> next - previous }
            assertTrue(gaps.zipWithNext().all { (previous, next) -> next > previous })
            assertEquals(VerseArrivalPattern.durationMs(primitiveMs), starts.last() + primitiveMs)
        }
    }
}
