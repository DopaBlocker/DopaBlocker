import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/api_base.dart';
import 'package:dopablocker_mobile/core/api/api_dtos.dart';
import 'package:dopablocker_mobile/shared/models/device.dart';
import 'package:dopablocker_mobile/shared/models/device_event.dart';

final devicesApiProvider =
    Provider<DevicesApi>((ref) => DevicesApi(ref.read(dioProvider)));

/// Cliente das rotas de dispositivos (`/devices*`): registro, listagem,
/// vínculo de filho (gerar/confirmar código), revogação e eventos de
/// adulteração.
class DevicesApi {
  final Dio _dio;
  DevicesApi(this._dio);

  /// `GET /devices/events` — alertas de adulteração dos filhos (só pai/Firebase).
  Future<List<DeviceEvent>> getDeviceEvents() async {
    final res = await _dio.get('/devices/events');
    return (res.data as List).map((e) => DeviceEvent.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<Device> registerDevice({required String deviceName}) async {
    final res = await _dio.post('/devices/register', data: {
      'device_name': deviceName,
      'platform': 'android',
    });
    return Device.fromJson(res.data as Map<String, dynamic>);
  }

  Future<List<Device>> getDevices() async {
    final res = await _dio.get('/devices');
    return (res.data as List).map((e) => Device.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<GenerateLinkCodeResponse> generateLinkCode() async {
    final res = await _dio.post('/devices/link/generate');
    return GenerateLinkCodeResponse.fromJson(res.data as Map<String, dynamic>);
  }

  Future<ConfirmLinkCodeResponse> confirmLinkCode({
    required String code,
    required String deviceName,
  }) async {
    final res = await _dio.post('/devices/link/confirm', data: {
      'code': code,
      'device_name': deviceName,
      'platform': 'android',
    });
    return ConfirmLinkCodeResponse.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> revokeDevice(String deviceId) => _dio.post('/devices/$deviceId/revoke');
}
