import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';

/// Tela exibida quando este dispositivo é um filho vinculado.
/// Fase 2: mostrar lista de sites bloqueados e status do bloqueio ativo.
class ChildBlockedScreen extends ConsumerWidget {
  const ChildBlockedScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = ref.watch(authProvider);
    final session = auth is AuthChildSession ? auth : null;

    return Scaffold(
      appBar: AppBar(
        title: const Text('DopaBlocker — Filho'),
        actions: [
          IconButton(
            icon: const Icon(Icons.logout),
            tooltip: 'Desvincular',
            onPressed: () => ref.read(authProvider.notifier).logout(),
          ),
        ],
      ),
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.shield_outlined, size: 64, color: Colors.indigo),
            const SizedBox(height: 16),
            const Text(
              'Dispositivo vinculado',
              style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            if (session != null)
              Text(
                'ID: ${session.deviceId}',
                style: const TextStyle(color: Colors.black54, fontSize: 12),
              ),
            const SizedBox(height: 24),
            const Text(
              'Bloqueio ativo — gerenciado pelo responsável.',
              textAlign: TextAlign.center,
              style: TextStyle(color: Colors.black54),
            ),
          ],
        ),
      ),
    );
  }
}
