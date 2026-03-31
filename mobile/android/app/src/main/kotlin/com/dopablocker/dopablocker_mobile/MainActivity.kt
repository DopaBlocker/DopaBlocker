// Entry point Android — registra o MethodChannel para comunicação com Flutter.
// Implementar: configurar FlutterMethodChannel('com.dopablocker/blocking'),
// registrar handlers para startVpn, stopVpn, isVpnActive, updateBlocklist,
// startAccessibilityService, isAccessibilityEnabled.
// Delegar chamadas para VpnManager e AppBlockerService.

package com.dopablocker.dopablocker_mobile

import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity()
