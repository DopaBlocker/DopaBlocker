import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/routes.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

class WelcomeScreen extends StatelessWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Stack(
          alignment: Alignment.topCenter,
          children: [
            const Positioned(top: 40, child: BrandGlow()),
            Padding(
          padding: const EdgeInsets.all(AppSpacing.x6),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Spacer(),
              const Center(child: AppBrandMark(size: 64)),
              const SizedBox(height: AppSpacing.x5),
              Text('DopaBlocker', textAlign: TextAlign.center, style: AppType.display),
              const SizedBox(height: AppSpacing.x2),
              Text(
                'Como você vai usar?',
                textAlign: TextAlign.center,
                style: AppType.body.copyWith(color: AppColors.textSecondary),
              ),
              const Spacer(),
              _ModeCard(
                title: 'Pessoal',
                subtitle: 'Bloqueio para mim mesmo',
                icon: Icons.person_outline,
                onTap: () =>
                    Navigator.pushNamed(context, AppRoutes.login, arguments: 'personal'),
              ),
              const SizedBox(height: AppSpacing.x3),
              _ModeCard(
                title: 'Pais',
                subtitle: 'Gerenciar bloqueios dos filhos',
                icon: Icons.family_restroom,
                onTap: () =>
                    Navigator.pushNamed(context, AppRoutes.login, arguments: 'parental'),
              ),
              const SizedBox(height: AppSpacing.x3),
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
          ],
        ),
      ),
    );
  }
}

/// Card de escolha de modo — ícone em recorte + título + subtítulo + chevron.
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
    return AppCard(
      onTap: onTap,
      padding: const EdgeInsets.all(AppSpacing.x4),
      child: Row(
        children: [
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: AppColors.primaryDim,
              borderRadius: BorderRadius.circular(AppRadii.avatar),
            ),
            alignment: Alignment.center,
            child: Icon(icon, color: AppColors.primary, size: 22),
          ),
          const SizedBox(width: AppSpacing.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: AppType.title),
                const SizedBox(height: 2),
                Text(subtitle, style: AppType.bodySm),
              ],
            ),
          ),
          const Icon(Icons.chevron_right, color: AppColors.textFaint),
        ],
      ),
    );
  }
}
