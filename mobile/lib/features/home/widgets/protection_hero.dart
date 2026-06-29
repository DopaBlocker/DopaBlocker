import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/blocking/providers/blocking_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Hero de status (real) da aba Início: estado de proteção ligado/pausado com
/// tempo decorrido e o switch que liga/desliga o bloqueio.
class ProtectionHero extends ConsumerWidget {
  final BlockingState blocking;
  const ProtectionHero({required this.blocking, super.key});

  String _elapsed(DateTime since) {
    final d = DateTime.now().difference(since);
    final h = d.inHours;
    final m = d.inMinutes % 60;
    if (h > 0) return 'ativo há ${h}h ${m}min';
    return 'ativo há ${m}min';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final active = blocking.isBlockingActive;
    final color = active ? AppColors.success : AppColors.warning;
    final subtitle = active
        ? (blocking.activeSince != null
            ? _elapsed(blocking.activeSince!)
            : 'Proteção ligada')
        : 'Toque para reativar a proteção';

    return AppCard(
      highlight: active,
      padding: const EdgeInsets.all(20),
      child: Column(
        children: [
          AnimatedContainer(
            duration: AppDurations.enter,
            curve: AppCurves.out,
            width: 64,
            height: 64,
            decoration: BoxDecoration(
              color: color.withValues(alpha: 0.14),
              shape: BoxShape.circle,
            ),
            child: Icon(
              active ? Icons.shield_rounded : Icons.gpp_maybe_outlined,
              color: color,
              size: 32,
            ),
          ),
          const SizedBox(height: 14),
          Text(active ? 'Protegido' : 'Proteção pausada', style: AppType.h2),
          const SizedBox(height: 4),
          Text(subtitle, style: AppType.bodySm, textAlign: TextAlign.center),
          const SizedBox(height: 18),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
            decoration: BoxDecoration(
              color: AppColors.surfaceHigh,
              borderRadius: BorderRadius.circular(14),
            ),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Proteção neste aparelho',
                          style: AppType.body.copyWith(fontSize: 14, fontWeight: FontWeight.w600)),
                      Text(active ? 'Bloqueio em execução' : 'Bloqueio pausado',
                          style: AppType.caption.copyWith(color: AppColors.textFaint)),
                    ],
                  ),
                ),
                Switch(
                  value: active,
                  activeThumbColor: Colors.white,
                  activeTrackColor: AppColors.success,
                  inactiveThumbColor: AppColors.textSecondary,
                  inactiveTrackColor: AppColors.surface,
                  trackOutlineColor: WidgetStateProperty.all(Colors.transparent),
                  onChanged: (v) => ref.read(blockingProvider.notifier).toggleBlocking(v),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
