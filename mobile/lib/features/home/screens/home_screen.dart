import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/features/auth/providers/auth_provider.dart';
import 'package:dopablocker_mobile/features/blocking/providers/blocking_provider.dart';
import 'package:dopablocker_mobile/features/home/providers/nav_provider.dart';
import 'package:dopablocker_mobile/features/home/widgets/children_summary.dart';
import 'package:dopablocker_mobile/features/home/widgets/layers_section.dart';
import 'package:dopablocker_mobile/features/home/widgets/protection_hero.dart';
import 'package:dopablocker_mobile/features/home/widgets/summary_section.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Aba "Início" — hub de status honesto: estado real da proteção, camadas ativas
/// e (em conta parental) um resumo dos filhos. Sem estatísticas mock — só dado
/// que existe de fato no engine/backend. Os blocos vivem em `widgets/`.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  String _firstName(WidgetRef ref) {
    final auth = ref.read(authProvider);
    if (auth is AuthAuthenticated) {
      final first = auth.user.displayName.split(' ').first;
      return first.isEmpty ? '' : first;
    }
    return '';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final blocking = ref.watch(blockingProvider);
    final auth = ref.watch(authProvider);
    final isParental = auth is AuthAuthenticated && auth.user.isParental;
    final name = _firstName(ref);

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('INÍCIO', style: AppType.label),
            Text(name.isEmpty ? 'Olá' : 'Olá, $name', style: AppType.h1),
          ],
        ),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
        children: [
          ProtectionHero(blocking: blocking),
          const SizedBox(height: 24),
          const LayersSection(),
          const SizedBox(height: 24),
          SummarySection(blocking: blocking, isParental: isParental),
          if (isParental) ...[
            const SizedBox(height: 24),
            const ChildrenSummary(),
          ],
          const SizedBox(height: 24),
          _ManageButton(),
        ],
      ),
    );
  }
}

// ── Ação rápida ─────────────────────────────────────────────────────────────

class _ManageButton extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return AppButton(
      label: 'Gerenciar bloqueios',
      icon: Icons.tune,
      variant: AppButtonVariant.secondary,
      onPressed: () => ref.read(navIndexProvider.notifier).state = NavTab.bloqueios,
    );
  }
}
