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

    /// Resolvers padrão (sem filtro de conteúdo).
    val DEFAULT_UPSTREAMS = listOf("1.1.1.1", "8.8.8.8")

    /// Resolver de família — Cloudflare for Families (bloqueia malware + adulto).
    /// É a base do filtro adulto mobile (C4): com o toggle ligado, trocamos o
    /// upstream por estes IPs e a cobertura adulta vem sempre atualizada do
    /// resolver, sem manter ~100k domínios no device.
    val FAMILY_UPSTREAMS = listOf("1.1.1.3", "1.0.0.3")

    private const val TIMEOUT_MS = 2000

    /// Upstreams ativos. `@Volatile`: lido no loop de pacotes, trocado pelo
    /// MethodChannel (toggle de filtro adulto) — troca de referência é atômica.
    @Volatile
    var upstreams: List<String> = DEFAULT_UPSTREAMS

    /// Seleciona os upstreams conforme o filtro adulto. Pura/determinística
    /// para ser testável sem Android.
    fun upstreamsFor(adultFilterEnabled: Boolean): List<String> =
        if (adultFilterEnabled) FAMILY_UPSTREAMS else DEFAULT_UPSTREAMS

    /// Envia o payload DNS cru e devolve a resposta crua, ou `null` em falha.
    /// `protect` é `VpnService.protect` (recebe o socket, retorna sucesso).
    fun forward(query: ByteArray, protect: (DatagramSocket) -> Boolean): ByteArray? {
        for (server in upstreams) {
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
