import 'package:dio/dio.dart';

import 'package:dopablocker_mobile/core/api/api_exception.dart';
import 'package:dopablocker_mobile/core/auth_header_holder.dart';

/// Interceptor de auth do Dio: injeta o header Authorization atual (Firebase JWT
/// ou Device Token do filho), tenta um refresh em 401 e reenvia a requisição uma
/// vez, e converte `DioException` em [ApiException] com `statusCode` + mensagem
/// legível para o resto do app.
class AuthInterceptor extends Interceptor {
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
