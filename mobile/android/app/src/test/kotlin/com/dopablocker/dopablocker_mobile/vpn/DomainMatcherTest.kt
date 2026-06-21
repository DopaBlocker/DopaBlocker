package com.dopablocker.dopablocker_mobile.vpn

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/// Espelha os testes de shared/src/domain_matcher.rs — a semântica de
/// normalização e matching no Android deve ser idêntica à do desktop.
class DomainMatcherTest {

    @Test
    fun normalize_strips_protocol_www_and_path() {
        assertEquals("youtube.com", DomainMatcher.normalizeDomain("https://www.YouTube.com/watch?v=123"))
        assertEquals("facebook.com", DomainMatcher.normalizeDomain("http://facebook.com/"))
        assertEquals("instagram.com", DomainMatcher.normalizeDomain("HTTP://Instagram.COM"))
    }

    @Test
    fun normalize_preserves_subdomain() {
        assertEquals("sub.example.com", DomainMatcher.normalizeDomain("sub.example.com"))
        assertEquals("m.youtube.com", DomainMatcher.normalizeDomain("m.youtube.com"))
    }

    @Test
    fun normalize_idempotent_on_clean_domain() {
        assertEquals("reddit.com", DomainMatcher.normalizeDomain("reddit.com"))
    }

    @Test
    fun is_blocked_matches_exact() {
        val list = listOf("youtube.com")
        assertTrue(DomainMatcher.isDomainBlocked("youtube.com", list))
        assertTrue(DomainMatcher.isDomainBlocked("https://www.youtube.com/", list))
    }

    @Test
    fun is_blocked_matches_subdomain() {
        val list = listOf("youtube.com")
        assertTrue(DomainMatcher.isDomainBlocked("m.youtube.com", list))
        assertTrue(DomainMatcher.isDomainBlocked("music.youtube.com", list))
    }

    @Test
    fun is_blocked_rejects_similar_domain() {
        // `notyoutube.com` termina com `.com`, não com `.youtube.com`.
        val list = listOf("youtube.com")
        assertFalse(DomainMatcher.isDomainBlocked("notyoutube.com", list))
        assertFalse(DomainMatcher.isDomainBlocked("myyoutube.com", list))
    }

    @Test
    fun is_blocked_empty_list_never_blocks() {
        assertFalse(DomainMatcher.isDomainBlocked("anything.com", emptyList()))
    }
}
