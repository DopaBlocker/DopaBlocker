import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Um item da lista "mais bloqueados".
class TopBlocked {
  final String domain;
  final double fraction; // 0..1
  const TopBlocked(this.domain, this.fraction);
}

/// Estatísticas de progresso exibidas nas seções Marcos / Seu mês.
///
/// NOTA: o backend ainda não tem telemetria de bloqueios (DECISOES_E_ROADMAP.md
/// F8 — "Sem painel de estatísticas"). Estes valores são representativos/locais até que
/// uma tabela de eventos exista. Quando F8 for implementado, trocar este
/// Provider por um StateNotifier que consome o endpoint real.
class StatsData {
  final Duration savedToday;
  final int interceptedToday;
  final double weekDeltaPct;
  final int streakDays;
  final Duration totalSaved;
  final Duration recoveredThisMonth;
  final double monthDeltaPct;
  final List<double> weekBars;
  final List<String> weekLabels;
  final List<double> monthBars;
  final List<String> monthLabels;
  final List<TopBlocked> topBlocked;

  const StatsData({
    required this.savedToday,
    required this.interceptedToday,
    required this.weekDeltaPct,
    required this.streakDays,
    required this.totalSaved,
    required this.recoveredThisMonth,
    required this.monthDeltaPct,
    required this.weekBars,
    required this.weekLabels,
    required this.monthBars,
    required this.monthLabels,
    required this.topBlocked,
  });
}

final statsProvider = Provider<StatsData>((ref) {
  return const StatsData(
    savedToday: Duration(hours: 2, minutes: 14),
    interceptedToday: 23,
    weekDeltaPct: 18,
    streakDays: 12,
    totalSaved: Duration(hours: 38),
    recoveredThisMonth: Duration(hours: 47, minutes: 30),
    monthDeltaPct: 30,
    weekBars: [0.32, 0.45, 0.4, 0.62, 0.55, 0.8, 1.0],
    weekLabels: ['S', 'T', 'Q', 'Q', 'S', 'S', 'D'],
    monthBars: [0.5, 0.72, 0.6, 0.92],
    monthLabels: ['S1', 'S2', 'S3', 'S4'],
    topBlocked: [
      TopBlocked('instagram.com', 0.38),
      TopBlocked('youtube.com', 0.24),
      TopBlocked('x.com', 0.18),
    ],
  );
});
