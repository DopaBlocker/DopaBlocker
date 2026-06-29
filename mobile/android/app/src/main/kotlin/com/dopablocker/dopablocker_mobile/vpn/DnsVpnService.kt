package com.dopablocker.dopablocker_mobile.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import com.dopablocker.dopablocker_mobile.reporting.TamperReporter
import com.dopablocker.dopablocker_mobile.accessibility.AppBlockerService
import java.io.FileInputStream
import java.io.FileOutputStream
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/// VpnService que intercepta o tráfego DNS do dispositivo e bloqueia os
/// domínios da blocklist (sinkhole DNS-only).
///
/// Arquitetura (paridade com o desktop, ver dns_proxy.rs):
/// - A TUN roteia apenas o DNS virtual (10.0.0.1/32); o resto do tráfego segue
///   pela rede normal. O loop lê só pacotes DNS.
/// - Domínio bloqueado → resposta A=127.0.0.1 (TTL 5s) / AAAA vazio / NXDOMAIN.
/// - Domínio permitido → encaminhado a um upstream via socket protegido.
class DnsVpnService : VpnService() {

    private var tunInterface: ParcelFileDescriptor? = null
    private var worker: Thread? = null
    private val running = AtomicBoolean(false)
    private var pool: ExecutorService? = null
    private val writeLock = Any()
    private var output: FileOutputStream? = null

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

        // Carrega a blocklist persistida ANTES de aceitar queries — cobre o
        // restart pelo sistema e o boot (quando o Dart ainda não rodou).
        loadBlocklist(this)
        // Aplica o filtro adulto persistido (escolhe o upstream filtrado/normal).
        loadAdultFilter(this)

        // TUN virtual roteando só o DNS virtual (10.0.0.1) — sinkhole DNS-only.
        tunInterface = Builder()
            .setSession("DopaBlocker")
            .addAddress("10.0.0.2", 32)
            .addDnsServer("10.0.0.1")
            .addRoute("10.0.0.1", 32)
            .establish()

        if (tunInterface == null) {
            Log.e(TAG, "Falha ao estabelecer a TUN")
            stopVpn()
            return
        }

        running.set(true)
        setBlockingActive(this, true)
        VpnManager.setActive(true)

