abstract final class AppConstants {
  // URLs do backend — trocar para prod antes de release
  static const String backendUrl = 'http://10.0.2.2:3000'; // emulador Android → localhost da máquina
  static const String backendUrlProd = 'https://api.dopablocker.com';

  // MethodChannel para bridge com Kotlin nativo
  static const String blockingChannel = 'com.dopablocker/blocking';

  // Chaves do flutter_secure_storage (sessão do filho)
  static const String keyDeviceToken = 'device_token';
  static const String keyDeviceId = 'device_id';
  static const String keyUserId = 'user_id';

  // Timeouts HTTP
  static const Duration connectTimeout = Duration(seconds: 10);
  static const Duration receiveTimeout = Duration(seconds: 15);
}
