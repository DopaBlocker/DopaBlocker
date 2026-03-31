// VpnService para bloqueio DNS — intercepta tráfego de rede no Android.
// Implementar: estender android.net.VpnService, configurar TUN interface,
// interceptar pacotes DNS, verificar domínio contra blocklist,
// se bloqueado retornar NXDOMAIN, senão encaminhar ao DNS upstream.
// Rodar como foreground service com notificação persistente.

package com.dopablocker.dopablocker_mobile.vpn
