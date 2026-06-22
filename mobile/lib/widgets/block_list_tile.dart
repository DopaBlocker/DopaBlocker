import 'package:flutter/material.dart';

import '../models/blocked_item.dart';
import '../theme.dart';
import 'ui_kit.dart';

/// Linha de um item bloqueado na lista (modo pessoal/pais).
/// Mostra ícone, valor, tipo e um chip de status. Swipe para a esquerda remove.
class BlockListTile extends StatelessWidget {
  final BlockedItem item;
  final bool readOnly;
  final VoidCallback? onRemove;
  final VoidCallback? onTap;

  const BlockListTile({
    required this.item,
    this.readOnly = false,
    this.onRemove,
    this.onTap,
    super.key,
  });

  IconData get _icon => switch (item.itemType) {
        'app' => Icons.smartphone,
        'keyword' => Icons.tag,
        _ => Icons.public,
      };

  String get _typeLabel => switch (item.itemType) {
        'app' => 'Aplicativo',
        'keyword' => 'Palavra-chave',
        _ => 'Site',
      };

  Widget get _chip => switch (item.itemType) {
        'app' => const AppChip('app'),
        'keyword' => AppChip.warning('tema'),
        _ => const AppChip('site'),
      };

  @override
  Widget build(BuildContext context) {
    final tile = AppCard(
      onTap: onTap,
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      child: Row(
        children: [
          InitialBadge(item.value, icon: _icon),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  item.value,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: 15,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(_typeLabel, style: const TextStyle(color: AppColors.textFaint, fontSize: 12)),
              ],
            ),
          ),
          const SizedBox(width: 8),
          _chip,
        ],
      ),
    );

    if (readOnly || onRemove == null) return tile;

    return Dismissible(
      key: ValueKey(item.id),
      direction: DismissDirection.endToStart,
      onDismissed: (_) => onRemove!(),
      background: Container(
        alignment: Alignment.centerRight,
        padding: const EdgeInsets.only(right: 24),
        decoration: BoxDecoration(
          color: AppColors.danger.withValues(alpha: 0.15),
          borderRadius: BorderRadius.circular(18),
        ),
        child: const Icon(Icons.delete_outline, color: AppColors.danger),
      ),
      child: tile,
    );
  }
}
