package com.dopablocker.dopablocker_mobile.channel

/// Nome do MethodChannel de bloqueio + nomes dos métodos. Fonte única no lado
/// nativo — devem casar **exatamente** com o lado Dart
/// (`lib/core/channels/blocking_methods.dart`); um nome divergente faz a chamada
/// cair em `result.notImplemented()`.
object BlockingMethods {
    const val CHANNEL = "com.dopablocker/blocking"

    const val START_VPN = "startVpn"
    const val STOP_VPN = "stopVpn"
    const val IS_VPN_ACTIVE = "isVpnActive"
    const val IS_VPN_PREPARED = "isVpnPrepared"
    const val UPDATE_BLOCKLIST = "updateBlocklist"
    const val UPDATE_BLOCKED_APPS = "updateBlockedApps"
    const val SET_ADULT_FILTER = "setAdultFilter"
    const val SET_TAMPER_CONFIG = "setTamperConfig"
    const val GET_INSTALLED_APPS = "getInstalledApps"
    const val IS_ACCESSIBILITY_ENABLED = "isAccessibilityEnabled"
    const val OPEN_ACCESSIBILITY_SETTINGS = "openAccessibilitySettings"
    const val CAN_DRAW_OVERLAYS = "canDrawOverlays"
    const val REQUEST_OVERLAY_PERMISSION = "requestOverlayPermission"
}
