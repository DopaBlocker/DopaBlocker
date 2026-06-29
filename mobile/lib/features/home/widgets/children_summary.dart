import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/home/providers/nav_provider.dart';
import 'package:dopablocker_mobile/features/parental/providers/device_event_provider.dart';
import 'package:dopablocker_mobile/features/parental/providers/device_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Resumo dos filhos (real, só conta parental) na aba Início: nº de dispositivos
/// e contagem de alertas de adulteração; toca para abrir a aba Filhos.
class ChildrenSummary extends ConsumerWidget {
  const ChildrenSummary({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final devices = ref.watch(deviceProvider).children;
    final alerts = ref.watch(deviceEventsProvider).events;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionLabel('Filhos'),
        AppCard(
          onTap: () => ref.read(navIndexProvider.notifier).state = NavTab.filhos,
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              const InitialBadge('F', icon: Icons.group_outlined),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('${devices.length} ${devices.length == 1 ? 'dispositivo' : 'dispositivos'}',
                        style: AppType.body.copyWith(fontSize: 15, fontWeight: FontWeight.w600)),
                    Text(
                      alerts.isEmpty
                          ? 'Nenhum alerta'
                          : '${alerts.length} ${alerts.length == 1 ? 'alerta' : 'alertas'} de adulteração',
                      style: AppType.caption.copyWith(
                        color: alerts.isEmpty ? AppColors.textFaint : AppColors.danger,
                      ),
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right, color: AppColors.textSecondary),
            ],
          ),
        ),
      ],
    );
  }
}
