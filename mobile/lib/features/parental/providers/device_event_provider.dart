import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/devices_api.dart';
import 'package:dopablocker_mobile/shared/models/device_event.dart';

/// Estado dos alertas de adulteração (C2.1) que o pai vê no painel.
class DeviceEventsState {
  final List<DeviceEvent> events;
  final bool isLoading;

  const DeviceEventsState({this.events = const [], this.isLoading = false});

  DeviceEventsState copyWith({List<DeviceEvent>? events, bool? isLoading}) =>
      DeviceEventsState(
        events: events ?? this.events,
        isLoading: isLoading ?? this.isLoading,
      );
}

/// Alertas de adulteração dos filhos. Faz poll periódico de `GET /devices/events`
/// (entrega in-app: o pai não recebe push nesta fase, só o painel atualiza).
final deviceEventsProvider =
    StateNotifierProvider<DeviceEventsNotifier, DeviceEventsState>(
  (ref) => DeviceEventsNotifier(ref.read(devicesApiProvider))..load(),
);

class DeviceEventsNotifier extends StateNotifier<DeviceEventsState> {
  final DevicesApi _api;
  Timer? _timer;

  static const Duration _interval = Duration(seconds: 60);

  DeviceEventsNotifier(this._api)
      : super(const DeviceEventsState(isLoading: true)) {
    _timer = Timer.periodic(_interval, (_) => load());
  }

  Future<void> load() async {
    try {
      final events = await _api.getDeviceEvents();
      state = state.copyWith(events: events, isLoading: false);
    } catch (_) {
      // Backend indisponível / sem permissão: mantém o que já tem.
      state = state.copyWith(isLoading: false);
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}
