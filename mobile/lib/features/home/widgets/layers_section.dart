import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/blocking/providers/blocking_provider.dart';
import 'package:dopablocker_mobile/features/blocking/providers/permissions_provider.dart';
import 'package:dopablocker_mobile/features/home/providers/nav_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Seção "Camadas ativas" (real) da aba Início: sites (DNS), apps (depende de
/// permissão) e filtro adulto, cada um com seu chip de status.
class LayersSection extends ConsumerWidget {
  const LayersSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final blocking = ref.watch(blockingProvider);
    final perms = ref.watch(protectionPermissionsProvider);
    final on = blocking.isBlockingActive;

    // Chip do bloqueio de apps: depende de haver apps na lista + permissão.
    Widget appsChip() {
      if (blocking.appCount == 0) return const AppChip('0 apps');
      if (!perms.accessibilityEnabled) return AppChip.warning('ativar');
      return on ? AppChip.success('ativo') : const AppChip('pausado');
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionLabel('Camadas ativas'),
        AppCard(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
          child: Column(
            children: [
              _LayerRow(
                icon: Icons.public,
                title: 'Bloqueio de sites',
                subtitle: 'Sinkhole de DNS',
                chip: on ? AppChip.success('ativo') : const AppChip('pausado'),
              ),
              const _LayerDivider(),
              _LayerRow(
                icon: Icons.smartphone_outlined,
                title: 'Bloqueio de apps',
                subtitle: '${blocking.appCount} na lista',
                chip: appsChip(),
                onTap: () => ref.read(navIndexProvider.notifier).state = NavTab.bloqueios,
              ),
              const _LayerDivider(),
              _LayerRow(
                icon: Icons.shield_outlined,
                title: 'Filtro adulto',
                subtitle: 'Resolver de família',
                chip: blocking.isAdultFilterEnabled
                    ? AppChip.success('ativo')
                    : const AppChip('off'),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _LayerDivider extends StatelessWidget {
  const _LayerDivider();
  @override
  Widget build(BuildContext context) =>
      const Divider(color: AppColors.divider, height: 1, indent: 56, endIndent: 12);
}

class _LayerRow extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget chip;
  final VoidCallback? onTap;
  const _LayerRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.chip,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(12),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
        child: Row(
          children: [
            Icon(icon, size: 20, color: AppColors.textSecondary),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: AppType.body.copyWith(fontSize: 14, fontWeight: FontWeight.w600)),
                  Text(subtitle, style: AppType.caption.copyWith(color: AppColors.textFaint)),
                ],
              ),
            ),
            chip,
          ],
        ),
      ),
    );
  }
}
