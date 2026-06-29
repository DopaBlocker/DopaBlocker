import 'package:firebase_auth/firebase_auth.dart' as fb;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:dio/dio.dart';

import 'package:dopablocker_mobile/core/api/api_exception.dart';
import 'package:dopablocker_mobile/core/api/auth_api.dart';
import 'package:dopablocker_mobile/core/api/devices_api.dart';
import 'package:dopablocker_mobile/core/auth_header_holder.dart';
import 'package:dopablocker_mobile/core/channels/blocking_channel.dart';
import 'package:dopablocker_mobile/core/constants.dart';
import 'package:dopablocker_mobile/core/firebase_service.dart';
import 'package:dopablocker_mobile/shared/models/user.dart';

// ── Estado ───────────────────────────────────────────────────────────────────

/// Máquina de estados de autenticação.
/// Contrato definido em docs/ARCHITECTURE.md ("Máquina de estados de auth") —
/// implementação de referência do desktop em desktop/src/lib/stores/auth.ts.
sealed class AuthState {}

class AuthBooting extends AuthState {}

class AuthSignedOut extends AuthState {}

class AuthAuthenticating extends AuthState {}

class AuthAuthenticated extends AuthState {
  final User user;
  final fb.User firebaseUser;
  AuthAuthenticated({required this.user, required this.firebaseUser});
}

class AuthChildSession extends AuthState {
  final String deviceToken; // "dt_<plain>"
  final String deviceId;
  final String userId; // user_id do pai
  AuthChildSession({required this.deviceToken, required this.deviceId, required this.userId});
}

/// Firebase OK, mas POST /auth/login retornou 404 — conta local ainda não existe.
/// Frontend deve chamar register() para concluir o cadastro.
class AuthPendingLocalRegistration extends AuthState {
  final fb.User firebaseUser;
  AuthPendingLocalRegistration({required this.firebaseUser});
}

class AuthBackendUnavailable extends AuthState {}

// ── Provider ─────────────────────────────────────────────────────────────────

final authProvider = StateNotifierProvider<AuthNotifier, AuthState>(
  (ref) => AuthNotifier(
    ref.read(authApiProvider),
    ref.read(devicesApiProvider),
    ref.read(firebaseServiceProvider),
  ),
);

// ── Notifier ─────────────────────────────────────────────────────────────────

class AuthNotifier extends StateNotifier<AuthState> {
  final AuthApi _api;
  final DevicesApi _devicesApi;
  final FirebaseService _firebase;
  final FlutterSecureStorage _storage;

