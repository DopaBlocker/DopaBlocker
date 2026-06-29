import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/parental/providers/device_event_provider.dart';
import 'package:dopablocker_mobile/shared/models/device_event.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Painel de alertas de adulteração (C2.1): mostra quando um filho desligou a
/// VPN ou abriu as Configs de VPN/DNS. Entrega in-app (sem push nesta fase).
class AlertsCard extends ConsumerWidget {
  const AlertsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final events = ref.watch(deviceEventsProvider).events;

    return AppCard(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Icon(Icons.warning_amber_rounded, size: 16, color: AppColors.warning),
              const SizedBox(width: 6),
              const Text('ALERTAS',
                  style: TextStyle(
                      color: AppColors.textFaint,
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 1.2)),
              const Spacer(),
              if (events.isNotEmpty)
                AppChip('${events.length}', color: AppColors.danger),
            ],
          ),
          const SizedBox(height: 12),
          if (events.isEmpty)
            const Text('Nenhum alerta. A proteção dos filhos está intacta.',
                style: TextStyle(color: AppColors.textSecondary, fontSize: 13))
          else
            for (final e in events.take(5)) ...[
              _AlertRow(event: e),
              const SizedBox(height: 8),
            ],
        ],
      ),
    );
  }
}

class _AlertRow extends StatelessWidget {
  final DeviceEvent event;
  const _AlertRow({required this.event});

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Padding(
          padding: EdgeInsets.only(top: 2),
          child: Icon(Icons.shield_moon_outlined, size: 16, color: AppColors.danger),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(event.label,
                  style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
              Text(event.createdAt,
                  style: const TextStyle(color: AppColors.textFaint, fontSize: 11)),
            ],
          ),
        ),
      ],
    );
  }
}
