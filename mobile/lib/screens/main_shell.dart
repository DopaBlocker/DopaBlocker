import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../providers/nav_provider.dart';
import '../providers/permissions_provider.dart';
import '../theme.dart';
import 'blocking_screen.dart';
import 'home_screen.dart';
import 'parental_screen.dart';
import 'settings_screen.dart';

/// Casca principal pós-login. Arquitetura de informação unificada com o desktop:
/// **Início · Bloqueios · Filhos (só parental) · Conta**. Mantém o estado das
/// abas via IndexedStack e a NavigationBar inferior.
class MainShell extends ConsumerStatefulWidget {
  const MainShell({super.key});

  @override
  ConsumerState<MainShell> createState() => _MainShellState();
}

/// Uma destinação da navegação principal.
class _NavDest {
  final String label;
  final IconData icon;
  final IconData activeIcon;
  final Widget screen;
  const _NavDest(this.label, this.icon, this.activeIcon, this.screen);
}

class _MainShellState extends ConsumerState<MainShell> with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // O usuário concede acessibilidade/overlay fora do app (Configurações) e
    // volta — re-checa o estado das permissões ao retomar.
    if (state == AppLifecycleState.resumed) {
      ref.read(protectionPermissionsProvider.notifier).refresh();
    }
  }

  /// Destinações conforme o modo da conta. "Filhos" só aparece em conta parental
  /// (paridade com o gating do desktop).
  List<_NavDest> _destinations(bool isParental) => [
        const _NavDest('Início', Icons.home_outlined, Icons.home_rounded, HomeScreen()),
        const _NavDest('Bloqueios', Icons.block_outlined, Icons.block_rounded, BlockingScreen()),
        if (isParental)
          const _NavDest('Filhos', Icons.group_outlined, Icons.group_rounded, ParentalScreen()),
        const _NavDest('Conta', Icons.person_outline, Icons.person_rounded, SettingsScreen()),
      ];

  @override
  Widget build(BuildContext context) {
    final auth = ref.watch(authProvider);
    final isParental = auth is AuthAuthenticated && auth.user.isParental;
    final dests = _destinations(isParental);

    // Clampa o índice (cobre a troca pessoal↔parental, que muda o nº de abas).
    final rawIndex = ref.watch(navIndexProvider);
    final index = rawIndex.clamp(0, dests.length - 1);

    return Scaffold(
      body: IndexedStack(index: index, children: [for (final d in dests) d.screen]),
      bottomNavigationBar: NavigationBarTheme(
        data: NavigationBarThemeData(
          backgroundColor: AppColors.surface,
          indicatorColor: AppColors.primaryDim,
          labelTextStyle: WidgetStateProperty.resolveWith(
            (states) => TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w600,
              color: states.contains(WidgetState.selected)
                  ? AppColors.textPrimary
                  : AppColors.textFaint,
            ),
          ),
          iconTheme: WidgetStateProperty.resolveWith(
            (states) => IconThemeData(
              color: states.contains(WidgetState.selected)
                  ? AppColors.primary
                  : AppColors.textFaint,
            ),
          ),
        ),
        child: NavigationBar(
          selectedIndex: index,
          height: 66,
          labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
          onDestinationSelected: (i) =>
              ref.read(navIndexProvider.notifier).state = i,
          destinations: [
            for (final d in dests)
              NavigationDestination(
                icon: Icon(d.icon),
                selectedIcon: Icon(d.activeIcon),
                label: d.label,
              ),
          ],
        ),
      ),
    );
  }
}
