import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Preferências locais da tela de Ajustes (em memória nesta fase; persistir em
/// SharedPreferences/SQLCipher na Fase M2).
class AppPreferences {
  final bool strictBlocking;
  final bool pause5min;
  final bool breathScreen;
  final bool dailySummary;
  final bool attemptAlerts;

  const AppPreferences({
    this.strictBlocking = true,
    this.pause5min = false,
    this.breathScreen = true,
    this.dailySummary = true,
    this.attemptAlerts = false,
  });

  AppPreferences copyWith({
    bool? strictBlocking,
    bool? pause5min,
    bool? breathScreen,
    bool? dailySummary,
    bool? attemptAlerts,
  }) =>
      AppPreferences(
        strictBlocking: strictBlocking ?? this.strictBlocking,
        pause5min: pause5min ?? this.pause5min,
        breathScreen: breathScreen ?? this.breathScreen,
        dailySummary: dailySummary ?? this.dailySummary,
        attemptAlerts: attemptAlerts ?? this.attemptAlerts,
      );
}

final preferencesProvider =
    StateNotifierProvider<PreferencesNotifier, AppPreferences>(
  (ref) => PreferencesNotifier(),
);

class PreferencesNotifier extends StateNotifier<AppPreferences> {
  PreferencesNotifier() : super(const AppPreferences());

  void setStrictBlocking(bool v) => state = state.copyWith(strictBlocking: v);
  void setPause5min(bool v) => state = state.copyWith(pause5min: v);
  void setBreathScreen(bool v) => state = state.copyWith(breathScreen: v);
  void setDailySummary(bool v) => state = state.copyWith(dailySummary: v);
  void setAttemptAlerts(bool v) => state = state.copyWith(attemptAlerts: v);
}
