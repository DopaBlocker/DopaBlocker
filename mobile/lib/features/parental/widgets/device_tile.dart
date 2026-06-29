import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/models/device.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Linha de um dispositivo filho vinculado: ícone por plataforma, nome, status
/// e menu para desvincular.
class DeviceTile extends StatelessWidget {
  final Device device;
  final VoidCallback onRevoke;
  const DeviceTile({required this.device, required this.onRevoke, super.key});

  IconData get _icon {
    final name = device.deviceName.toLowerCase();
    if (device.platform == 'windows' || name.contains('notebook') || name.contains('pc')) {
      return Icons.laptop_mac;
    }
    if (name.contains('tablet')) return Icons.tablet_mac;
    return Icons.smartphone;
  }

  String get _platformLabel => switch (device.platform) {
        'windows' => 'Windows',
        'android' => 'Android',
        _ => device.platform,
      };

  @override
  Widget build(BuildContext context) {
    return AppCard(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      child: Row(
        children: [
          InitialBadge(device.deviceName, icon: _icon),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(device.deviceName,
                    maxLines: 1, overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
                const SizedBox(height: 2),
                Text(_platformLabel, style: const TextStyle(color: AppColors.textFaint, fontSize: 12)),
              ],
            ),
          ),
          AppChip.success('on'),
          const SizedBox(width: 4),
          PopupMenuButton<String>(
            icon: const Icon(Icons.more_vert, color: AppColors.textSecondary, size: 20),
            color: AppColors.surfaceHigh,
            onSelected: (v) {
              if (v == 'revoke') onRevoke();
            },
            itemBuilder: (_) => const [
              PopupMenuItem(
                value: 'revoke',
                child: Text('Desvincular', style: TextStyle(color: AppColors.danger)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
