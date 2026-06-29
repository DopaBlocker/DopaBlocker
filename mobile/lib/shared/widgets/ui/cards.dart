import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';

/// Container padrão de card arredondado do app.
class AppCard extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color? color;
  final VoidCallback? onTap;
  final Border? border;
  final bool highlight;

  const AppCard({
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.color,
    this.onTap,
    this.border,
    this.highlight = false,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    final base = Container(
      width: double.infinity,
      padding: padding,
      decoration: BoxDecoration(
        color: color ?? AppColors.surface,
        borderRadius: AppRadii.cardR,
        border: border ?? const Border(top: BorderSide(color: AppColors.hairline)),
        boxShadow: const [
          BoxShadow(color: Color(0x80000000), blurRadius: 8, offset: Offset(0, 2)),
        ],
      ),
      child: child,
    );

    final card = highlight
        ? Container(
            padding: const EdgeInsets.all(1),
            decoration: BoxDecoration(
              gradient: AppColors.brandGradient,
              borderRadius: AppRadii.cardR,
            ),
            child: base,
          )
        : base;

    if (onTap == null) return card;
    return _PressableCard(onTap: onTap!, child: card);
  }
}

/// Card tappável com feedback de pressão (scale 0.98 + ripple). Mantém o ripple
/// via InkWell e adiciona o scale via `onHighlightChanged`. Respeita
/// reduced-motion (sem scale).
class _PressableCard extends StatefulWidget {
  final Widget child;
  final VoidCallback onTap;
  const _PressableCard({required this.child, required this.onTap});

  @override
  State<_PressableCard> createState() => _PressableCardState();
}

class _PressableCardState extends State<_PressableCard> {
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.maybeOf(context)?.disableAnimations ?? false;
    final inner = Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: AppRadii.cardR,
        onTap: widget.onTap,
        onHighlightChanged: reduce ? null : (h) => setState(() => _pressed = h),
        child: widget.child,
      ),
    );
    if (reduce) return inner;
    return AnimatedScale(
      scale: _pressed ? 0.98 : 1.0,
      duration: AppDurations.micro,
      curve: AppCurves.out,
      child: inner,
    );
  }
}
