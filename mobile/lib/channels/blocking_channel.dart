import 'package:flutter/services.dart';

import '../core/constants.dart';
import '../models/installed_app.dart';

/// Ponte Flutter ↔ Kotlin para os serviços nativos de bloqueio (VPN +
/// Accessibility). O lado Kotlin vive em MainActivity.kt e delega para
/// VpnManager / DnsVpnService / AppBlockerService.
///
/// Fase M1: a ponte e a tipagem estão prontas. A interceptação DNS e o
/// bloqueio de apps no Kotlin são da Fase M2 — até lá, `startVpn` sobe a VPN
/// mas o filtro de pacotes ainda é stub.
abstract final class BlockingChannel {
  static const MethodChannel _channel = MethodChannel(AppConstants.blockingChannel);

  /// Solicita a permissão de VPN (se necessário) e inicia o serviço.
  /// Retorna `true` se a VPN foi autorizada e iniciada.
  static Future<bool> startVpn() async {
    final ok = await _channel.invokeMethod<bool>('startVpn');
    return ok ?? false;
  }

  /// Para o serviço de VPN.
  static Future<void> stopVpn() => _channel.invokeMethod('stopVpn');

  /// Indica se a VPN está rodando.
  static Future<bool> isVpnActive() async {
    final active = await _channel.invokeMethod<bool>('isVpnActive');
    return active ?? false;
  }

  /// Indica se o consentimento de VPN já foi concedido (não precisa mais abrir o
  /// diálogo do sistema). Usado no muro obrigatório do filho para saber se a
  /// etapa de VPN já está cumprida.
  static Future<bool> isVpnPrepared() async {
    final prepared = await _channel.invokeMethod<bool>('isVpnPrepared');
    return prepared ?? false;
  }

  /// Atualiza a lista de domínios bloqueados sem reiniciar a VPN.
  /// A regra do pai imune é aplicada antes de chamar isto (lista vazia no
  /// device do pai em modo parental).
  static Future<void> updateBlocklist(List<String> domains) =>
      _channel.invokeMethod('updateBlocklist', {'domains': domains});

  /// Atualiza a lista de **pacotes de apps** bloqueados (C3). Os pacotes vêm
  /// dos itens `item_type == 'app'`. Pai imune aplica antes (lista vazia).
  static Future<void> updateBlockedApps(List<String> packages) =>
      _channel.invokeMethod('updateBlockedApps', {'packages': packages});

  /// Liga/desliga o filtro adulto (C4): troca o resolver DNS upstream por um
  /// de família quando ligado.
  static Future<void> setAdultFilter(bool enabled) =>
      _channel.invokeMethod('setAdultFilter', {'enabled': enabled});

  /// Grava no nativo a config usada para reportar adulteração ao backend
  /// (C2.1/C2.2). Necessário porque o serviço nativo não lê o
  /// `flutter_secure_storage`. `deviceToken`/`backendUrl` nulos limpam a config.
  static Future<void> setTamperConfig({
    required String? deviceToken,
    required String? backendUrl,
    required bool isChild,
  }) =>
      _channel.invokeMethod('setTamperConfig', {
        'deviceToken': deviceToken,
        'backendUrl': backendUrl,
        'isChild': isChild,
      });

  /// Lista os apps lançáveis instalados (nome + ícone) para o seletor visual de
  /// bloqueio de app. Devolve vazio se o nativo não suportar/indisponível.
  static Future<List<InstalledApp>> getInstalledApps() async {
    final raw = await _channel.invokeMethod<List<dynamic>>('getInstalledApps');
    if (raw == null) return const [];
    return raw
        .whereType<Map>()
        .map(InstalledApp.fromMap)
        .toList();
  }

  /// Indica se o AccessibilityService do DopaBlocker está ativado pelo usuário.
  static Future<bool> isAccessibilityEnabled() async {
    final enabled = await _channel.invokeMethod<bool>('isAccessibilityEnabled');
    return enabled ?? false;
  }

  /// Abre a tela de configurações de acessibilidade do sistema para o usuário
  /// ativar o AppBlockerService manualmente.
  static Future<void> openAccessibilitySettings() =>
      _channel.invokeMethod('openAccessibilitySettings');

  /// Indica se a permissão "Sobrepor a outros apps" (SYSTEM_ALERT_WINDOW) está
  /// concedida — necessária para o overlay de bloqueio aparecer de forma
  /// confiável.
  static Future<bool> canDrawOverlays() async {
    final ok = await _channel.invokeMethod<bool>('canDrawOverlays');
    return ok ?? false;
  }

  /// Abre a tela do sistema para o usuário conceder "Sobrepor a outros apps".
  static Future<void> requestOverlayPermission() =>
      _channel.invokeMethod('requestOverlayPermission');
}
