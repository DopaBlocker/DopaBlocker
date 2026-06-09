package com.dopablocker.dopablocker_mobile.receivers

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import com.dopablocker.dopablocker_mobile.vpn.VpnManager

/// Reinicia o bloqueio após o boot do dispositivo, se estava ativo antes.
///
/// ESQUELETO (Fase M1): lê uma flag de SharedPreferences. A gravação dessa
/// flag (quando a VPN liga/desliga) deve ser feita no fluxo de toggle da Fase
/// M2 — ver KEY_BLOCKING_ACTIVE.
class BootReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return

        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        if (prefs.getBoolean(KEY_BLOCKING_ACTIVE, false)) {
            VpnManager.start(context)
        }
    }

    companion object {
        const val PREFS = "dopablocker_prefs"
        const val KEY_BLOCKING_ACTIVE = "blocking_active"
    }
}
