package com.dopablocker.dopablocker_mobile.vpn

/// Datagrama UDP extraído de um pacote IPv4 lido da interface TUN.
data class UdpDatagram(
    val srcAddr: ByteArray,
    val dstAddr: ByteArray,
    val srcPort: Int,
    val dstPort: Int,
    val payload: ByteArray,
)

/// Parsing/encoding mínimo de IPv4 + UDP + DNS, em Kotlin puro (testável na
/// JVM, sem dependências do Android). Replica a semântica de bloqueio do
/// desktop (desktop/src-tauri/src/blocking/dns_proxy.rs::build_block_redirect):
/// A → 127.0.0.1 (TTL 5s), AAAA → answer vazio, demais → NXDOMAIN.
object DnsPacket {

    const val TYPE_A = 1
    const val TYPE_AAAA = 28
    const val BLOCK_TTL = 5

    /// Extrai o datagrama UDP de um pacote IPv4. Retorna `null` se não for
    /// IPv4/UDP ou se o pacote for curto/malformado.
    fun parseIpv4Udp(packet: ByteArray): UdpDatagram? {
        if (packet.size < 20) return null
        if ((packet[0].toInt() and 0xF0) ushr 4 != 4) return null      // versão IPv4
        val ihl = (packet[0].toInt() and 0x0F) * 4
        if (ihl < 20 || packet.size < ihl + 8) return null
        if (packet[9].toInt() and 0xFF != 17) return null              // protocolo UDP
        val srcAddr = packet.copyOfRange(12, 16)
        val dstAddr = packet.copyOfRange(16, 20)
        val srcPort = u16(packet, ihl)
        val dstPort = u16(packet, ihl + 2)
        val udpLen = u16(packet, ihl + 4)
        val payloadStart = ihl + 8
        val payloadLen = (udpLen - 8).coerceAtLeast(0)
        val end = (payloadStart + payloadLen).coerceAtMost(packet.size)
        return UdpDatagram(srcAddr, dstAddr, srcPort, dstPort, packet.copyOfRange(payloadStart, end))
    }

    /// Extrai o QNAME da primeira question (labels length-prefixed). A pergunta
    /// nunca usa ponteiros de compressão — se encontrar um, trata como inválido.
    fun extractQName(dnsPayload: ByteArray): String? {
        if (dnsPayload.size < 12) return null
        var i = 12
        val sb = StringBuilder()
        while (i < dnsPayload.size) {
            val len = dnsPayload[i].toInt() and 0xFF
            if (len == 0) break
            if (len and 0xC0 != 0) return null
            i++
            if (i + len > dnsPayload.size) return null
            if (sb.isNotEmpty()) sb.append('.')
            sb.append(String(dnsPayload, i, len, Charsets.US_ASCII))
            i += len
        }
        return if (sb.isEmpty()) null else sb.toString()
    }

    /// QTYPE da primeira question. Retorna -1 se malformado.
    fun extractQType(dnsPayload: ByteArray): Int {
        val end = qnameEnd(dnsPayload) ?: return -1
        if (end + 2 > dnsPayload.size) return -1
        return u16(dnsPayload, end)
    }

    /// Monta a resposta DNS de bloqueio (mesma semântica do desktop):
    /// A → answer 127.0.0.1 TTL 5s; AAAA → NoError sem answer; demais → NXDOMAIN.
    fun buildBlockResponse(dnsQuery: ByteArray, qType: Int): ByteArray {
        val qEnd = qnameEnd(dnsQuery) ?: return ByteArray(0)
        val questionEnd = qEnd + 4 // + QTYPE + QCLASS
        if (questionEnd > dnsQuery.size) return ByteArray(0)
        val addAnswer = qType == TYPE_A
        val nxdomain = qType != TYPE_A && qType != TYPE_AAAA

        val resp = ByteArray(questionEnd + if (addAnswer) 16 else 0)
        System.arraycopy(dnsQuery, 0, resp, 0, questionEnd) // header + question
        val rd = dnsQuery[2].toInt() and 0x01
        resp[2] = (0x80 or rd).toByte()              // QR=1, Opcode=0, RD copiado
        resp[3] = (0x80 or if (nxdomain) 3 else 0).toByte() // RA=1, RCODE
        resp[4] = 0x00; resp[5] = 0x01               // QDCOUNT=1
        val ancount = if (addAnswer) 1 else 0
        resp[6] = (ancount ushr 8).toByte(); resp[7] = ancount.toByte()
        resp[8] = 0x00; resp[9] = 0x00               // NSCOUNT=0
        resp[10] = 0x00; resp[11] = 0x00             // ARCOUNT=0
        if (addAnswer) {
            var o = questionEnd
            resp[o++] = 0xC0.toByte(); resp[o++] = 0x0C.toByte()  // ponteiro p/ QNAME @ offset 12
            resp[o++] = 0x00; resp[o++] = TYPE_A.toByte()         // TYPE=A
            resp[o++] = 0x00; resp[o++] = 0x01                    // CLASS=IN
            resp[o++] = 0x00; resp[o++] = 0x00
            resp[o++] = (BLOCK_TTL ushr 8 and 0xFF).toByte(); resp[o++] = (BLOCK_TTL and 0xFF).toByte() // TTL
            resp[o++] = 0x00; resp[o++] = 0x04                    // RDLENGTH=4
            resp[o++] = 127; resp[o++] = 0; resp[o++] = 0; resp[o] = 1 // RDATA=127.0.0.1
        }
        return resp
    }

