import 'package:flutter/material.dart';

import '../theme.dart';

/// Container padrão de card arredondado do app.
class AppCard extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color? color;
  final VoidCallback? onTap;
  final Border? border;

  const AppCard({
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.color,
    this.onTap,
    this.border,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    final card = Container(
      width: double.infinity,
      padding: padding,
      decoration: BoxDecoration(
        color: color ?? AppColors.surface,
        borderRadius: BorderRadius.circular(18),
        border: border,
      ),
      child: child,
    );
    if (onTap == null) return card;
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: onTap,
        child: card,
      ),
    );
  }
}

/// Rótulo pequeno em maiúsculas usado acima das seções.
class SectionLabel extends StatelessWidget {
  final String text;
  final EdgeInsetsGeometry padding;

  const SectionLabel(this.text, {this.padding = const EdgeInsets.only(bottom: 10), super.key});

  @override
  Widget build(BuildContext context) => Padding(
        padding: padding,
        child: Text(
          text.toUpperCase(),
          style: const TextStyle(
            color: AppColors.textFaint,
            fontSize: 11,
            fontWeight: FontWeight.w700,
            letterSpacing: 1.3,
          ),
        ),
      );
}

/// Pílula compacta (chips de status / categoria).
class AppChip extends StatelessWidget {
  final String label;
  final Color color;
  final Color background;
  final IconData? icon;

  const AppChip(
    this.label, {
    this.color = AppColors.textSecondary,
    this.background = AppColors.surfaceHigh,
    this.icon,
    super.key,
  });

  /// Atalho para chip no tom de acento (roxo) — ex: "+18%", "Pro".
  factory AppChip.accent(String label) =>
      AppChip(label, color: AppColors.primary, background: AppColors.primaryDim);

  /// Atalho para chip verde de sucesso — ex: "ativo".
  factory AppChip.success(String label) =>
      AppChip(label, color: AppColors.success, background: AppColors.successDim);

  /// Atalho para chip âmbar — ex: "pausado", "Liber".
  factory AppChip.warning(String label) =>
      AppChip(label, color: AppColors.warning, background: AppColors.warningDim);

  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 4),
        decoration: BoxDecoration(
          color: background,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (icon != null) ...[
              Icon(icon, size: 12, color: color),
              const SizedBox(width: 4),
            ],
            Text(
              label,
              style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.w600),
            ),
          ],
        ),
      );
}

/// Avatar quadrado arredondado com inicial — usado em itens de lista.
class InitialBadge extends StatelessWidget {
  final String source;
  final IconData? icon;
  final double size;

  const InitialBadge(this.source, {this.icon, this.size = 40, super.key});

  static const _palette = [
    Color(0xFF7C5CFF),
    Color(0xFFEC4899),
    Color(0xFF2FD477),
    Color(0xFFE0A23B),
    Color(0xFF38BDF8),
    Color(0xFFF2545B),
  ];

  @override
  Widget build(BuildContext context) {
    final color = _palette[source.isEmpty ? 0 : source.codeUnitAt(0) % _palette.length];
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: color.withOpacity(0.18),
        borderRadius: BorderRadius.circular(12),
      ),
      alignment: Alignment.center,
      child: icon != null
          ? Icon(icon, size: size * 0.5, color: color)
          : Text(
              source.isEmpty ? '?' : source.substring(0, 1).toUpperCase(),
              style: TextStyle(color: color, fontWeight: FontWeight.w700, fontSize: size * 0.4),
            ),
    );
  }
}
