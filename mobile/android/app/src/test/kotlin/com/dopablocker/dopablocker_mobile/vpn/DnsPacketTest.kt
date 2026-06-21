package com.dopablocker.dopablocker_mobile.vpn

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/// Testes do parser/encoder DNS + IPv4/UDP. Usa vetores de bytes construídos à
/// mão (query DNS válida) para validar parsing, respostas de bloqueio e
/// checksums sem depender de captura ou de device.
class DnsPacketTest {

    // ── Helpers de construção ────────────────────────────────────────────────

    /// Monta um payload de query DNS (header + 1 question).
    private fun dnsQuery(id: Int, qType: Int, vararg labels: String): ByteArray {
        val out = ArrayList<Byte>()
        out.add((id ushr 8).toByte()); out.add(id.toByte())
        out.add(0x01); out.add(0x00)            // flags: RD=1
        out.add(0x00); out.add(0x01)            // QDCOUNT=1
        out.add(0x00); out.add(0x00)            // ANCOUNT=0
        out.add(0x00); out.add(0x00)            // NSCOUNT=0
        out.add(0x00); out.add(0x00)            // ARCOUNT=0
        for (l in labels) {
            val b = l.toByteArray(Charsets.US_ASCII)
            out.add(b.size.toByte())
            b.forEach { out.add(it) }
        }
        out.add(0x00)                            // terminador do QNAME
        out.add((qType ushr 8).toByte()); out.add(qType.toByte()) // QTYPE
        out.add(0x00); out.add(0x01)            // QCLASS=IN
        return out.toByteArray()
    }

    /// Envolve um payload em um pacote IPv4+UDP (sem opções IP; checksums = 0).
    private fun ipv4Udp(
        srcIp: String, dstIp: String, srcPort: Int, dstPort: Int,
        payload: ByteArray, protocol: Int = 17,
    ): ByteArray {
        val udpLen = 8 + payload.size
        val totalLen = 20 + udpLen
        val pkt = ByteArray(totalLen)
        pkt[0] = 0x45.toByte()                   // versão 4, IHL=5
        pkt[2] = (totalLen ushr 8).toByte(); pkt[3] = totalLen.toByte()
        pkt[8] = 64                              // TTL
        pkt[9] = protocol.toByte()
        srcIp.split(".").forEachIndexed { i, o -> pkt[12 + i] = o.toInt().toByte() }
        dstIp.split(".").forEachIndexed { i, o -> pkt[16 + i] = o.toInt().toByte() }
        pkt[20] = (srcPort ushr 8).toByte(); pkt[21] = srcPort.toByte()
        pkt[22] = (dstPort ushr 8).toByte(); pkt[23] = dstPort.toByte()
        pkt[24] = (udpLen ushr 8).toByte(); pkt[25] = udpLen.toByte()
        System.arraycopy(payload, 0, pkt, 28, payload.size)
        return pkt
    }

    private fun u16(b: ByteArray, off: Int): Int =
        ((b[off].toInt() and 0xFF) shl 8) or (b[off + 1].toInt() and 0xFF)

    /// Offset logo após a question (header 12 + QNAME + QTYPE/QCLASS).
    private fun questionEnd(payload: ByteArray): Int {
        var i = 12
        while (payload[i].toInt() != 0) i += (payload[i].toInt() and 0xFF) + 1
        return i + 1 + 4
    }

    /// Soma de verificação de Internet (one's complement) sobre todo o array.
    /// Para um header válido (com seu checksum), o resultado deve ser 0xFFFF.
    private fun onesComplementSum(b: ByteArray, start: Int, len: Int): Int {
        var sum = 0
        var i = start
        val end = start + len
        while (i + 1 < end) { sum += u16(b, i); i += 2 }
        if (i < end) sum += (b[i].toInt() and 0xFF) shl 8
        while (sum ushr 16 != 0) sum = (sum and 0xFFFF) + (sum ushr 16)
        return sum and 0xFFFF
    }

    // ── extractQName / extractQType ──────────────────────────────────────────

    @Test
    fun extractQName_parsesLabels() {
        val q = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        assertEquals("instagram.com", DnsPacket.extractQName(q))
    }

    @Test
    fun extractQType_returnsA() {
        val q = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        assertEquals(DnsPacket.TYPE_A, DnsPacket.extractQType(q))
    }

    @Test
    fun extractQType_returnsAaaa() {
        val q = dnsQuery(0x1234, DnsPacket.TYPE_AAAA, "instagram", "com")
        assertEquals(DnsPacket.TYPE_AAAA, DnsPacket.extractQType(q))
    }

    // ── buildBlockResponse ───────────────────────────────────────────────────

