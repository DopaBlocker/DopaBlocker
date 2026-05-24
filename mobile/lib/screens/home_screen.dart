import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../routes.dart';

/// Dashboard pós-login. Fase 2: integrar BlockingProvider e mostrar estado real.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    final user = auth is AuthAuthenticated ? auth.user : null;

    return Scaffold(
      appBar: AppBar(
        title: const Text('DopaBlocker'),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings_outlined),
            onPressed: () => Navigator.pushNamed(context, AppRoutes.settings),
          ),
        ],
      ),
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (user != null)
              Text('Olá, ${user.displayName}', style: const TextStyle(fontSize: 18)),
            const SizedBox(height: 8),
            const Text('Bloqueio — em construção', style: TextStyle(color: Colors.black54)),
          ],
        ),
      ),
      bottomNavigationBar: NavigationBar(
        destinations: [
          const NavigationDestination(icon: Icon(Icons.home_outlined), label: 'Início'),
          const NavigationDestination(icon: Icon(Icons.block_outlined), label: 'Bloqueios'),
          if (user?.isParental == true)
            const NavigationDestination(icon: Icon(Icons.family_restroom), label: 'Filhos'),
          const NavigationDestination(icon: Icon(Icons.settings_outlined), label: 'Config'),
        ],
        onDestinationSelected: (i) {
          final routes = [
            AppRoutes.home,
            AppRoutes.blocking,
            if (user?.isParental == true) AppRoutes.parental,
            AppRoutes.settings,
          ];
          if (i < routes.length) Navigator.pushNamed(context, routes[i]);
        },
        selectedIndex: 0,
      ),
    );
  }
}