        pool = Executors.newFixedThreadPool(WORKER_THREADS)
        worker = Thread({ packetLoop() }, "dopablocker-vpn").apply { start() }
    }

    private fun packetLoop() {
        val fd = tunInterface?.fileDescriptor ?: return
        val input = FileInputStream(fd)
        output = FileOutputStream(fd)
        val buffer = ByteArray(MAX_PACKET)
        try {
            while (running.get()) {
                val n = try {
                    input.read(buffer)
                } catch (_: Exception) {
                    if (!running.get()) break else continue
                }
                if (n <= 0) continue
                val packet = buffer.copyOf(n)
                pool?.execute { handlePacket(packet) }
            }
        } finally {
            try { input.close() } catch (_: Exception) {}
        }
    }

    /// Processa um pacote DNS: bloqueia (responde localmente) ou encaminha.
    private fun handlePacket(packet: ByteArray) {
        val datagram = DnsPacket.parseIpv4Udp(packet) ?: return
        if (datagram.dstPort != 53) return
        val qName = DnsPacket.extractQName(datagram.payload) ?: return
        val qType = DnsPacket.extractQType(datagram.payload)

        if (DomainMatcher.isDomainBlocked(qName, blocklist)) {
            val dns = DnsPacket.buildBlockResponse(datagram.payload, qType)
            if (dns.isNotEmpty()) writePacket(DnsPacket.buildIpv4UdpResponse(packet, dns))
            Log.i(TAG, "BLOCK -> 127.0.0.1 $qName")
            // Overlay de site bloqueado (C1): best-effort, só se a acessibilidade
            // estiver ativa e um navegador em foco. Sem ela, o bloqueio DNS segue
            // normal — apenas não há tela.
            AppBlockerService.instance?.notifyBlockedDomain(qName)
        } else {
            val upstream = DnsForwarder.forward(datagram.payload) { protect(it) }
            val dns = upstream ?: DnsPacket.buildServfail(datagram.payload)
            if (dns.isNotEmpty()) writePacket(DnsPacket.buildIpv4UdpResponse(packet, dns))
        }
    }

    private fun writePacket(pkt: ByteArray) {
        synchronized(writeLock) {
            try {
                output?.write(pkt)
            } catch (_: Exception) {
                // TUN fechada (stop/revogação) — ignora.
            }
        }
    }

    private fun stopVpn() {
        running.set(false)
        setBlockingActive(this, false)
        worker?.interrupt()
        worker = null
        pool?.shutdownNow()
        pool = null
        synchronized(writeLock) {
            try { output?.close() } catch (_: Exception) {}
            output = null
        }
        try { tunInterface?.close() } catch (_: Exception) {}
        tunInterface = null
        VpnManager.setActive(false)
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    /// Chamado quando o usuário desliga a VPN nas Configurações ou outra VPN
    /// assume — limpa o estado e marca como inativo. Antes disso, REPORTA o
    /// evento ao backend (C2.1): no device do filho, o pai passa a ver que a
    /// proteção caiu (é o maior ganho anti-bypass do mobile sem root).
    override fun onRevoke() {
        TamperReporter.report(applicationContext, "vpn_revoked")
        stopVpn()
        super.onRevoke()
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
        private const val MAX_PACKET = 32767
        private const val WORKER_THREADS = 4

        /// SharedPreferences compartilhado com o BootReceiver.
        const val PREFS = "dopablocker_prefs"
        const val KEY_BLOCKING_ACTIVE = "blocking_active"
        const val KEY_BLOCKLIST = "blocklist"
        const val KEY_ADULT_FILTER = "adult_filter"

        /// Blocklist em runtime (já normalizada). `@Volatile`: lida no loop,
        /// escrita pelo MethodChannel — troca de referência é atômica.
        @Volatile
        var blocklist: Set<String> = emptySet()
            private set

        private fun prefs(context: Context) =
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

        /// Atualiza a blocklist (normaliza, persiste e aplica em runtime) sem
        /// reiniciar a VPN. A regra do pai imune é aplicada no Dart antes daqui
        /// (envia lista vazia no device do pai).
        fun updateBlocklist(context: Context, domains: List<String>) {
            val normalized = domains
                .map { DomainMatcher.normalizeDomain(it.trim()) }
                .filter { it.isNotBlank() }
                .toSet()
            blocklist = normalized
            prefs(context).edit().putStringSet(KEY_BLOCKLIST, normalized).apply()
        }

        /// Recarrega a blocklist persistida para o runtime (boot/restart).
        fun loadBlocklist(context: Context) {
            blocklist = prefs(context).getStringSet(KEY_BLOCKLIST, emptySet())?.toSet() ?: emptySet()
        }

        /// Liga/desliga o filtro adulto (C4): persiste a flag e troca o upstream
        /// do `DnsForwarder` para o resolver de família ou o padrão. Não exige
        /// reiniciar a VPN.
        fun setAdultFilter(context: Context, enabled: Boolean) {
            prefs(context).edit().putBoolean(KEY_ADULT_FILTER, enabled).apply()
            DnsForwarder.upstreams = DnsForwarder.upstreamsFor(enabled)
        }

        /// Recarrega a flag de filtro adulto e aplica o upstream (boot/restart).
        fun loadAdultFilter(context: Context) {
            val enabled = prefs(context).getBoolean(KEY_ADULT_FILTER, false)
            DnsForwarder.upstreams = DnsForwarder.upstreamsFor(enabled)
        }

        /// Persiste a flag lida pelo BootReceiver para religar após o boot.
        fun setBlockingActive(context: Context, active: Boolean) {
            prefs(context).edit().putBoolean(KEY_BLOCKING_ACTIVE, active).apply()
        }
    }
}
