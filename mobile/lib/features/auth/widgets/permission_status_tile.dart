import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Tile de status de uma permissão do sistema (acessibilidade / overlay):
/// mostra um chip "ativo"/"ativar" e, ao tocar, abre a tela do sistema.
class PermissionStatusTile extends StatelessWidget {
  final String title;
  final String subtitle;
  final bool granted;
  final VoidCallback onTap;

  const PermissionStatusTile({
    required this.title,
    required this.subtitle,
    required this.granted,
    required this.onTap,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: granted ? null : onTap,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: AppType.body.copyWith(fontWeight: FontWeight.w600)),
                  const SizedBox(height: 2),
                  Text(subtitle, style: AppType.caption.copyWith(color: AppColors.textFaint)),
                ],
              ),
            ),
            const SizedBox(width: 12),
            granted ? AppChip.success('ativo') : AppChip.warning('ativar'),
          ],
        ),
      ),
    );
  }
}
