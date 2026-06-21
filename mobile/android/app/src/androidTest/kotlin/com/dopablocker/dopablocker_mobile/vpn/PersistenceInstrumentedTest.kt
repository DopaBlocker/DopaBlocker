package com.dopablocker.dopablocker_mobile.vpn

import android.content.Context
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/// Verificação on-device (Fase B) da persistência do bloqueio em
/// SharedPreferences. Não depende de Firebase nem de consentimento de VPN.
@RunWith(AndroidJUnit4::class)
class PersistenceInstrumentedTest {

    private val ctx: Context = InstrumentationRegistry.getInstrumentation().targetContext

    private fun prefs() = ctx.getSharedPreferences(DnsVpnService.PREFS, Context.MODE_PRIVATE)

    @Before
    fun clearPrefs() {
        prefs().edit().clear().commit()
    }

    @Test
    fun updateBlocklist_normalizesAndPersistsToDisk() {
        DnsVpnService.updateBlocklist(
            ctx,
            listOf("https://www.Instagram.com/", "youtube.com", "   "),
        )
        val expected = setOf("instagram.com", "youtube.com")
        assertEquals("runtime normalizado", expected, DnsVpnService.blocklist)
        assertEquals("persistido em disco", expected, prefs().getStringSet(DnsVpnService.KEY_BLOCKLIST, null))
    }

    @Test
    fun loadBlocklist_readsFromDisk() {
        // Simula o que o boot/restart encontra no disco.
        prefs().edit().putStringSet(DnsVpnService.KEY_BLOCKLIST, setOf("reddit.com")).commit()
        DnsVpnService.loadBlocklist(ctx)
        assertEquals(setOf("reddit.com"), DnsVpnService.blocklist)
    }

    @Test
    fun setBlockingActive_persistsFlagForBootReceiver() {
        DnsVpnService.setBlockingActive(ctx, true)
        assertTrue(prefs().getBoolean(DnsVpnService.KEY_BLOCKING_ACTIVE, false))
        DnsVpnService.setBlockingActive(ctx, false)
        assertFalse(prefs().getBoolean(DnsVpnService.KEY_BLOCKING_ACTIVE, true))
    }
}
