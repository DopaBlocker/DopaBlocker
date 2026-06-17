import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/blocking_provider.dart';
import '../providers/stats_provider.dart';
import '../theme.dart';
import '../widgets/mini_bar_chart.dart';
import '../widgets/ui_kit.dart';

/// Aba "Início" — junta o status de proteção, os marcos do dia e o dashboard
/// do mês (os três mockups com "Início" selecionado).
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  String _hm(Duration d) {
    final h = d.inHours;
    final m = d.inMinutes % 60;
    if (h == 0) return '${m}min';
    if (m == 0) return '${h}h';
    return '${h}h ${m}min';
  }

  String _firstName(WidgetRef ref) {
    final auth = ref.read(authProvider);
    if (auth is AuthAuthenticated) {
      return auth.user.displayName.split(' ').first;
    }
    return '';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final blocking = ref.watch(blockingProvider);
    final stats = ref.watch(statsProvider);
    final name = _firstName(ref);

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('INÍCIO',
                style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.4)),
            Text(name.isEmpty ? 'Olá 👋' : 'Olá, $name',
                style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w700)),
          ],
        ),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
        children: [
          _ProtectionHero(blocking: blocking),
          const SizedBox(height: 24),
          _MarcosSection(stats: stats, fmt: _hm),
          const SizedBox(height: 24),
          _MonthSection(stats: stats, fmt: _hm),
        ],
      ),
    );
  }
}

// ── Hero: status de proteção ────────────────────────────────────────────────

class _ProtectionHero extends ConsumerWidget {
  final BlockingState blocking;
  const _ProtectionHero({required this.blocking});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final active = blocking.isBlockingActive;
    final color = active ? AppColors.success : AppColors.warning;
    final elapsed = active && blocking.activeSince != null
        ? DateTime.now().difference(blocking.activeSince!)
        : Duration.zero;
    final elapsedLabel = elapsed.inHours > 0
        ? '${elapsed.inHours}h ${elapsed.inMinutes % 60}min ativo'
        : '${elapsed.inMinutes}min ativo';

    return AppCard(
      padding: const EdgeInsets.all(20),
      child: Column(
        children: [
          Container(
            width: 64,
            height: 64,
            decoration: BoxDecoration(
              color: color.withOpacity(0.14),
              shape: BoxShape.circle,
            ),
            child: Icon(active ? Icons.check_rounded : Icons.warning_amber_rounded, color: color, size: 34),
          ),
          const SizedBox(height: 14),
          Text(active ? 'Tudo certo' : 'Proteção desligada',
              style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
          const SizedBox(height: 4),
          Text(
            active ? 'Protegido · $elapsedLabel' : 'Toque para reativar o bloqueio',
            style: const TextStyle(color: AppColors.textSecondary, fontSize: 13),
          ),
          const SizedBox(height: 18),
          // Toggle de proteção
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
            decoration: BoxDecoration(
              color: AppColors.surfaceHigh,
              borderRadius: BorderRadius.circular(14),
            ),
            child: Row(
              children: [
                const Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Proteção', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
                      Text('desligar exige senha',
                          style: TextStyle(color: AppColors.textFaint, fontSize: 12)),
                    ],
                  ),
                ),
                Switch(
                  value: active,
                  activeColor: Colors.white,
                  activeTrackColor: AppColors.success,
                  inactiveThumbColor: AppColors.textSecondary,
                  inactiveTrackColor: AppColors.surface,
                  trackOutlineColor: WidgetStateProperty.all(Colors.transparent),
                  onChanged: (v) {
                    if (!v) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(content: Text('Desligar exige senha (em breve)')),
                      );
                    }
                    ref.read(blockingProvider.notifier).toggleBlocking(v);
                  },
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          const Align(alignment: Alignment.centerLeft, child: SectionLabel('Camadas ativas')),
          _LayerRow(
            icon: Icons.shield_outlined,
            title: 'Filtro adulto',
            subtitle: '2,1M domínios',
            chip: blocking.isAdultFilterEnabled ? AppChip.success('ativo') : const AppChip('off'),
          ),
          const SizedBox(height: 8),
          _LayerRow(
            icon: Icons.center_focus_strong_outlined,
            title: 'Modo foco',
            subtitle: 'termina às 14h',
            chip: AppChip.warning('Liber'),
          ),
        ],
      ),
    );
  }
}

