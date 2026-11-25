package com.exojosh.minecraftsecondscreen.net

/**
 * Who owns the port the companion app talks to, and what the player can do
 * about it — on the device, without a PC.
 *
 * The banner used to say "clear a leftover 'adb reverse tcp:48291 tcp:48291'",
 * which names the real cause but isn't an instruction anyone can follow from a
 * handheld: `adb` runs on the computer the Thor is plugged into, and Android has
 * no `adb reverse` to clear. Worse, `adb reverse` is per-connection and dies
 * with the cable: shown while nothing is plugged in, the advice was a dead end.
 *
 * [advise] turns the hello line the other end sent (or the lack of one) into
 * the line to show. It is pure so the mapping is unit tested; the socket side
 * only supplies the facts.
 */
enum class PortOwner {
    /** The mod answered: a working connection, not a conflict. */
    MOD,

    /** adbd answered with its own hello: a leftover `adb reverse` on this port. */
    ADBD,

    /** Something accepted and hung up without saying anything. */
    SILENT,

    /** Something speaking our hello that this app can't use. */
    OTHER,
}

/** A verdict plus the line to show, or null when there is nothing to say. */
data class PortAdvice(val owner: PortOwner, val message: String?)

/** The mod's id, from its hello line. */
const val MOD_ID = "aynthor_secondscreen"

/** The socket protocol this app speaks. */
const val PROTOCOL = 1

/** The mod's port. */
const val PORT = 48291

/**
 * Reads what the other end called itself.
 *
 * @param mod the hello's `mod` field, or null when no usable hello arrived.
 * @param protocol the hello's `protocol` field, whatever the mod name; null when
 *     there was no hello.
 * @param expectedProtocol the protocol this build of the app speaks.
 */
fun advise(
    mod: String?,
    protocol: Int?,
    expectedProtocol: Int = PROTOCOL,
    port: Int = PORT,
): PortAdvice {
    if (mod == MOD_ID && protocol == expectedProtocol) {
        return PortAdvice(PortOwner.MOD, null)
    }
    if (mod != null && (mod == "adbd" || mod.contains("adbd"))) {
        return PortAdvice(
            PortOwner.ADBD,
            "adb is forwarding port $port from a computer: that is the 'adb reverse' " +
                "left over from a development session. Unplug the USB cable, or run " +
                "'adb reverse --remove tcp:$port' on that computer, and replay and " +
                "Minecraft reconnect on their own. Nothing to do on the Thor.",
        )
    }
    if (mod != null) {
        // Something that speaks our hello: another mod, or halves from releases
        // that disagree about the protocol.
        return PortAdvice(
            PortOwner.OTHER,
            "Port $port is answered by '$mod' (protocol ${protocol ?: "none"}). This app " +
                "speaks protocol $expectedProtocol: install the mod and the app from the " +
                "same release.",
        )
    }
    return PortAdvice(
        PortOwner.SILENT,
        "Something is on port $port and is not the mod: it accepts and hangs up without " +
            "answering. Close other screen-mirroring or game apps, then replay and start " +
            "Minecraft again.",
    )
}