  AuthNotifier(this._api, this._devicesApi, this._firebase)
      : _storage = const FlutterSecureStorage(
          aOptions: AndroidOptions(encryptedSharedPreferences: true),
          iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock),
        ),
        super(AuthBooting()) {
    _boot();
  }

  // ── Boot ───────────────────────────────────────────────────────────────────

  Future<void> _boot() async {
    // 1. Tenta restaurar child_session do secure storage
    final token = await _storage.read(key: AppConstants.keyDeviceToken);
    if (token != null) {
      final deviceId = await _storage.read(key: AppConstants.keyDeviceId) ?? '';
      final userId = await _storage.read(key: AppConstants.keyUserId) ?? '';
      AuthHeaderHolder.instance.setChild(token);
      final valid = await _api.me().then((_) => true).catchError((_) => false);
      if (valid) {
        _syncTamperConfig(deviceToken: token, isChild: true);
        state = AuthChildSession(deviceToken: token, deviceId: deviceId, userId: userId);
        return;
      }
      // Token revogado — limpa e continua
      await _clearChildSession();
    }

    // 2. Verifica Firebase
    final firebaseUser = fb.FirebaseAuth.instance.currentUser;
    if (firebaseUser == null) {
      AuthHeaderHolder.instance.clear();
      state = AuthSignedOut();
      return;
    }
    AuthHeaderHolder.instance.setFirebase();
    await _hydrateFromFirebase(firebaseUser);
  }

  Future<void> _hydrateFromFirebase(fb.User firebaseUser) async {
    state = AuthAuthenticating();
    try {
      final user = await _api.login();
      state = AuthAuthenticated(user: user, firebaseUser: firebaseUser);
    } on DioException catch (e) {
      final err = e.error;
      if (err is ApiException && err.statusCode == 404) {
        state = AuthPendingLocalRegistration(firebaseUser: firebaseUser);
      } else {
        state = AuthBackendUnavailable();
      }
    } catch (_) {
      state = AuthBackendUnavailable();
    }
  }

  // ── Login / cadastro ───────────────────────────────────────────────────────

  Future<void> loginWithEmail(String email, String password) async {
    state = AuthAuthenticating();
    try {
      AuthHeaderHolder.instance.clear();
      final result = await _firebase.signInWithEmail(email, password);
      AuthHeaderHolder.instance.setFirebase();
      await _hydrateFromFirebase(result.user!);
    } catch (_) {
      state = AuthSignedOut();
      rethrow; // deixa a tela mostrar o erro específico do Firebase
    }
  }

  Future<void> loginWithGoogle() async {
    state = AuthAuthenticating();
    try {
      AuthHeaderHolder.instance.clear();
      final result = await _firebase.signInWithGoogle();
      AuthHeaderHolder.instance.setFirebase();
      await _hydrateFromFirebase(result.user!);
    } catch (_) {
      state = AuthSignedOut();
      rethrow;
    }
  }

  /// Conclui o cadastro local após Firebase signup.
  /// Chamado quando state == AuthPendingLocalRegistration.
  Future<void> register({
    required String displayName,
    required String mode,
    String? emailVerificationToken,
  }) async {
    final current = state;
    if (current is! AuthPendingLocalRegistration) return;
    final firebaseUser = current.firebaseUser;
    try {
      final user = await _api.register(
        email: firebaseUser.email!,
        displayName: displayName,
        mode: mode,
        emailVerificationToken: emailVerificationToken,
      );
      state = AuthAuthenticated(user: user, firebaseUser: firebaseUser);
    } catch (_) {
      state = AuthBackendUnavailable();
      rethrow;
    }
  }

  /// Cadastro por email/senha. Dirige o fluxo inteiro a partir da tela de
  /// cadastro: cria a conta no Firebase e conclui o registro local com o
  /// `emailVerificationToken` (obtido via email-code/verify). Diferente de
  /// [register], que parte do estado AuthPendingLocalRegistration.
  Future<void> registerWithEmail({
    required String email,
    required String password,
    required String displayName,
    required String mode,
    required String emailVerificationToken,
  }) async {
    state = AuthAuthenticating();
    fb.UserCredential? cred;
    try {
      AuthHeaderHolder.instance.clear();
      cred = await _firebase.createAccountWithEmail(email, password);
      AuthHeaderHolder.instance.setFirebase();
      final user = await _api.register(
        email: email,
        displayName: displayName,
        mode: mode,
        emailVerificationToken: emailVerificationToken,
      );
      state = AuthAuthenticated(user: user, firebaseUser: cred.user!);
    } catch (_) {
      // Firebase criou o usuário mas o backend falhou: remove o órfão para o
      // email ficar livre numa nova tentativa (best-effort).
      try {
        await cred?.user?.delete();
      } catch (_) {}
      AuthHeaderHolder.instance.clear();
      state = AuthSignedOut();
      rethrow;
    }
  }

  // ── Fluxo filho ────────────────────────────────────────────────────────────

  Future<void> confirmChildCode(String code, String deviceName) async {
    state = AuthAuthenticating();
    AuthHeaderHolder.instance.clear(); // rota pública, sem header
    try {
      final response = await _devicesApi.confirmLinkCode(code: code, deviceName: deviceName);
      await _storage.write(key: AppConstants.keyDeviceToken, value: response.deviceToken);
      await _storage.write(key: AppConstants.keyDeviceId, value: response.deviceId);
      await _storage.write(key: AppConstants.keyUserId, value: response.userId);
      AuthHeaderHolder.instance.setChild(response.deviceToken);
      _syncTamperConfig(deviceToken: response.deviceToken, isChild: true);
      state = AuthChildSession(
        deviceToken: response.deviceToken,
        deviceId: response.deviceId,
        userId: response.userId,
      );
    } catch (_) {
      state = AuthSignedOut();
      rethrow;
    }
  }

  // ── Logout / revogação ─────────────────────────────────────────────────────

  Future<void> logout() async {
    if (state is AuthChildSession) {
      await _clearChildSession();
    } else {
      await _firebase.signOut();
    }
    AuthHeaderHolder.instance.clear();
    state = AuthSignedOut();
  }

  /// Exclui a conta permanentemente. Ordem (igual ao desktop): **backend
  /// primeiro**, com o token Firebase ainda válido — assim o `DELETE /auth/me`
  /// realmente apaga o user e libera o email (UNIQUE). Se invertêssemos, após
  /// `deleteAccount()` no Firebase o `getIdToken()` volta null e o backend
  /// rejeitaria com 401, deixando um órfão que prende o email no recadastro.
  /// Se o backend falhar, propaga e NÃO toca no Firebase. Se o Firebase exigir
  /// `requires-recent-login`, o backend já foi apagado (email livre) e o erro
  /// propaga para a UI oferecer o relogin.
  Future<void> deleteAccount() async {
    await _api.deleteAccount();
    await _firebase.deleteAccount();
    AuthHeaderHolder.instance.clear();
    state = AuthSignedOut();
  }

  /// Troca o modo da conta (personal↔parental) sem recriá-la. Só em sessão
  /// autenticada (Firebase); atualiza o `user` em memória no sucesso para a UI
  /// refletir na hora. O efeito no engine/sync vem no próximo poll.
  Future<User> updateMode(String mode) async {
    final current = state;
    if (current is! AuthAuthenticated) {
      throw StateError('Só é possível trocar o modo em sessão autenticada.');
    }
    final user = await _api.updateMode(mode);
    state = AuthAuthenticated(user: user, firebaseUser: current.firebaseUser);
    return user;
  }

  /// Tenta sincronizar novamente com o backend quando em BackendUnavailable.
  Future<void> retryBackendSync() async {
    final firebaseUser = fb.FirebaseAuth.instance.currentUser;
    if (firebaseUser != null) {
      AuthHeaderHolder.instance.setFirebase();
      await _hydrateFromFirebase(firebaseUser);
    }
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  Future<void> _clearChildSession() async {
    await _storage.delete(key: AppConstants.keyDeviceToken);
    await _storage.delete(key: AppConstants.keyDeviceId);
    await _storage.delete(key: AppConstants.keyUserId);
    // Limpa a config de tamper no nativo (sem token = não reporta mais).
    _syncTamperConfig(deviceToken: null, isChild: false);
  }

  /// Espelha no engine nativo os dados que o `TamperReporter` usa para reportar
  /// adulteração ao backend (o nativo não lê o `flutter_secure_storage`).
  /// `isChild=false`/token nulo desativa o report.
  void _syncTamperConfig({String? deviceToken, required bool isChild}) {
    BlockingChannel.setTamperConfig(
      deviceToken: deviceToken,
      backendUrl: isChild ? AppConstants.backendUrl : null,
      isChild: isChild,
    ).catchError((_) {});
  }
}
