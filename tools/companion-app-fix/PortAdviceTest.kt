package com.exojosh.minecraftsecondscreen.net

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The banner is the only thing the player sees when the port is answered by
 * something else, and the old wording sent them to a command that needs a PC
 * and a cable. These pin who gets named and that the advice stays on-device.
 */
class PortAdviceTest {

    @Test
    fun the_mods_own_hello_is_a_working_connection_with_nothing_to_say() {
        val advice = advise(MOD_ID, PROTOCOL)
        assertEquals(PortOwner.MOD, advice.owner)
        assertNull(advice.message)
    }

    @Test
    fun adbd_is_named_and_the_fix_needs_no_command_from_the_thor() {
        val advice = advise("adbd", PROTOCOL)
        assertEquals(PortOwner.ADBD, advice.owner)
        val message = requireNotNull(advice.message)
        assertTrue(message, message.contains("adb reverse"))
        // The instruction has to be doable on the handheld: unplugging.
        assertTrue(message, message.contains("Unplug the USB cable"))
        assertTrue(message, message.contains("Nothing to do on the Thor"))
        assertTrue(message, message.contains(PORT.toString()))
    }

    @Test
    fun silence_is_not_blamed_on_adb() {
        val advice = advise(null, null)
        assertEquals(PortOwner.SILENT, advice.owner)
        val message = requireNotNull(advice.message)
        assertTrue(message, !message.contains("adb reverse"))
        assertTrue(message, message.contains(PORT.toString()))
    }

    @Test
    fun another_mod_or_another_protocol_says_which_and_asks_for_a_matching_release() {
        val other = advise("someone_elses_hud", PROTOCOL)
        assertEquals(PortOwner.OTHER, other.owner)
        assertTrue(other.message!!, other.message.contains("someone_elses_hud"))

        val old = advise(MOD_ID, PROTOCOL + 1)
        assertEquals(PortOwner.OTHER, old.owner)
        assertTrue(old.message!!, old.message.contains("same release"))
    }

    @Test
    fun every_message_names_the_port_it_talks_about() {
        for ((mod, protocol) in listOf("adbd" to PROTOCOL, null to null, "other" to PROTOCOL)) {
            val message = advise(mod, protocol, port = 1234).message
            assertNotNull(message)
            assertTrue(message!!, message.contains("1234"))
        }
    }
}
