package com.dopablocker.dopablocker_mobile.vpn

import org.junit.Assert.assertEquals
import org.junit.Test

/// Verifica a seleção de upstream do filtro adulto (C4): ligado → resolver de
/// família (Cloudflare for Families); desligado → resolvers padrão.
class DnsForwarderTest {

    @Test
    fun adult_filter_on_uses_family_resolver() {
        assertEquals(DnsForwarder.FAMILY_UPSTREAMS, DnsForwarder.upstreamsFor(true))
        assertEquals(listOf("1.1.1.3", "1.0.0.3"), DnsForwarder.upstreamsFor(true))
    }

    @Test
    fun adult_filter_off_uses_default_resolver() {
        assertEquals(DnsForwarder.DEFAULT_UPSTREAMS, DnsForwarder.upstreamsFor(false))
        assertEquals(listOf("1.1.1.1", "8.8.8.8"), DnsForwarder.upstreamsFor(false))
    }
}
