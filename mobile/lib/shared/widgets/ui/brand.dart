import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';

/// Glow de marca difuso e estático — halo radial sutil atrás dos heros
/// (welcome / child-blocked / login). Espelha o glow do desktop. Não anima
/// (a UX guideline desaconselha animação infinita decorativa).
class BrandGlow extends StatelessWidget {
  final double size;
  const BrandGlow({this.size = 320, super.key});

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: Container(
        width: size,
        height: size,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          gradient: RadialGradient(
            colors: [
              AppColors.primary.withValues(alpha: 0.12),
              AppColors.background.withValues(alpha: 0.0),
            ],
          ),
        ),
      ),
    );
  }
}

/// Marca do app — quadrado com gradiente azul→roxo + quadrado branco interno e
/// glow sutil. Usado nos heros (welcome / child-blocked).
class AppBrandMark extends StatelessWidget {
  final double size;
  const AppBrandMark({this.size = 56, super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        gradient: AppColors.brandGradient,
        borderRadius: BorderRadius.circular(AppRadii.lg),
        boxShadow: [
          BoxShadow(
            color: AppColors.primary.withValues(alpha: 0.35),
            blurRadius: size * 0.6,
            spreadRadius: 1,
          ),
        ],
      ),
      alignment: Alignment.center,
      child: Container(
        width: size * 0.4,
        height: size * 0.4,
        decoration: BoxDecoration(
          color: Colors.white.withValues(alpha: 0.92),
          borderRadius: BorderRadius.circular(size * 0.1),
        ),
      ),
    );
  }
}
