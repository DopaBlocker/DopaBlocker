/// Nomes dos métodos do MethodChannel de bloqueio (`AppConstants.blockingChannel`).
///
/// Fonte única no lado Dart — devem casar **exatamente** com o lado nativo
/// (Kotlin `BlockingMethods`); um nome divergente faz a chamada cair em
/// `result.notImplemented()`. Centralizar aqui evita que os dois lados saiam de
/// sincronia silenciosamente.
abstract final class BlockingMethods {
  static const String startVpn = 'startVpn';
  static const String stopVpn = 'stopVpn';
  static const String isVpnActive = 'isVpnActive';
  static const String isVpnPrepared = 'isVpnPrepared';
  static const String updateBlocklist = 'updateBlocklist';
  static const String updateBlockedApps = 'updateBlockedApps';
  static const String setAdultFilter = 'setAdultFilter';
  static const String setTamperConfig = 'setTamperConfig';
  static const String getInstalledApps = 'getInstalledApps';
  static const String isAccessibilityEnabled = 'isAccessibilityEnabled';
  static const String openAccessibilitySettings = 'openAccessibilitySettings';
  static const String canDrawOverlays = 'canDrawOverlays';
  static const String requestOverlayPermission = 'requestOverlayPermission';
}
