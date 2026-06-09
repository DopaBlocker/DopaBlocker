package com.dopablocker.dopablocker_mobile.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import java.util.concurrent.atomic.AtomicBoolean

/// VpnService que intercepta o tráfego DNS do dispositivo.
///
/// ESQUELETO (Fase M1): interface TUN, foreground service, notificação
/// persistente e ciclo de vida estão prontos. O loop de leitura/parse de
/// pacotes DNS e o encaminhamento ao upstream são TODO da Fase M2 — ver
/// docs/DEVELOPMENT_GUIDE.md § "Fase M2 — vpn/DnsVpnService.kt".
class DnsVpnService : VpnService() {

    private var tunInterface: ParcelFileDescriptor? = null
    private var worker: Thread? = null
    private val running = AtomicBoolean(false)

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                stopVpn()
                return START_NOT_STICKY
            }
            else -> startVpn()
        }
        return START_STICKY
    }

    private fun startVpn() {
        if (running.get()) return
        startForeground(NOTIFICATION_ID, buildNotification())

        // Estabelece a interface TUN virtual que captura todo o tráfego IPv4.
        // O DNS da VPN (10.0.0.1) é o ponto onde as queries serão filtradas.
        tunInterface = Builder()
            .setSession("DopaBlocker")
            .addAddress("10.0.0.2", 32)
            .addDnsServer("10.0.0.1")
            .addRoute("0.0.0.0", 0)
            .establish()

        running.set(true)
        VpnManager.setActive(true)

        worker = Thread({ packetLoop() }, "dopablocker-vpn").apply { start() }
    }

    private fun packetLoop() {
        // TODO (Fase M2): ler pacotes IP da interface TUN em loop enquanto
        // running.get(); identificar pacotes DNS (porta 53); parsear o QNAME;
        // normalizar e checar contra `blocklist`. Se bloqueado → responder
        // 0.0.0.0 / NXDOMAIN. Se permitido → encaminhar ao upstream (ex:
        // 8.8.8.8:53) por um socket protegido com protect(socket) para evitar
        // loop, e devolver a resposta na interface TUN.
        Log.i(TAG, "packetLoop ativo — filtragem DNS pendente (Fase M2)")
    }

    private fun stopVpn() {
        running.set(false)
        worker?.interrupt()
        worker = null
        tunInterface?.close()
        tunInterface = null
        VpnManager.setActive(false)
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        stopVpn()
        super.onDestroy()
    }

    private fun buildNotification(): Notification {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Bloqueio DopaBlocker",
                NotificationManager.IMPORTANCE_LOW
            )
            getSystemService(NotificationManager::class.java)
                ?.createNotificationChannel(channel)
        }
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("DopaBlocker ativo")
            .setContentText("Bloqueio de distrações em execução")
            .setSmallIcon(android.R.drawable.ic_lock_idle_lock)
            .setOngoing(true)
            .build()
    }

    companion object {
        private const val TAG = "DnsVpnService"
        const val ACTION_START = "com.dopablocker.vpn.START"
        const val ACTION_STOP = "com.dopablocker.vpn.STOP"
        private const val CHANNEL_ID = "dopablocker_blocking"
        private const val NOTIFICATION_ID = 1001

        /// Blocklist compartilhada — atualizada via MethodChannel sem reiniciar
        /// a VPN. A regra do pai imune é aplicada no lado Dart antes de chamar
        /// updateBlocklist (envia lista vazia para o device do pai).
        @Volatile
        var blocklist: List<String> = emptyList()
            private set

        fun updateBlocklist(domains: List<String>) {
            blocklist = domains
        }
    }
}
