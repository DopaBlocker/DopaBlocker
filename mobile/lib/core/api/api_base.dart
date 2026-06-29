import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/interceptors.dart';
import 'package:dopablocker_mobile/core/constants.dart';

/// Dio compartilhado por todos os clientes de API por domínio
/// (`AuthApi`/`BlocklistApi`/`DevicesApi`). Configura `baseUrl`, timeouts, o
/// interceptor de auth ([AuthInterceptor] — header Firebase/Device Token +
/// refresh de 401) e o log de rede em debug.
final dioProvider = Provider<Dio>((ref) {
  final dio = Dio(BaseOptions(
    baseUrl: AppConstants.backendUrl,
    connectTimeout: AppConstants.connectTimeout,
    receiveTimeout: AppConstants.receiveTimeout,
  ));
  dio.interceptors.add(AuthInterceptor());
  if (kDebugMode) {
    dio.interceptors.add(LogInterceptor(requestBody: true, responseBody: true));
  }
  return dio;
});
