package com.dopablocker.dopablocker_mobile.vpn

import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import java.util.concurrent.atomic.AtomicBoolean

/// Gerencia o ciclo de vida da VPN. Encapsula prepare/start/stop para que o
/// MainActivity e o BootReceiver não precisem conhecer os detalhes do
/// DnsVpnService.
object VpnManager {

    private val active = AtomicBoolean(false)

    /// Retorna um Intent de consentimento se a VPN ainda não foi autorizada,
    /// ou null se já está autorizada.
    fun prepare(context: Context): Intent? = VpnService.prepare(context)

    fun start(context: Context) {
        val intent = Intent(context, DnsVpnService::class.java).apply {
            action = DnsVpnService.ACTION_START
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent)
        } else {
            context.startService(intent)
        }
        active.set(true)
    }

    fun stop(context: Context) {
        val intent = Intent(context, DnsVpnService::class.java).apply {
            action = DnsVpnService.ACTION_STOP
        }
        context.startService(intent)
        active.set(false)
    }

    fun isActive(): Boolean = active.get()

    /// Atualizado pelo próprio DnsVpnService quando o serviço sobe/desce.
    internal fun setActive(value: Boolean) = active.set(value)
}
