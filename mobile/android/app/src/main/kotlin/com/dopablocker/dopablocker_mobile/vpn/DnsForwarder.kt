package com.dopablocker.dopablocker_mobile.vpn

import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress

/// Encaminha queries DNS permitidas a um upstream público via socket UDP
/// **protegido** (`protect`), que faz o tráfego sair pela rede real em vez de
/// voltar pela TUN — sem isso haveria um loop infinito de VPN.
///
/// Espelha o fallback UDP do desktop (dns_proxy.rs): tenta cada upstream com
/// timeout curto; se todos falharem, retorna `null` (o chamador responde
/// SERVFAIL).
object DnsForwarder {

    private val UPSTREAMS = listOf("1.1.1.1", "8.8.8.8")
    private const val TIMEOUT_MS = 2000

    /// Envia o payload DNS cru e devolve a resposta crua, ou `null` em falha.
    /// `protect` é `VpnService.protect` (recebe o socket, retorna sucesso).
    fun forward(query: ByteArray, protect: (DatagramSocket) -> Boolean): ByteArray? {
        for (server in UPSTREAMS) {
            try {
                DatagramSocket().use { socket ->
                    protect(socket)
                    socket.soTimeout = TIMEOUT_MS
                    val addr = InetAddress.getByName(server) // literal IP → sem lookup
                    socket.send(DatagramPacket(query, query.size, addr, 53))
                    val buf = ByteArray(4096)
                    val resp = DatagramPacket(buf, buf.size)
                    socket.receive(resp)
                    return buf.copyOf(resp.length)
                }
            } catch (_: Exception) {
                // timeout / erro de rede → tenta o próximo upstream
            }
        }
        return null
    }
}
