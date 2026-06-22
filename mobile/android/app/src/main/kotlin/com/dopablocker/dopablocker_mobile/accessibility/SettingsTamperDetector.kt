package com.dopablocker.dopablocker_mobile.accessibility

/// Heurística pura (testável sem Android) para detectar, a partir do pacote e
/// da classe da janela em foco, que o usuário abriu uma tela de Configurações
/// de **VPN** ou **DNS privado** — os caminhos óbvios de adulteração no Android
/// sem root (C2.2).
///
/// Limitação assumida: como o AccessibilityService NÃO lê o conteúdo das telas
/// (`canRetrieveWindowContent=false`), só temos `packageName`/`className`. Os
/// nomes de classe variam entre fabricantes/versões, então isto é best-effort
/// (pode ter falsos negativos). Retorna o `kind` a reportar, ou `null`.
object SettingsTamperDetector {

    private const val SETTINGS_PACKAGE = "com.android.settings"

    fun kindFor(packageName: String, className: String): String? {
        if (packageName != SETTINGS_PACKAGE) return null
        val lower = className.lowercase()
        return when {
            lower.contains("privatedns") || lower.contains("private_dns") -> "dns_settings_opened"
            lower.contains("vpn") -> "vpn_settings_opened"
            // Telas de "Tethering/VPN" e dashboards de rede onde a VPN é desligada.
            lower.contains("tether") -> "vpn_settings_opened"
            else -> null
        }
    }
}
