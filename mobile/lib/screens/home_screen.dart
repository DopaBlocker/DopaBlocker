import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/blocking_provider.dart';
import '../providers/device_event_provider.dart';
import '../providers/device_provider.dart';
import '../providers/nav_provider.dart';
import '../providers/permissions_provider.dart';
import '../theme.dart';
import '../widgets/ui_kit.dart';

/// Aba "Início" — hub de status honesto: estado real da proteção, camadas ativas
/// e (em conta parental) um resumo dos filhos. Sem estatísticas mock — só dado
/// que existe de fato no engine/backend.
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
          _ProtectionHero(blocking: blocking),
          const SizedBox(height: 24),
          const _LayersSection(),
          const SizedBox(height: 24),
          _SummarySection(blocking: blocking, isParental: isParental),
          if (isParental) ...[
            const SizedBox(height: 24),
            const _ChildrenSummary(),
          ],
          const SizedBox(height: 24),
          _ManageButton(),
        ],
      ),
    );
  }
}

// ── Hero: status de proteção (real) ─────────────────────────────────────────

class _ProtectionHero extends ConsumerWidget {
  final BlockingState blocking;
  const _ProtectionHero({required this.blocking});

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

// ── Camadas ativas (real) ───────────────────────────────────────────────────

class _LayersSection extends ConsumerWidget {
  const _LayersSection();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final blocking = ref.watch(blockingProvider);
    final perms = ref.watch(protectionPermissionsProvider);
    final on = blocking.isBlockingActive;

    // Chip do bloqueio de apps: depende de haver apps na lista + permissão.
    Widget appsChip() {
      if (blocking.appCount == 0) return const AppChip('0 apps');
      if (!perms.accessibilityEnabled) return AppChip.warning('ativar');
      return on ? AppChip.success('ativo') : const AppChip('pausado');
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionLabel('Camadas ativas'),
        AppCard(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
          child: Column(
            children: [
              _LayerRow(
                icon: Icons.public,
                title: 'Bloqueio de sites',
                subtitle: 'Sinkhole de DNS',
                chip: on ? AppChip.success('ativo') : const AppChip('pausado'),
              ),
              const _LayerDivider(),
              _LayerRow(
                icon: Icons.smartphone_outlined,
                title: 'Bloqueio de apps',
                subtitle: '${blocking.appCount} na lista',
                chip: appsChip(),
                onTap: () => ref.read(navIndexProvider.notifier).state = NavTab.bloqueios,
              ),
              const _LayerDivider(),
              _LayerRow(
                icon: Icons.shield_outlined,
                title: 'Filtro adulto',
                subtitle: 'Resolver de família',
                chip: blocking.isAdultFilterEnabled
                    ? AppChip.success('ativo')
                    : const AppChip('off'),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _LayerDivider extends StatelessWidget {
  const _LayerDivider();
  @override
  Widget build(BuildContext context) =>
      const Divider(color: AppColors.divider, height: 1, indent: 56, endIndent: 12);
}

class _LayerRow extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget chip;
  final VoidCallback? onTap;
  const _LayerRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.chip,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(12),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
        child: Row(
          children: [
            Icon(icon, size: 20, color: AppColors.textSecondary),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: AppType.body.copyWith(fontSize: 14, fontWeight: FontWeight.w600)),
                  Text(subtitle, style: AppType.caption.copyWith(color: AppColors.textFaint)),
                ],
              ),
            ),
            chip,
          ],
        ),
      ),
    );
  }
}

// ── Resumo (real) ───────────────────────────────────────────────────────────

class _SummarySection extends StatelessWidget {
  final BlockingState blocking;
  final bool isParental;
  const _SummarySection({required this.blocking, required this.isParental});

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

// ── Resumo dos filhos (parental, real) ──────────────────────────────────────

class _ChildrenSummary extends ConsumerWidget {
  const _ChildrenSummary();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final devices = ref.watch(deviceProvider).children;
    final alerts = ref.watch(deviceEventsProvider).events;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionLabel('Filhos'),
        AppCard(
          onTap: () => ref.read(navIndexProvider.notifier).state = NavTab.filhos,
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              const InitialBadge('F', icon: Icons.group_outlined),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('${devices.length} ${devices.length == 1 ? 'dispositivo' : 'dispositivos'}',
                        style: AppType.body.copyWith(fontSize: 15, fontWeight: FontWeight.w600)),
                    Text(
                      alerts.isEmpty
                          ? 'Nenhum alerta'
                          : '${alerts.length} ${alerts.length == 1 ? 'alerta' : 'alertas'} de adulteração',
                      style: AppType.caption.copyWith(
                        color: alerts.isEmpty ? AppColors.textFaint : AppColors.danger,
                      ),
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right, color: AppColors.textSecondary),
            ],
          ),
        ),
      ],
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
