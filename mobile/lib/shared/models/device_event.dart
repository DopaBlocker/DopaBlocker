/// Espelha backend/src/models.rs → struct DeviceEvent.
/// Evento de adulteração (tamper) reportado por um device filho (C2.1/C2.2).
class DeviceEvent {
  final String id;
  final String deviceId;
  final String kind; // "vpn_revoked" | "vpn_settings_opened" | "dns_settings_opened"
  final String createdAt;
  final String? acknowledgedAt;

  const DeviceEvent({
    required this.id,
    required this.deviceId,
    required this.kind,
    required this.createdAt,
    this.acknowledgedAt,
  });

  factory DeviceEvent.fromJson(Map<String, dynamic> j) => DeviceEvent(
        id: j['id'] as String,
        deviceId: j['device_id'] as String,
        kind: j['kind'] as String,
        createdAt: j['created_at'] as String,
        acknowledgedAt: j['acknowledged_at'] as String?,
      );

  /// Rótulo legível do evento para a tela de alertas do pai.
  String get label => switch (kind) {
        'vpn_revoked' => 'Proteção (VPN) desligada',
        'vpn_settings_opened' => 'Abriu as Configurações de VPN',
        'dns_settings_opened' => 'Abriu as Configurações de DNS',
        _ => 'Atividade suspeita',
      };
}
