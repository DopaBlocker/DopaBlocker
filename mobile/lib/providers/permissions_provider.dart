import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../channels/blocking_channel.dart';

/// Estado das permissões nativas de bloqueio: consentimento de **VPN** (bloqueio
/// de sites por DNS), **serviço de acessibilidade** (bloqueio de apps) e
/// **"Sobrepor a outros apps"** (SYSTEM_ALERT_WINDOW, overlay de bloqueio).
///
/// Sem a VPN, sites não são bloqueados; sem a acessibilidade,
/// `onAccessibilityEvent` nunca dispara e apps não são bloqueados; sem o
/// overlay, a tela de bloqueio pode não aparecer de forma confiável entre OEMs.
class ProtectionPermissions {
  final bool vpnPrepared;
  final bool accessibilityEnabled;
  final bool canDrawOverlays;

  const ProtectionPermissions({
    this.vpnPrepared = false,
    this.accessibilityEnabled = false,
    this.canDrawOverlays = false,
  });

  /// Próxima permissão pendente do **bloqueio de apps** (acessibilidade →
  /// overlay), ou null se tudo ok. Usado pela aba Bloqueios na conta pessoal/pai
  /// — NÃO inclui VPN, que no modo pessoal sobe pelo toggle de bloqueio.
  ProtectionStep? get pendingStep {
    if (!accessibilityEnabled) return ProtectionStep.accessibility;
    if (!canDrawOverlays) return ProtectionStep.overlay;
    return null;
  }

  /// Próxima permissão pendente da **proteção completa do filho** (VPN de sites →
  /// acessibilidade → overlay de apps), ou null se tudo ok. Usado no muro
  /// obrigatório da tela do filho.
  ProtectionStep? get childPendingStep {
    if (!vpnPrepared) return ProtectionStep.vpn;
    if (!accessibilityEnabled) return ProtectionStep.accessibility;
    if (!canDrawOverlays) return ProtectionStep.overlay;
    return null;
  }

  ProtectionPermissions copyWith({
    bool? vpnPrepared,
    bool? accessibilityEnabled,
    bool? canDrawOverlays,
  }) =>
      ProtectionPermissions(
        vpnPrepared: vpnPrepared ?? this.vpnPrepared,
        accessibilityEnabled: accessibilityEnabled ?? this.accessibilityEnabled,
        canDrawOverlays: canDrawOverlays ?? this.canDrawOverlays,
      );
}

/// Permissão de proteção pendente a pedir ao usuário.
enum ProtectionStep { vpn, accessibility, overlay }

final protectionPermissionsProvider =
    StateNotifierProvider<ProtectionPermissionsNotifier, ProtectionPermissions>(
  (ref) => ProtectionPermissionsNotifier()..refresh(),
);

class ProtectionPermissionsNotifier extends StateNotifier<ProtectionPermissions> {
  ProtectionPermissionsNotifier() : super(const ProtectionPermissions());

  /// Re-consulta o estado das permissões no nativo. Chamado no boot e a cada
  /// retorno ao app — o usuário concede as permissões fora do app (Configurações)
  /// e volta.
  Future<void> refresh() async {
    try {
      final vpn = await BlockingChannel.isVpnPrepared();
      final accessibility = await BlockingChannel.isAccessibilityEnabled();
      final overlay = await BlockingChannel.canDrawOverlays();
      if (!mounted) return;
      state = ProtectionPermissions(
        vpnPrepared: vpn,
        accessibilityEnabled: accessibility,
        canDrawOverlays: overlay,
      );
    } catch (_) {
      // Plataforma sem suporte nativo — mantém o estado atual.
    }
  }
}
