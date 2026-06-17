import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/device.dart';
import '../providers/device_provider.dart';
import '../theme.dart';
import '../widgets/countdown_text.dart';
import '../widgets/ui_kit.dart';

/// Aba "Pais" — código de vinculação e lista de dispositivos filhos.
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
          _LinkCodeCard(state: state),
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
              padding: EdgeInsets.symmetric(vertical: 32),
              child: Center(
                child: Text('Nenhum filho vinculado ainda.',
                    style: TextStyle(color: AppColors.textFaint)),
              ),
            )
          else
            for (final d in children) ...[
              _DeviceTile(device: d, onRevoke: () => ref.read(deviceProvider.notifier).revoke(d.id)),
              const SizedBox(height: 10),
            ],
        ],
      ),
    );
  }
}

class _LinkCodeCard extends ConsumerWidget {
  final DeviceState state;
  const _LinkCodeCard({required this.state});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final code = state.linkCode;
    final expires = state.linkCodeExpiresAt;

    return AppCard(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('CÓDIGO DE VINCULAÇÃO',
                  style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.2)),
              if (code != null && expires != null)
                Row(
                  children: [
                    const Icon(Icons.timer_outlined, size: 13, color: AppColors.warning),
                    const SizedBox(width: 4),
                    CountdownText(
                      expiresAt: expires,
                      style: const TextStyle(color: AppColors.warning, fontSize: 12, fontWeight: FontWeight.w600),
                    ),
                  ],
                ),
            ],
          ),
          const SizedBox(height: 16),
          if (code == null)
            Column(
              children: [
                const Text('Gere um código e digite-o no app do filho.',
                    textAlign: TextAlign.center,
                    style: TextStyle(color: AppColors.textSecondary, fontSize: 13)),
                const SizedBox(height: 16),
                FilledButton.icon(
                  onPressed: state.isGenerating
                      ? null
                      : () => ref.read(deviceProvider.notifier).generateLinkCode(),
                  style: FilledButton.styleFrom(
                    backgroundColor: AppColors.primary,
                    padding: const EdgeInsets.symmetric(vertical: 14),
                  ),
                  icon: state.isGenerating
                      ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                      : const Icon(Icons.add_link),
                  label: const Text('Gerar código'),
                ),
              ],
            )
          else ...[
            GestureDetector(
              onTap: () {
                Clipboard.setData(ClipboardData(text: code));
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Código copiado!')),
                );
              },
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  for (var i = 0; i < 6; i++)
                    Container(
                      width: 44,
                      height: 56,
                      margin: const EdgeInsets.symmetric(horizontal: 3),
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        color: AppColors.surfaceInput,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Text(
                        i < code.length ? code[i] : '',
                        style: const TextStyle(fontSize: 26, fontWeight: FontWeight.w800, color: AppColors.primary),
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(height: 12),
            Center(
              child: TextButton.icon(
                onPressed: () => ref.read(deviceProvider.notifier).generateLinkCode(),
                icon: const Icon(Icons.refresh, size: 16, color: AppColors.textSecondary),
                label: const Text('Gerar novo', style: TextStyle(color: AppColors.textSecondary)),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _DeviceTile extends StatelessWidget {
  final Device device;
  final VoidCallback onRevoke;
  const _DeviceTile({required this.device, required this.onRevoke});

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
