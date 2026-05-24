import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/user.dart';
import '../models/blocked_item.dart';
import '../models/device.dart';
import 'api_dtos.dart';
import 'auth_header_holder.dart';
import 'constants.dart';

final apiClientProvider = Provider<ApiClient>((ref) => ApiClient());

/// Exceção tipada lançada quando o backend retorna um erro HTTP conhecido.
class ApiException implements Exception {
  final int statusCode;
  final String message;
  const ApiException({required this.statusCode, required this.message});

  @override
  String toString() => 'ApiException($statusCode): $message';
}

class ApiClient {
  late final Dio _dio;

  ApiClient() {
    _dio = Dio(BaseOptions(
      baseUrl: AppConstants.backendUrl,
      connectTimeout: AppConstants.connectTimeout,
      receiveTimeout: AppConstants.receiveTimeout,
    ));
    _dio.interceptors.add(_AuthInterceptor());
    if (kDebugMode) {
      _dio.interceptors.add(LogInterceptor(requestBody: true, responseBody: true));
    }
  }

  // ── Auth ────────────────────────────────────────────────────────────────────

  Future<EmailCodeStartResponse> emailCodeStart(String email) async {
    final res = await _dio.post('/auth/email-code/start', data: {'email': email});
    return EmailCodeStartResponse.fromJson(res.data as Map<String, dynamic>);
  }

  Future<EmailCodeVerifyResponse> emailCodeVerify(String email, String code) async {
    final res = await _dio.post('/auth/email-code/verify', data: {'email': email, 'code': code});
    return EmailCodeVerifyResponse.fromJson(res.data as Map<String, dynamic>);
  }

  Future<User> register({
    required String email,
    required String displayName,
    required String mode,
    String? emailVerificationToken,
  }) async {
    final res = await _dio.post('/auth/register', data: {
      'email': email,
      'display_name': displayName,
      'mode': mode,
      if (emailVerificationToken != null) 'email_verification_token': emailVerificationToken,
    });
    return User.fromJson(res.data as Map<String, dynamic>);
  }

  Future<User> login() async {
    final res = await _dio.post('/auth/login');
    return User.fromJson(res.data as Map<String, dynamic>);
  }

  Future<User> me() async {
    final res = await _dio.get('/auth/me');
    return User.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> deleteAccount() => _dio.delete('/auth/me');

  // ── Blocklist ───────────────────────────────────────────────────────────────

  Future<List<BlockedItem>> getBlocklist() async {
    final res = await _dio.get('/blocklist');
    return (res.data as List).map((e) => BlockedItem.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<BlockedItem> addBlockedItem({required String itemType, required String value}) async {
    final res = await _dio.post('/blocklist', data: {'item_type': itemType, 'value': value});
    return BlockedItem.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> removeBlockedItem(String id) => _dio.delete('/blocklist/$id');

  Future<AdultFilterSettings> setAdultFilter(bool enabled) async {
    final res = await _dio.put('/blocklist/adult-filter', data: {'enabled': enabled});
    return AdultFilterSettings.fromJson(res.data as Map<String, dynamic>);
  }

  // ── Devices ─────────────────────────────────────────────────────────────────

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

// ── Interceptor ──────────────────────────────────────────────────────────────

class _AuthInterceptor extends Interceptor {
  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) async {
    final header = await AuthHeaderHolder.instance.getHeader();
    if (header != null) options.headers['Authorization'] = header;
    handler.next(options);
  }

  @override
  void onError(DioException err, ErrorInterceptorHandler handler) async {
    // Tenta refresh em 401 e reenvia uma vez
    if (err.response?.statusCode == 401) {
      final refreshed = await AuthHeaderHolder.instance.refresh();
      if (refreshed) {
        final newHeader = await AuthHeaderHolder.instance.getHeader();
        final opts = err.requestOptions;
        if (newHeader != null) opts.headers['Authorization'] = newHeader;
        try {
          final response = await Dio().fetch(opts);
          handler.resolve(response);
          return;
        } catch (_) {}
      }
    }

    // Converte DioException em ApiException com statusCode e mensagem legível
    final statusCode = err.response?.statusCode ?? 0;
    final data = err.response?.data;
    final message = (data is Map ? data['error'] : null) ?? err.message ?? 'Erro desconhecido';
    handler.next(DioException(
      requestOptions: err.requestOptions,
      response: err.response,
      error: ApiException(statusCode: statusCode, message: message.toString()),
    ));
  }
}
