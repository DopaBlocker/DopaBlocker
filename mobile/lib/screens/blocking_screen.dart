import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/blocking_provider.dart';
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

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('BLOQUEADOR',
                style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.4)),
            const Text('Sua lista', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w700)),
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
                // Filtro adulto (master)
                AppCard(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  child: Row(
                    children: [
                      const InitialBadge('A', icon: Icons.shield_outlined),
                      const SizedBox(width: 12),
                      const Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('Filtro adulto', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
                            Text('2,1M domínios', style: TextStyle(color: AppColors.textFaint, fontSize: 12)),
                          ],
                        ),
                      ),
                      Switch(
                        value: state.isAdultFilterEnabled,
                        activeColor: Colors.white,
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
                    Text('${state.activeCount} ativos',
                        style: const TextStyle(color: AppColors.textFaint, fontSize: 12)),
                  ],
                ),
                if (state.items.isEmpty)
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 48),
                    child: Center(
                      child: Text('Nenhum item bloqueado ainda.',
                          style: TextStyle(color: AppColors.textFaint)),
                    ),
                  )
                else
                  for (final item in state.items) ...[
                    BlockListTile(
                      item: item,
                      readOnly: readOnly,
                      onTap: readOnly ? null : () => ref.read(blockingProvider.notifier).toggleItemActive(item),
                      onRemove: readOnly ? null : () => ref.read(blockingProvider.notifier).removeItem(item.id),
                    ),
                    const SizedBox(height: 10),
                  ],
              ],
            ),
    );
  }
}
