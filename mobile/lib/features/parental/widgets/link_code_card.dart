import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/parental/providers/device_provider.dart';
import 'package:dopablocker_mobile/features/parental/widgets/countdown_text.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Card do código de vinculação parental: gera o código (TTL 5 min com
/// countdown), exibe os 6 dígitos e permite copiar/regenerar.
class LinkCodeCard extends ConsumerWidget {
  final DeviceState state;
  const LinkCodeCard({required this.state, super.key});

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
                AppButton(
                  label: 'Gerar código',
                  icon: Icons.add_link,
                  loading: state.isGenerating,
                  onPressed: () => ref.read(deviceProvider.notifier).generateLinkCode(),
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
                        style: AppType.mono(size: 26, weight: FontWeight.w800, color: AppColors.primary),
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(height: 12),
            Center(
              child: AppButton(
                label: 'Gerar novo',
                icon: Icons.refresh,
                variant: AppButtonVariant.ghost,
                fullWidth: false,
                onPressed: () => ref.read(deviceProvider.notifier).generateLinkCode(),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