    /// Resposta SERVFAIL (RCODE=2, sem answer) — usada quando o upstream falha.
    fun buildServfail(dnsQuery: ByteArray): ByteArray {
        val qEnd = qnameEnd(dnsQuery) ?: return ByteArray(0)
        val questionEnd = qEnd + 4
        if (questionEnd > dnsQuery.size) return ByteArray(0)
        val resp = ByteArray(questionEnd)
        System.arraycopy(dnsQuery, 0, resp, 0, questionEnd)
        resp[2] = (0x80 or (dnsQuery[2].toInt() and 0x01)).toByte() // QR=1, RD copiado
        resp[3] = (0x80 or 2).toByte()                              // RA=1, RCODE=2 (SERVFAIL)
        resp[4] = 0x00; resp[5] = 0x01                              // QDCOUNT=1
        resp[6] = 0x00; resp[7] = 0x00                              // ANCOUNT=0
        resp[8] = 0x00; resp[9] = 0x00                              // NSCOUNT=0
        resp[10] = 0x00; resp[11] = 0x00                           // ARCOUNT=0
        return resp
    }

    /// Monta o pacote IPv4+UDP de resposta a partir do pedido original: troca
    /// origem/destino e portas, anexa o payload DNS e recalcula os checksums.
    fun buildIpv4UdpResponse(originalRequest: ByteArray, dnsResponsePayload: ByteArray): ByteArray {
        val ihl = (originalRequest[0].toInt() and 0x0F) * 4
        val origSrcPort = u16(originalRequest, ihl)
        val origDstPort = u16(originalRequest, ihl + 2)

        val udpLen = 8 + dnsResponsePayload.size
        val totalLen = 20 + udpLen
        val out = ByteArray(totalLen)
        // IPv4 (20 bytes, sem opções)
        out[0] = 0x45.toByte()
        out[2] = (totalLen ushr 8).toByte(); out[3] = totalLen.toByte()
        out[8] = 64                                  // TTL
        out[9] = 17                                  // protocolo UDP
        System.arraycopy(originalRequest, 16, out, 12, 4) // src = antigo dst
        System.arraycopy(originalRequest, 12, out, 16, 4) // dst = antigo src
        val ipChk = onesComplement(out, 0, 20)
        out[10] = (ipChk ushr 8).toByte(); out[11] = ipChk.toByte()
        // UDP
        out[20] = (origDstPort ushr 8).toByte(); out[21] = origDstPort.toByte() // src port = 53
        out[22] = (origSrcPort ushr 8).toByte(); out[23] = origSrcPort.toByte() // dst port = cliente
        out[24] = (udpLen ushr 8).toByte(); out[25] = udpLen.toByte()
        System.arraycopy(dnsResponsePayload, 0, out, 28, dnsResponsePayload.size)
        val udpChk = udpChecksum(out, udpLen)
        out[26] = (udpChk ushr 8).toByte(); out[27] = udpChk.toByte()
        return out
    }

    // ── Helpers internos ─────────────────────────────────────────────────────

    private fun u16(b: ByteArray, off: Int): Int =
        ((b[off].toInt() and 0xFF) shl 8) or (b[off + 1].toInt() and 0xFF)

    /// Offset logo após o byte terminador (0) do QNAME. `null` se malformado.
    private fun qnameEnd(payload: ByteArray): Int? {
        if (payload.size < 12) return null
        var i = 12
        while (i < payload.size) {
            val len = payload[i].toInt() and 0xFF
            if (len == 0) return i + 1
            if (len and 0xC0 != 0) return null
            i += len + 1
        }
        return null
    }

    /// Checksum da Internet (one's complement) já invertido, pronto p/ inserir.
    private fun onesComplement(b: ByteArray, start: Int, len: Int): Int {
        var sum = 0
        var i = start
        val end = start + len
        while (i + 1 < end) { sum += u16(b, i); i += 2 }
        if (i < end) sum += (b[i].toInt() and 0xFF) shl 8
        while (sum ushr 16 != 0) sum = (sum and 0xFFFF) + (sum ushr 16)
        return sum.inv() and 0xFFFF
    }

    /// Checksum UDP com pseudo-header IPv4 (src/dst já estão em `out`).
    private fun udpChecksum(out: ByteArray, udpLen: Int): Int {
        var sum = 0
        // pseudo-header: src(12..15) + dst(16..19) + zero/proto + udpLen
        sum += u16(out, 12) + u16(out, 14) + u16(out, 16) + u16(out, 18)
        sum += 17 + udpLen
        var i = 20
        val end = 20 + udpLen
        while (i + 1 < end) { sum += u16(out, i); i += 2 }
        if (i < end) sum += (out[i].toInt() and 0xFF) shl 8
        while (sum ushr 16 != 0) sum = (sum and 0xFFFF) + (sum ushr 16)
        val result = sum.inv() and 0xFFFF
        return if (result == 0) 0xFFFF else result // 0 significaria "sem checksum"
    }
}