class _LayerRow extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget chip;
  const _LayerRow({required this.icon, required this.title, required this.subtitle, required this.chip});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
      decoration: BoxDecoration(
        color: AppColors.surfaceHigh,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        children: [
          Icon(icon, size: 20, color: AppColors.textSecondary),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14)),
                Text(subtitle, style: const TextStyle(color: AppColors.textFaint, fontSize: 12)),
              ],
            ),
          ),
          chip,
        ],
      ),
    );
  }
}

// ── Marcos (hoje + semana) ──────────────────────────────────────────────────

class _MarcosSection extends StatelessWidget {
  final StatsData stats;
  final String Function(Duration) fmt;
  const _MarcosSection({required this.stats, required this.fmt});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionLabel('Marcos'),
        AppCard(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('HOJE VOCÊ POUPOU',
                  style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.2)),
              const SizedBox(height: 6),
              Text(fmt(stats.savedToday),
                  style: const TextStyle(fontSize: 34, fontWeight: FontWeight.w800, letterSpacing: -1)),
              const SizedBox(height: 4),
              Text('${stats.interceptedToday} tentativas interceptadas',
                  style: const TextStyle(color: AppColors.textSecondary, fontSize: 13)),
            ],
          ),
        ),
        const SizedBox(height: 12),
        AppCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text('Semana', style: TextStyle(fontWeight: FontWeight.w600)),
                  AppChip.accent('+${stats.weekDeltaPct.toStringAsFixed(0)}%'),
                ],
              ),
              const SizedBox(height: 16),
              MiniBarChart(
                values: stats.weekBars,
                labels: stats.weekLabels,
                highlightIndex: stats.weekBars.length - 1,
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(child: _MiniStat(label: 'Sequência', value: '${stats.streakDays} dias')),
            const SizedBox(width: 12),
            Expanded(child: _MiniStat(label: 'Total', value: fmt(stats.totalSaved))),
          ],
        ),
      ],
    );
  }
}

class _MiniStat extends StatelessWidget {
  final String label;
  final String value;
  const _MiniStat({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return AppCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label.toUpperCase(),
              style: const TextStyle(color: AppColors.textFaint, fontSize: 10, fontWeight: FontWeight.w700, letterSpacing: 1)),
          const SizedBox(height: 6),
          Text(value, style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700)),
        ],
      ),
    );
  }
}

// ── Seu mês (dashboard detalhado) ───────────────────────────────────────────

class _MonthSection extends StatelessWidget {
  final StatsData stats;
  final String Function(Duration) fmt;
  const _MonthSection({required this.stats, required this.fmt});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SectionLabel('Seu mês'),
        AppCard(
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('VOCÊ RECUPEROU',
                      style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.2)),
                  AppChip.accent('+${stats.monthDeltaPct.toStringAsFixed(0)}%'),
                ],
              ),
              const SizedBox(height: 6),
              Text(fmt(stats.recoveredThisMonth),
                  style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w800, letterSpacing: -1)),
              const SizedBox(height: 2),
              const Text('vs. mês anterior',
                  style: TextStyle(color: AppColors.textSecondary, fontSize: 13)),
            ],
          ),
        ),
        const SizedBox(height: 12),
        AppCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SectionLabel('Por semana'),
              MiniBarChart(
                values: stats.monthBars,
                labels: stats.monthLabels,
                highlightIndex: stats.monthBars.length - 1,
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        AppCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SectionLabel('Mais bloqueados'),
              for (final t in stats.topBlocked) ...[
                _TopBlockedRow(item: t),
                if (t != stats.topBlocked.last) const SizedBox(height: 12),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

class _TopBlockedRow extends StatelessWidget {
  final TopBlocked item;
  const _TopBlockedRow({required this.item});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(item.domain, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
            Text('${(item.fraction * 100).round()}%',
                style: const TextStyle(color: AppColors.textSecondary, fontSize: 13, fontWeight: FontWeight.w600)),
          ],
        ),
        const SizedBox(height: 6),
        ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: LinearProgressIndicator(
            value: item.fraction,
            minHeight: 6,
            backgroundColor: AppColors.surfaceHigh,
            valueColor: const AlwaysStoppedAnimation(AppColors.primary),
          ),
        ),
      ],
    );
  }
}
