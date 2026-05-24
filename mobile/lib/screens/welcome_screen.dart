import 'package:flutter/material.dart';

import '../routes.dart';

class WelcomeScreen extends StatelessWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Spacer(),
              const Text(
                'DopaBlocker',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 32, fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 8),
              const Text(
                'Como você vai usar?',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 16, color: Colors.black54),
              ),
              const Spacer(),
              _ModeCard(
                title: 'Pessoal',
                subtitle: 'Bloqueio para mim mesmo',
                icon: Icons.person_outline,
                onTap: () => Navigator.pushNamed(context, AppRoutes.login, arguments: 'personal'),
              ),
              const SizedBox(height: 12),
              _ModeCard(
                title: 'Pais',
                subtitle: 'Gerenciar bloqueios dos filhos',
                icon: Icons.family_restroom,
                onTap: () => Navigator.pushNamed(context, AppRoutes.login, arguments: 'parental'),
              ),
              const SizedBox(height: 12),
              _ModeCard(
                title: 'Filhos',
                subtitle: 'Inserir código do responsável',
                icon: Icons.child_care,
                onTap: () => Navigator.pushNamed(context, AppRoutes.childCode),
              ),
              const Spacer(),
            ],
          ),
        ),
      ),
    );
  }
}

class _ModeCard extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;
  final VoidCallback onTap;

  const _ModeCard({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: Icon(icon, size: 32, color: Theme.of(context).colorScheme.primary),
        title: Text(title, style: const TextStyle(fontWeight: FontWeight.bold)),
        subtitle: Text(subtitle),
        trailing: const Icon(Icons.chevron_right),
        onTap: onTap,
      ),
    );
  }
}
