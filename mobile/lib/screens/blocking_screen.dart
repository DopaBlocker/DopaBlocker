import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../channels/blocking_channel.dart';
import '../providers/auth_provider.dart';
import '../providers/blocking_provider.dart';
import '../providers/permissions_provider.dart';
import '../theme.dart';
import '../widgets/add_block_dialog.dart';
import '../widgets/block_list_tile.dart';
import '../widgets/ui_kit.dart';

/// Aba "Bloqueios" — lista de itens bloqueados + filtro adulto + adicionar.
class BlockingScreen extends ConsumerWidget {
  const BlockingScreen({super.key});

  bool _isChild(WidgetRef ref) => ref.read(authProvider) is AuthChildSession;

  Future<void> _add(BuildContext context, WidgetRef ref) async {
    final result = await AddBlockDialog.show(context);
    if (result != null) {
      await ref.read(blockingProvider.notifier).addItem(result.value, result.itemType);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(blockingProvider);
    final readOnly = _isChild(ref);
    final perms = ref.watch(protectionPermissionsProvider);
    // Só faz sentido pedir as permissões de bloqueio de app quando há ao menos
    // um app na lista e o usuário pode editar (não é sessão de filho).
    final needsAppPermission =
        !readOnly && state.appCount > 0 && perms.pendingStep != null;

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('BLOQUEADOR', style: AppType.label),
            const Text('Sua lista', style: AppType.h1),
          ],
        ),
        actions: [
          if (!readOnly)
            Padding(
              padding: const EdgeInsets.only(right: 16),
              child: Material(
                color: AppColors.primary,
                borderRadius: BorderRadius.circular(12),
                child: InkWell(
                  borderRadius: BorderRadius.circular(12),
                  onTap: () => _add(context, ref),
                  child: const SizedBox(
                    width: 40, height: 40,
                    child: Icon(Icons.add, color: Colors.white),
                  ),
                ),
              ),
            ),
        ],
      ),
      body: state.isLoading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
              children: [
                if (needsAppPermission) ...[
                  _PermissionBanner(step: perms.pendingStep!),
                  const SizedBox(height: 16),
                ],
                // Filtro adulto (master)
                AppCard(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  child: Row(
                    children: [
                      const InitialBadge('A', icon: Icons.shield_outlined),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('Filtro adulto', style: AppType.body.copyWith(fontWeight: FontWeight.w600)),
                            Text('Resolver de família', style: AppType.caption.copyWith(color: AppColors.textFaint)),
                          ],
                        ),
                      ),
                      Switch(
                        value: state.isAdultFilterEnabled,
                        activeThumbColor: Colors.white,
                        activeTrackColor: AppColors.primary,
                        inactiveThumbColor: AppColors.textSecondary,
                        inactiveTrackColor: AppColors.surfaceHigh,
                        trackOutlineColor: WidgetStateProperty.all(Colors.transparent),
                        onChanged: readOnly
                            ? null
                            : (v) => ref.read(blockingProvider.notifier).toggleAdultFilter(v),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const SectionLabel('Itens'),
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text('${state.activeCount}',
                            style: AppType.mono(
                                size: 12, weight: FontWeight.w600, color: AppColors.textSecondary)),
                        Text(' ativos', style: AppType.caption.copyWith(color: AppColors.textFaint)),
                      ],
                    ),
                  ],
                ),
                if (state.items.isEmpty)
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: AppSpacing.x8),
                    child: AppEmptyState(
                      icon: Icons.shield_outlined,
                      title: 'Nenhum bloqueio ainda',
                      description: 'Comece pelos sites que mais te distraem.',
                    ),
                  )
                else
                  for (var i = 0; i < state.items.length; i++) ...[
                    StaggeredItem(
                      index: i,
                      child: BlockListTile(
                        item: state.items[i],
                        readOnly: readOnly,
                        onRemove: readOnly
                            ? null
                            : () => ref.read(blockingProvider.notifier).removeItem(state.items[i].id),
                      ),
                    ),
                    const SizedBox(height: 10),
                  ],
              ],
            ),
    );
  }
}

/// Banner de aviso quando há apps na lista mas falta uma permissão para o
/// bloqueio de app/overlay funcionar (acessibilidade ou "Sobrepor a outros
/// apps"). Pede uma permissão por vez (a próxima pendente).
class _PermissionBanner extends StatelessWidget {
  final ProtectionStep step;

  const _PermissionBanner({required this.step});

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
