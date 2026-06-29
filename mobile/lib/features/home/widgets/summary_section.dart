import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/features/blocking/providers/blocking_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Resumo (real) da aba Início: itens bloqueados e modo da conta.
class SummarySection extends StatelessWidget {
  final BlockingState blocking;
  final bool isParental;
  const SummarySection({required this.blocking, required this.isParental, super.key});

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: _MiniStat(
            label: 'Itens bloqueados',
            value: '${blocking.activeCount}',
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: _MiniStat(
            label: 'Modo',
            value: isParental ? 'Pais' : 'Pessoal',
            mono: false,
          ),
        ),
      ],
    );
  }
}

class _MiniStat extends StatelessWidget {
  final String label;
  final String value;
  final bool mono;
  const _MiniStat({required this.label, required this.value, this.mono = true});

  @override
  Widget build(BuildContext context) {
    return AppCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label.toUpperCase(), style: AppType.label),
          const SizedBox(height: 6),
          Text(
            value,
            style: mono
                ? AppType.mono(size: 22, weight: FontWeight.w700)
                : AppType.h2,
          ),
        ],
      ),
    );
  }
}
