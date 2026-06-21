package com.dopablocker.dopablocker_mobile.vpn

import android.content.Context
import android.net.VpnService
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.FileInputStream
import java.net.InetAddress

/// Verificação E2E real (Fases C+D) no emulador: sobe a VPN de fato e checa que
/// um domínio bloqueado resolve para 127.0.0.1 e um permitido resolve normal.
///
/// O consentimento de VPN (normalmente um diálogo) é concedido via shell da
/// instrumentação — assim não depende do login Firebase para chegar ao toggle.
@RunWith(AndroidJUnit4::class)
class DnsBlockingInstrumentedTest {

    private val ctx: Context = InstrumentationRegistry.getInstrumentation().targetContext

    private fun shell(cmd: String) {
        val pfd = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(cmd)
        FileInputStream(pfd.fileDescriptor).use { it.readBytes() }
    }

    @After
    fun stopVpn() {
        VpnManager.stop(ctx)
        Thread.sleep(500)
    }

    @Test
    fun blockedResolvesToLoopback_allowedResolvesReal() {
        // Concede o consentimento de VPN sem o diálogo (uid shell).
        shell("appops set ${ctx.packageName} ACTIVATE_VPN allow")
        Thread.sleep(300)
        assertNull("consentimento de VPN não concedido", VpnService.prepare(ctx))

        DnsVpnService.updateBlocklist(ctx, listOf("instagram.com"))
        VpnManager.start(ctx)

        var waited = 0
        while (!VpnManager.isActive() && waited < 8000) { Thread.sleep(100); waited += 100 }
        assertTrue("VPN não ativou", VpnManager.isActive())
        Thread.sleep(1500) // deixa a TUN estabilizar

        val blocked = InetAddress.getByName("instagram.com")
        assertEquals("domínio bloqueado deve resolver p/ loopback", "127.0.0.1", blocked.hostAddress)

        val allowed = InetAddress.getByName("example.com")
        assertNotEquals("domínio permitido não deve ser bloqueado", "127.0.0.1", allowed.hostAddress)
    }
}
