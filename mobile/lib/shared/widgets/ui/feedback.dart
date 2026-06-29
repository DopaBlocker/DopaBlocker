import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';

/// Placeholder de carregamento com shimmer (>300ms). Respeita reduced-motion.
class AppSkeleton extends StatefulWidget {
  final double? width;
  final double height;
  final double radius;

  const AppSkeleton({this.width, this.height = 16, this.radius = AppRadii.sm, super.key});

  @override
  State<AppSkeleton> createState() => _AppSkeletonState();
}

class _AppSkeletonState extends State<AppSkeleton>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c =
      AnimationController(vsync: this, duration: const Duration(milliseconds: 1400))
        ..repeat();

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.maybeOf(context)?.disableAnimations ?? false;
    final box = ClipRRect(
      borderRadius: BorderRadius.circular(widget.radius),
      child: Container(
        width: widget.width,
        height: widget.height,
        color: AppColors.surfaceHigh,
      ),
    );
    if (reduce) return box;
    return AnimatedBuilder(
      animation: _c,
      builder: (context, child) => ShaderMask(
        shaderCallback: (rect) {
          final dx = rect.width * (_c.value * 2 - 1);
          return LinearGradient(
            colors: const [AppColors.surfaceHigh, AppColors.surfaceHover, AppColors.surfaceHigh],
            stops: const [0.35, 0.5, 0.65],
            transform: _SlideGradient(dx),
          ).createShader(rect);
        },
        child: child,
      ),
      child: box,
    );
  }
}

class _SlideGradient extends GradientTransform {
  final double dx;
  const _SlideGradient(this.dx);
  @override
  Matrix4 transform(Rect bounds, {TextDirection? textDirection}) =>
      Matrix4.translationValues(dx, 0, 0);
}

/// Estado vazio padrão: ícone + título + descrição + ação opcional.
class AppEmptyState extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? description;
  final Widget? action;

  const AppEmptyState({
    required this.icon,
    required this.title,
    this.description,
    this.action,
    super.key,
  });

  @override
  Widget build(BuildContext context) => Center(
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.x6),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 56,
                height: 56,
                decoration: BoxDecoration(
                  color: AppColors.surfaceHigh,
                  borderRadius: BorderRadius.circular(AppRadii.lg),
                ),
                child: Icon(icon, color: AppColors.textSecondary, size: 26),
              ),
              const SizedBox(height: AppSpacing.x3),
              Text(title, style: AppType.title, textAlign: TextAlign.center),
              if (description != null) ...[
                const SizedBox(height: AppSpacing.x1),
                Text(description!,
                    style: AppType.bodySm, textAlign: TextAlign.center),
              ],
              if (action != null) ...[
                const SizedBox(height: AppSpacing.x4),
                action!,
              ],
            ],
          ),
        ),
      );
}
