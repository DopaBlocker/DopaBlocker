import 'package:firebase_auth/firebase_auth.dart';

/// Holder singleton que decide qual header Authorization mandar em cada request.
/// O [AuthNotifier] atualiza o modo sempre que o estado de auth muda.
/// O interceptor do Dio consulta [getHeader] sem precisar saber o tipo de conta.
///
/// Espelha o AuthProvider do desktop (auth-provider.ts).
class AuthHeaderHolder {
  AuthHeaderHolder._();
  static final AuthHeaderHolder instance = AuthHeaderHolder._();

  Future<String?> Function() _getter = () async => null;
  Future<bool> Function() _refresher = () async => false;

  /// Configura para usar Firebase JWT — renovação automática pelo SDK.
  void setFirebase() {
    _getter = () async {
      final token = await FirebaseAuth.instance.currentUser?.getIdToken();
      return token != null ? 'Bearer $token' : null;
    };
    _refresher = () async {
      final user = FirebaseAuth.instance.currentUser;
      if (user == null) return false;
      try {
        await user.getIdToken(true); // força refresh
        return true;
      } catch (_) {
        return false;
      }
    };
  }

  /// Configura para usar Device Token do filho (prefixo dt_ já incluso).
  void setChild(String deviceToken) {
    _getter = () async => 'Bearer $deviceToken';
    _refresher = () async => false; // device token não expira via refresh
  }

  /// Sem autenticação — rotas públicas (link/confirm, email-code/*).
  void clear() {
    _getter = () async => null;
    _refresher = () async => false;
  }

  Future<String?> getHeader() => _getter();
  Future<bool> refresh() => _refresher();
}
