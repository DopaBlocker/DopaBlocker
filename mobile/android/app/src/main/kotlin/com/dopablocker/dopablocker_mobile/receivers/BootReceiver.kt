// BroadcastReceiver para reiniciar VPN no boot do dispositivo.
// Implementar: estender BroadcastReceiver, escutar BOOT_COMPLETED intent,
// verificar se o bloqueio estava ativo antes do reboot (SharedPreferences),
// se sim, reiniciar DnsVpnService automaticamente.
// Registrar no AndroidManifest.xml com intent-filter BOOT_COMPLETED.

package com.dopablocker.dopablocker_mobile.receivers
