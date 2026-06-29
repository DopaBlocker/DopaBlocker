import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/channels/blocking_channel.dart';
import 'package:dopablocker_mobile/features/auth/providers/auth_provider.dart';
import 'package:dopablocker_mobile/features/blocking/providers/blocking_provider.dart';
import 'package:dopablocker_mobile/features/blocking/providers/permissions_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Tela do dispositivo de um filho vinculado. Diferente das outras telas, aqui o
/// bloqueio é **obrigatório**: enquanto faltar alguma permissão (consentimento
/// de VPN, acessibilidade, overlay) a tela vira um **muro de configuração** que
/// guia o filho passo a passo e não mostra "proteção ativa" enganosa. Quando
/// tudo está concedido, o engine sobe sozinho (VPN + sincronização da lista do
/// responsável) e o status reflete o estado real.
///
/// Reaplica no `resumed` (o filho concede as permissões fora do app e volta).
class ChildBlockedScreen extends ConsumerStatefulWidget {
  const ChildBlockedScreen({super.key});

  @override
  ConsumerState<ChildBlockedScreen> createState() => _ChildBlockedScreenState();
}

class _ChildBlockedScreenState extends ConsumerState<ChildBlockedScreen>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    WidgetsBinding.instance.addPostFrameCallback((_) => _refreshAndEnforce());
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // O filho sai para as Configurações do sistema para conceder cada permissão
    // e volta — reconsultamos o estado e (re)ativamos o engine.
    if (state == AppLifecycleState.resumed) _refreshAndEnforce();
  }

  /// Reconsulta as permissões e, se todas concedidas, garante o engine rodando.
  Future<void> _refreshAndEnforce() async {
    await ref.read(protectionPermissionsProvider.notifier).refresh();
    if (!mounted) return;
    final perms = ref.read(protectionPermissionsProvider);
    if (perms.childPendingStep == null) {
      await ref.read(blockingProvider.notifier).ensureEngineRunning();
    }
  }

  @override
  Widget build(BuildContext context) {
    final auth = ref.watch(authProvider);
    final session = auth is AuthChildSession ? auth : null;
    // Instancia o blockingProvider (liga o poll B2) e lê os contadores.
    final blocking = ref.watch(blockingProvider);
    final perms = ref.watch(protectionPermissionsProvider);
    final pending = perms.childPendingStep;

    return Scaffold(
      appBar: AppBar(
        title: const Text('DopaBlocker — Filho'),
        actions: [
          IconButton(
            icon: const Icon(Icons.logout),
            tooltip: 'Desvincular',
            onPressed: () => ref.read(authProvider.notifier).logout(),
          ),
        ],
      ),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 360),
          child: Padding(
            padding: const EdgeInsets.all(AppSpacing.x6),
            child: pending != null
                ? _SetupGate(
                    step: pending,
                    onRecheck: _refreshAndEnforce,
                  )
                : _ActiveStatus(deviceId: session?.deviceId, blocking: blocking),
          ),
        ),
      ),
    );
  }
}

/// Muro de configuração obrigatório: pede uma permissão por vez (VPN →
/// acessibilidade → overlay), explicando que o responsável ativou o bloqueio e
/// que ele só funciona depois de concluir os passos.
class _SetupGate extends StatelessWidget {
  final ProtectionStep step;
  final Future<void> Function() onRecheck;

  const _SetupGate({required this.step, required this.onRecheck});

  @override
  Widget build(BuildContext context) {
    final (stepIndex, title, subtitle, buttonLabel, action) = switch (step) {
      ProtectionStep.vpn => (
          1,
          'Ative a proteção de sites',
          'O responsável ativou o bloqueio neste dispositivo. Permita a VPN de '
              'bloqueio para que os sites definidos por ele sejam bloqueados.',
          'Ativar proteção',
          BlockingChannel.startVpn,
        ),
      ProtectionStep.accessibility => (
          2,
          'Ative o bloqueio de apps',
          'Permita o serviço de acessibilidade para que os apps definidos pelo '
              'responsável sejam bloqueados.',
          'Abrir configurações',
          BlockingChannel.openAccessibilitySettings,
        ),
      ProtectionStep.overlay => (
          3,
          'Permita sobrepor a outros apps',
          'Necessário para a tela de bloqueio aparecer por cima do app ou do '
              'navegador.',
          'Permitir',
          BlockingChannel.requestOverlayPermission,
        ),
    };

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const Icon(Icons.shield_outlined, size: 56, color: AppColors.warning),
        const SizedBox(height: AppSpacing.x4),
        Text('Configure a proteção',
            style: AppType.h2, textAlign: TextAlign.center),
        const SizedBox(height: AppSpacing.x2),
        Text('Passo $stepIndex de 3',
            style: AppType.label, textAlign: TextAlign.center),
        const SizedBox(height: AppSpacing.x5),
        AppCard(
          color: AppColors.warningDim,
          border: Border.all(color: AppColors.warning.withValues(alpha: 0.4)),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title,
                  style: AppType.body.copyWith(fontWeight: FontWeight.w700)),
              const SizedBox(height: 6),
              Text(subtitle,
                  style:
                      AppType.bodySm.copyWith(color: AppColors.textSecondary)),
              const SizedBox(height: 14),
              AppButton(label: buttonLabel, onPressed: () => action()),
            ],
          ),
        ),
        const SizedBox(height: AppSpacing.x3),
        AppButton(
          label: 'Já permiti — verificar',
          variant: AppButtonVariant.ghost,
          onPressed: () => onRecheck(),
        ),
      ],
    );
  }
}

/// Status real quando todas as permissões estão concedidas e o engine roda.
class _ActiveStatus extends StatelessWidget {
  final String? deviceId;
  final BlockingState blocking;

  const _ActiveStatus({required this.deviceId, required this.blocking});

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        const AppBrandMark(size: 72),
        const SizedBox(height: AppSpacing.x5),
        Text('Proteção ativa', style: AppType.h2, textAlign: TextAlign.center),
        if (deviceId != null) ...[
          const SizedBox(height: AppSpacing.x1),
          Text('ID: $deviceId',
              style: AppType.mono(size: 12, color: AppColors.textFaint),
              textAlign: TextAlign.center),
        ],
        const SizedBox(height: AppSpacing.x4),
        Text(
          'Bloqueio ativo — gerenciado pelo responsável.',
          style: AppType.body.copyWith(color: AppColors.textSecondary),
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: AppSpacing.x5),
        AppCard(
          padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.x5, vertical: AppSpacing.x4),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              _StatBlock(value: blocking.siteCount, label: 'sites'),
              Container(width: 1, height: 36, color: AppColors.divider),
              _StatBlock(value: blocking.appCount, label: 'apps'),
            ],
          ),
        ),
      ],
    );
  }
}

/// Número grande (mono tabular) + rótulo — usado nas contagens de bloqueio.
class _StatBlock extends StatelessWidget {
  final int value;
  final String label;
  const _StatBlock({required this.value, required this.label});

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('$value',
            style: AppType.mono(
                size: 24, weight: FontWeight.w700, color: AppColors.primary)),
        const SizedBox(height: 2),
        Text(label.toUpperCase(), style: AppType.label),
      ],
    );
  }
}
