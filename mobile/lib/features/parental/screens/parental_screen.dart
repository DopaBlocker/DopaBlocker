import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/parental/providers/device_provider.dart';
import 'package:dopablocker_mobile/features/parental/widgets/alerts_card.dart';
import 'package:dopablocker_mobile/features/parental/widgets/device_tile.dart';
import 'package:dopablocker_mobile/features/parental/widgets/link_code_card.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Aba "Pais" — código de vinculação e lista de dispositivos filhos. Os blocos
/// (código, alertas, tile de dispositivo) vivem em `widgets/`.
class ParentalScreen extends ConsumerWidget {
  const ParentalScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(deviceProvider);
    final children = state.children;

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('MODO PAIS',
                style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.4)),
            const Text('Filhos vinculados', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w700)),
          ],
        ),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
        children: [
          LinkCodeCard(state: state),
          const SizedBox(height: 24),
          const AlertsCard(),
          const SizedBox(height: 24),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              SectionLabel('${children.length} dispositivos'),
              const SizedBox.shrink(),
            ],
          ),
          if (children.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16),
              child: AppEmptyState(
                icon: Icons.devices_other,
                title: 'Nenhum filho vinculado',
                description: 'Gere um código acima e peça para o filho digitar no app dele.',
              ),
            )
          else
            for (final d in children) ...[
              DeviceTile(device: d, onRevoke: () => ref.read(deviceProvider.notifier).revoke(d.id)),
              const SizedBox(height: 10),
            ],
        ],
      ),
    );
  }
}
