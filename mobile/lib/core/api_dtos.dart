/// DTOs de request e response da API REST.
/// Espelham os modelos de backend/src/models.rs (RegisterRequest, ConfirmLinkRequest, etc.).

// ── Auth ──────────────────────────────────────────────────────────────────────

class EmailCodeStartResponse {
  final String expiresAt;
  final int resendAfterSeconds;

  const EmailCodeStartResponse({required this.expiresAt, required this.resendAfterSeconds});

  factory EmailCodeStartResponse.fromJson(Map<String, dynamic> j) =>
      EmailCodeStartResponse(
        expiresAt: j['expires_at'] as String,
        resendAfterSeconds: j['resend_after_seconds'] as int,
      );
}

class EmailCodeVerifyResponse {
  final String emailVerificationToken;

  const EmailCodeVerifyResponse({required this.emailVerificationToken});

  factory EmailCodeVerifyResponse.fromJson(Map<String, dynamic> j) =>
      EmailCodeVerifyResponse(emailVerificationToken: j['email_verification_token'] as String);
}

// ── Devices ───────────────────────────────────────────────────────────────────

class GenerateLinkCodeResponse {
  final String code;
  final String expiresAt;

  const GenerateLinkCodeResponse({required this.code, required this.expiresAt});

  factory GenerateLinkCodeResponse.fromJson(Map<String, dynamic> j) =>
      GenerateLinkCodeResponse(
        code: j['code'] as String,
        expiresAt: j['expires_at'] as String,
      );
}

class ConfirmLinkCodeResponse {
  final String deviceToken; // "dt_<plain>" — guardar no secure storage
  final String deviceId;
  final String userId;
  final String parentDeviceId;

  const ConfirmLinkCodeResponse({
    required this.deviceToken,
    required this.deviceId,
    required this.userId,
    required this.parentDeviceId,
  });

  factory ConfirmLinkCodeResponse.fromJson(Map<String, dynamic> j) =>
      ConfirmLinkCodeResponse(
        deviceToken: j['device_token'] as String,
        deviceId: j['device_id'] as String,
        userId: j['user_id'] as String,
        parentDeviceId: j['parent_device_id'] as String,
      );
}

// ── Blocklist ─────────────────────────────────────────────────────────────────

class AdultFilterSettings {
  final String id;
  final String userId;
  final bool isEnabled;
  final String? lastListUpdate;

  const AdultFilterSettings({
    required this.id,
    required this.userId,
    required this.isEnabled,
    this.lastListUpdate,
  });

  factory AdultFilterSettings.fromJson(Map<String, dynamic> j) => AdultFilterSettings(
        id: j['id'] as String,
        userId: j['user_id'] as String,
        isEnabled: j['is_enabled'] == true || j['is_enabled'] == 1,
        lastListUpdate: j['last_list_update'] as String?,
      );
}
