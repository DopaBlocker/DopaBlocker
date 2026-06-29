import 'dart:convert';
import 'dart:typed_data';

/// App instalado no device, exposto pelo nativo (`getInstalledApps`) para o
/// seletor visual de bloqueio de app. O ícone vem como PNG base64 do Kotlin.
class InstalledApp {
  final String packageName;
  final String appName;
  final Uint8List? icon;

  const InstalledApp({
    required this.packageName,
    required this.appName,
    this.icon,
  });

  factory InstalledApp.fromMap(Map<dynamic, dynamic> map) {
    final iconB64 = map['icon'] as String?;
    return InstalledApp(
      packageName: (map['packageName'] as String?) ?? '',
      appName: (map['appName'] as String?) ?? '',
      icon: iconB64 != null ? base64Decode(iconB64) : null,
    );
  }
}
