package com.dopablocker.dopablocker_mobile.accessibility

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/// Verifica a heurística de detecção de adulteração (C2.2): identifica telas de
/// VPN/DNS das Configurações a partir do pacote + classe da janela.
class SettingsTamperDetectorTest {

    @Test
    fun detects_vpn_settings_screen() {
        assertEquals(
            "vpn_settings_opened",
            SettingsTamperDetector.kindFor("com.android.settings", "com.android.settings.vpn2.VpnSettings"),
        )
    }

    @Test
    fun detects_private_dns_screen() {
        assertEquals(
            "dns_settings_opened",
            SettingsTamperDetector.kindFor("com.android.settings", "com.android.settings.network.PrivateDnsSettings"),
        )
    }

    @Test
    fun ignores_non_settings_packages_and_unrelated_screens() {
        assertNull(SettingsTamperDetector.kindFor("com.instagram.android", "android.app.Activity"))
        assertNull(SettingsTamperDetector.kindFor("com.android.settings", "com.android.settings.DisplaySettings"))
    }
}