    @Test
    fun buildBlockResponse_typeA_returns127001() {
        val q = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        val r = DnsPacket.buildBlockResponse(q, DnsPacket.TYPE_A)
        assertEquals("mesmo ID", 0x1234, u16(r, 0))
        assertTrue("QR setado", (r[2].toInt() and 0x80) != 0)
        assertEquals("RCODE NoError", 0, r[3].toInt() and 0x0F)
        assertEquals("ANCOUNT=1", 1, u16(r, 6))
        val ans = questionEnd(q)
        assertEquals("ponteiro de compressão 0xC0", 0xC0, r[ans].toInt() and 0xFF)
        assertEquals("ponteiro aponta p/ offset 12", 0x0C, r[ans + 1].toInt() and 0xFF)
        assertEquals("TYPE A", DnsPacket.TYPE_A, u16(r, ans + 2))
        assertEquals("TTL=5", 5L, ((u16(r, ans + 6).toLong() shl 16) or u16(r, ans + 8).toLong()))
        assertEquals("RDLENGTH=4", 4, u16(r, ans + 10))
        assertArrayEquals(
            "RDATA = 127.0.0.1",
            byteArrayOf(127, 0, 0, 1),
            r.copyOfRange(ans + 12, ans + 16),
        )
    }

    @Test
    fun buildBlockResponse_typeAaaa_noAnswer() {
        val q = dnsQuery(0x1234, DnsPacket.TYPE_AAAA, "instagram", "com")
        val r = DnsPacket.buildBlockResponse(q, DnsPacket.TYPE_AAAA)
        assertTrue("QR setado", (r[2].toInt() and 0x80) != 0)
        assertEquals("RCODE NoError", 0, r[3].toInt() and 0x0F)
        assertEquals("ANCOUNT=0", 0, u16(r, 6))
    }

    @Test
    fun buildBlockResponse_otherType_nxdomain() {
        val mx = 15
        val q = dnsQuery(0x1234, mx, "instagram", "com")
        val r = DnsPacket.buildBlockResponse(q, mx)
        assertEquals("RCODE NXDOMAIN", 3, r[3].toInt() and 0x0F)
        assertEquals("ANCOUNT=0", 0, u16(r, 6))
    }

    @Test
    fun buildServfail_setsRcode2_noAnswer() {
        val q = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        val r = DnsPacket.buildServfail(q)
        assertEquals("mesmo ID", 0x1234, u16(r, 0))
        assertTrue("QR setado", (r[2].toInt() and 0x80) != 0)
        assertEquals("RCODE SERVFAIL", 2, r[3].toInt() and 0x0F)
        assertEquals("ANCOUNT=0", 0, u16(r, 6))
        assertEquals("QDCOUNT=1", 1, u16(r, 4))
    }

    // ── parseIpv4Udp ─────────────────────────────────────────────────────────

    @Test
    fun parseIpv4Udp_extractsPortsAndPayload() {
        val dns = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        val pkt = ipv4Udp("10.0.0.2", "10.0.0.1", 54321, 53, dns)
        val d = DnsPacket.parseIpv4Udp(pkt)
        assertNotNull(d)
        assertEquals(53, d!!.dstPort)
        assertEquals(54321, d.srcPort)
        assertArrayEquals(byteArrayOf(10, 0, 0, 2), d.srcAddr)
        assertArrayEquals(byteArrayOf(10, 0, 0, 1), d.dstAddr)
        assertArrayEquals(dns, d.payload)
    }

    @Test
    fun parseIpv4Udp_rejectsNonUdp() {
        val dns = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        val tcp = ipv4Udp("10.0.0.2", "10.0.0.1", 54321, 53, dns, protocol = 6) // TCP
        assertNull(DnsPacket.parseIpv4Udp(tcp))
    }

    // ── buildIpv4UdpResponse ─────────────────────────────────────────────────

    @Test
    fun buildIpv4UdpResponse_swapsAddrsPortsAndChecksumsValid() {
        val dns = dnsQuery(0x1234, DnsPacket.TYPE_A, "instagram", "com")
        val req = ipv4Udp("10.0.0.2", "10.0.0.1", 54321, 53, dns)
        val respPayload = DnsPacket.buildBlockResponse(dns, DnsPacket.TYPE_A)
        val out = DnsPacket.buildIpv4UdpResponse(req, respPayload)

        // IPv4: versão 4 / IHL 5, protocolo UDP, endereços trocados
        assertEquals(0x45, out[0].toInt() and 0xFF)
        assertEquals(17, out[9].toInt() and 0xFF)
        assertArrayEquals("src = antigo dst", byteArrayOf(10, 0, 0, 1), out.copyOfRange(12, 16))
        assertArrayEquals("dst = antigo src", byteArrayOf(10, 0, 0, 2), out.copyOfRange(16, 20))
        // UDP: porta de origem agora é 53, destino é a efêmera do cliente
        assertEquals(53, u16(out, 20))
        assertEquals(54321, u16(out, 22))
        // Comprimentos coerentes
        assertEquals("IP total length", out.size, u16(out, 2))
        assertEquals("UDP length", out.size - 20, u16(out, 24))
        // Payload DNS preservado
        assertArrayEquals(respPayload, out.copyOfRange(28, out.size))
        // Checksums válidos (invariante: soma com o próprio checksum = 0xFFFF)
        assertEquals("IPv4 checksum válido", 0xFFFF, onesComplementSum(out, 0, 20))
    }
}
