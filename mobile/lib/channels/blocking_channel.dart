import 'package:flutter/services.dart';

import '../core/constants.dart';

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

  /// Atualiza a lista de domínios bloqueados sem reiniciar a VPN.
  /// A regra do pai imune é aplicada antes de chamar isto (lista vazia no
  /// device do pai em modo parental).
  static Future<void> updateBlocklist(List<String> domains) =>
      _channel.invokeMethod('updateBlocklist', {'domains': domains});

  /// Indica se o AccessibilityService do DopaBlocker está ativado pelo usuário.
  static Future<bool> isAccessibilityEnabled() async {
    final enabled = await _channel.invokeMethod<bool>('isAccessibilityEnabled');
    return enabled ?? false;
  }

  /// Abre a tela de configurações de acessibilidade do sistema para o usuário
  /// ativar o AppBlockerService manualmente.
  static Future<void> openAccessibilitySettings() =>
      _channel.invokeMethod('openAccessibilitySettings');
}
