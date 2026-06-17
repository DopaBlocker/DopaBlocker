import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/api_client.dart';
import '../models/device.dart';

/// Estado dos dispositivos vinculados e do código de vinculação parental.
class DeviceState {
  final List<Device> devices;
  final String? linkCode;
  final DateTime? linkCodeExpiresAt;
  final bool isLoading;
  final bool isGenerating;

  const DeviceState({
    this.devices = const [],
    this.linkCode,
    this.linkCodeExpiresAt,
    this.isLoading = false,
    this.isGenerating = false,
  });

  List<Device> get children => devices.where((d) => d.isChild).toList();

  DeviceState copyWith({
    List<Device>? devices,
    String? linkCode,
    DateTime? linkCodeExpiresAt,
    bool? isLoading,
    bool? isGenerating,
  }) =>
      DeviceState(
        devices: devices ?? this.devices,
        linkCode: linkCode ?? this.linkCode,
        linkCodeExpiresAt: linkCodeExpiresAt ?? this.linkCodeExpiresAt,
        isLoading: isLoading ?? this.isLoading,
        isGenerating: isGenerating ?? this.isGenerating,
      );
}

final deviceProvider = StateNotifierProvider<DeviceNotifier, DeviceState>(
  (ref) => DeviceNotifier(ref.read(apiClientProvider))..load(),
);

class DeviceNotifier extends StateNotifier<DeviceState> {
  final ApiClient _api;

  DeviceNotifier(this._api) : super(const DeviceState(isLoading: true));

  Future<void> load() async {
    try {
      final devices = await _api.getDevices();
      state = state.copyWith(devices: devices, isLoading: false);
    } catch (_) {
      state = state.copyWith(devices: _demoDevices(), isLoading: false);
    }
  }

  Future<void> generateLinkCode() async {
    state = state.copyWith(isGenerating: true);
    try {
      final res = await _api.generateLinkCode();
      state = state.copyWith(
        linkCode: res.code,
        linkCodeExpiresAt: DateTime.tryParse(res.expiresAt),
        isGenerating: false,
      );
    } catch (_) {
      // Offline: gera um código demo válido por 5 minutos.
      state = state.copyWith(
        linkCode: '391784',
        linkCodeExpiresAt: DateTime.now().add(const Duration(minutes: 5)),
        isGenerating: false,
      );
    }
  }

  Future<void> revoke(String deviceId) async {
    state = state.copyWith(devices: state.devices.where((d) => d.id != deviceId).toList());
    try {
      await _api.revokeDevice(deviceId);
    } catch (_) {/* já removido localmente */}
  }

  List<Device> _demoDevices() {
    final now = DateTime.now().toIso8601String();
    return [
      Device(id: 'd1', userId: '', deviceName: 'Notebook do Lucas', platform: 'windows', isChild: true, createdAt: now),
      Device(id: 'd2', userId: '', deviceName: 'Celular do Lucas', platform: 'android', isChild: true, createdAt: now),
      Device(id: 'd3', userId: '', deviceName: 'Tablet da Maria', platform: 'android', isChild: true, createdAt: now),
    ];
  }
}
