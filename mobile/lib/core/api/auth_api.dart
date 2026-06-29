import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/api_base.dart';
import 'package:dopablocker_mobile/core/api/api_dtos.dart';
import 'package:dopablocker_mobile/shared/models/user.dart';

final authApiProvider = Provider<AuthApi>((ref) => AuthApi(ref.read(dioProvider)));

/// Cliente das rotas de autenticação/conta (`/auth/*`): código de email,
/// login, cadastro, leitura/atualização do modo e exclusão da conta.
class AuthApi {
  final Dio _dio;
  AuthApi(this._dio);

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

  /// Troca o modo da conta (personal↔parental) sem recriá-la. Só Firebase JWT.
  Future<User> updateMode(String mode) async {
    final res = await _dio.put('/auth/me', data: {'mode': mode});
    return User.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> deleteAccount() => _dio.delete('/auth/me');
}
