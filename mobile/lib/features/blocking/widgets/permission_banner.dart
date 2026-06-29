import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/core/channels/blocking_channel.dart';
import 'package:dopablocker_mobile/features/blocking/providers/permissions_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Banner de aviso quando há apps na lista mas falta uma permissão para o
/// bloqueio de app/overlay funcionar (acessibilidade ou "Sobrepor a outros
/// apps"). Pede uma permissão por vez (a próxima pendente).
class PermissionBanner extends StatelessWidget {
  final ProtectionStep step;

  const PermissionBanner({required this.step, super.key});

  @override
  Widget build(BuildContext context) {
    final (title, subtitle, action) = switch (step) {
      ProtectionStep.vpn => (
          'Ative a VPN de bloqueio',
          'O DopaBlocker usa uma VPN local para bloquear os sites da sua lista.',
          BlockingChannel.startVpn,
        ),
      ProtectionStep.accessibility => (
          'Ative o bloqueio de apps',
          'O DopaBlocker precisa do serviço de acessibilidade para detectar e '
              'bloquear os apps da sua lista.',
          BlockingChannel.openAccessibilitySettings,
        ),
      ProtectionStep.overlay => (
          'Permita sobrepor a outros apps',
          'Necessário para a tela de bloqueio aparecer por cima do app ou '
              'navegador.',
          BlockingChannel.requestOverlayPermission,
        ),
    };

    return AppCard(
      color: AppColors.warningDim,
      border: Border.all(color: AppColors.warning.withValues(alpha: 0.4)),
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.warning_amber_rounded, color: AppColors.warning, size: 20),
              const SizedBox(width: 8),
              Expanded(
                child: Text(title,
                    style: AppType.body.copyWith(fontWeight: FontWeight.w700)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(subtitle, style: AppType.bodySm.copyWith(color: AppColors.textSecondary)),
          const SizedBox(height: 12),
          AppButton(
            label: 'Ativar',
            icon: Icons.tune,
            fullWidth: false,
            onPressed: () => action(),
          ),
        ],
      ),
    );
  }
}
