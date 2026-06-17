import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/preferences_provider.dart';
import '../theme.dart';
import '../widgets/setting_toggle_tile.dart';
import '../widgets/ui_kit.dart';

/// Aba "Ajustes" — conta, proteção, notificações e logout.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  Future<void> _confirmLogout(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: AppColors.surface,
        title: const Text('Sair da conta?'),
        content: const Text('Você precisará entrar novamente para gerenciar bloqueios.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancelar', style: TextStyle(color: AppColors.textSecondary)),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: FilledButton.styleFrom(backgroundColor: AppColors.danger),
            child: const Text('Sair'),
          ),
        ],
      ),
    );
    if (ok == true) await ref.read(authProvider.notifier).logout();
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    final prefs = ref.watch(preferencesProvider);
    final prefsNotifier = ref.read(preferencesProvider.notifier);
    final user = auth is AuthAuthenticated ? auth.user : null;

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 20,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: const [
            Text('CONTA',
                style: TextStyle(color: AppColors.textFaint, fontSize: 11, fontWeight: FontWeight.w700, letterSpacing: 1.4)),
            Text('Ajustes', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w700)),
          ],
        ),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 28),
        children: [
          // Conta
          AppCard(
            padding: const EdgeInsets.all(14),
            child: Row(
              children: [
                InitialBadge(user?.displayName ?? 'D', size: 48),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(user?.displayName ?? 'Convidado',
                          style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 16)),
                      const SizedBox(height: 2),
                      Text(user?.email ?? 'sem conta vinculada',
                          maxLines: 1, overflow: TextOverflow.ellipsis,
                          style: const TextStyle(color: AppColors.textFaint, fontSize: 13)),
                    ],
                  ),
                ),
                AppChip.accent('Pro'),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Proteção
          const SectionLabel('Proteção'),
          AppCard(
            child: Column(
              children: [
                SettingToggleTile(
                  title: 'Bloqueio rígido',
                  subtitle: 'Impede desativar sem senha',
                  value: prefs.strictBlocking,
                  onChanged: prefsNotifier.setStrictBlocking,
                ),
                const Divider(color: AppColors.divider, height: 1),
                SettingToggleTile(
                  title: 'Pausa de 5 min',
                  subtitle: 'Permite uma folga curta por dia',
                  value: prefs.pause5min,
                  onChanged: prefsNotifier.setPause5min,
                ),
                const Divider(color: AppColors.divider, height: 1),
                SettingToggleTile(
                  title: 'Tela de respiro',
                  subtitle: 'Mostra um lembrete ao abrir app bloqueado',
                  value: prefs.breathScreen,
                  onChanged: prefsNotifier.setBreathScreen,
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Notificações
          const SectionLabel('Notificações'),
          AppCard(
            child: Column(
              children: [
                SettingToggleTile(
                  title: 'Resumo diário',
                  value: prefs.dailySummary,
                  onChanged: prefsNotifier.setDailySummary,
                ),
                const Divider(color: AppColors.divider, height: 1),
                SettingToggleTile(
                  title: 'Alertas de tentativa',
                  value: prefs.attemptAlerts,
                  onChanged: prefsNotifier.setAttemptAlerts,
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Logout
          AppCard(
            onTap: () => _confirmLogout(context, ref),
            child: const Row(
              children: [
                Icon(Icons.logout, color: AppColors.danger, size: 20),
                SizedBox(width: 12),
                Text('Sair', style: TextStyle(color: AppColors.danger, fontWeight: FontWeight.w600, fontSize: 15)),
              ],
            ),
          ),
          const SizedBox(height: 16),
          const Center(
            child: Text('DopaBlocker · v0.2', style: TextStyle(color: AppColors.textFaint, fontSize: 12)),
          ),
        ],
      ),
    );
  }
}
