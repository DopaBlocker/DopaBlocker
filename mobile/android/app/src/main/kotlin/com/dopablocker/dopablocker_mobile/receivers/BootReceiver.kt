package com.dopablocker.dopablocker_mobile.receivers

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import com.dopablocker.dopablocker_mobile.vpn.DnsVpnService
import com.dopablocker.dopablocker_mobile.vpn.VpnManager

/// Reinicia o bloqueio após o boot do dispositivo, se estava ativo antes.
///
/// A flag e a blocklist são persistidas pelo DnsVpnService (start/stop). Aqui
/// só lemos a flag e subimos a VPN — o próprio startVpn() recarrega a blocklist
/// do SharedPreferences.
class BootReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return

        val prefs = context.getSharedPreferences(DnsVpnService.PREFS, Context.MODE_PRIVATE)
        if (prefs.getBoolean(DnsVpnService.KEY_BLOCKING_ACTIVE, false)) {
            VpnManager.start(context)
        }
    }
}
