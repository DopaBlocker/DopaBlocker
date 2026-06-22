import 'package:flutter/material.dart';

import '../theme.dart';

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

  static const _palette = AppColors.categorical;

  @override
  Widget build(BuildContext context) {
    final color = _palette[source.isEmpty ? 0 : source.codeUnitAt(0) % _palette.length];
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.18),
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

/// Variantes do botão padrão do app.
enum AppButtonVariant { primary, secondary, ghost, danger }

/// Botão padrão: primário com gradiente de marca, mais secundário/ghost/danger.
/// Press com scale 0.97; estado loading mostra spinner e desabilita.
class AppButton extends StatefulWidget {
  final String label;
  final VoidCallback? onPressed;
  final AppButtonVariant variant;
  final IconData? icon;
  final bool loading;
  final bool fullWidth;

  const AppButton({
    required this.label,
    this.onPressed,
    this.variant = AppButtonVariant.primary,
    this.icon,
    this.loading = false,
    this.fullWidth = true,
    super.key,
  });

  @override
  State<AppButton> createState() => _AppButtonState();
}

class _AppButtonState extends State<AppButton> {
  bool _pressed = false;

  bool get _enabled => widget.onPressed != null && !widget.loading;

  @override
  Widget build(BuildContext context) {
    final reduceMotion = MediaQuery.maybeOf(context)?.disableAnimations ?? false;

    final fg = switch (widget.variant) {
      AppButtonVariant.primary => Colors.white,
      AppButtonVariant.danger => Colors.white,
      AppButtonVariant.secondary => AppColors.textPrimary,
      AppButtonVariant.ghost => AppColors.textSecondary,
    };

    final Widget content = Row(
      mainAxisSize: widget.fullWidth ? MainAxisSize.max : MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        if (widget.loading)
          SizedBox(
            width: 18,
            height: 18,
            child: CircularProgressIndicator(strokeWidth: 2, color: fg),
          )
        else ...[
          if (widget.icon != null) ...[
            Icon(widget.icon, size: 18, color: fg),
            const SizedBox(width: AppSpacing.x2),
          ],
          Text(widget.label,
              style: AppType.title.copyWith(color: fg, fontSize: 15)),
        ],
      ],
    );

    final decoration = switch (widget.variant) {
      AppButtonVariant.primary => BoxDecoration(
          gradient: AppColors.brandGradient, borderRadius: AppRadii.controlR),
      AppButtonVariant.danger => BoxDecoration(
          color: AppColors.danger, borderRadius: AppRadii.controlR),
      AppButtonVariant.secondary => BoxDecoration(
          color: AppColors.surfaceHigh,
          borderRadius: AppRadii.controlR,
          border: Border.all(color: AppColors.border)),
      AppButtonVariant.ghost => BoxDecoration(borderRadius: AppRadii.controlR),
    };

    return Opacity(
      opacity: _enabled ? 1 : 0.5,
      child: GestureDetector(
        onTapDown: _enabled ? (_) => setState(() => _pressed = true) : null,
        onTapCancel: _enabled ? () => setState(() => _pressed = false) : null,
        onTapUp: _enabled ? (_) => setState(() => _pressed = false) : null,
        onTap: _enabled ? widget.onPressed : null,
        child: AnimatedScale(
          scale: (_pressed && !reduceMotion) ? 0.97 : 1.0,
          duration: AppDurations.micro,
          curve: AppCurves.out,
          child: Container(
            height: 48,
            width: widget.fullWidth ? double.infinity : null,
            padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
            alignment: Alignment.center,
            decoration: decoration,
            child: DefaultTextStyle(style: AppType.body, child: content),
          ),
        ),
      ),
    );
  }
}

/// Campo de texto padrão: label visível acima, erro abaixo com ícone.
class AppInput extends StatelessWidget {
  final String label;
  final TextEditingController? controller;
  final String? hint;
  final String? error;
  final bool obscure;
  final TextInputType? keyboardType;
  final ValueChanged<String>? onChanged;

  const AppInput({
    required this.label,
    this.controller,
    this.hint,
    this.error,
    this.obscure = false,
    this.keyboardType,
    this.onChanged,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    final hasError = error != null && error!.isNotEmpty;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionLabel(label),
        TextField(
          controller: controller,
          obscureText: obscure,
          keyboardType: keyboardType,
          onChanged: onChanged,
          style: AppType.body,
          decoration: InputDecoration(
            hintText: hint,
            enabledBorder: OutlineInputBorder(
              borderRadius: AppRadii.controlR,
              borderSide: BorderSide(
                  color: hasError ? AppColors.danger : Colors.transparent),
            ),
          ),
        ),
        if (hasError)
          Padding(
            padding: const EdgeInsets.only(top: AppSpacing.x1, left: AppSpacing.x1),
            child: Row(children: [
              const Icon(Icons.error_outline, size: 14, color: AppColors.danger),
              const SizedBox(width: AppSpacing.x1),
              Expanded(
                child: Text(error!,
                    style: AppType.caption.copyWith(color: AppColors.danger)),
              ),
            ]),
          ),
      ],
    );
  }
}

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

/// Entrada com stagger (fade + leve translate) para itens de lista. O atraso
/// cresce com [index] (até um teto) para o efeito cascata. Respeita
/// reduced-motion (mostra direto, sem animação).
class StaggeredItem extends StatefulWidget {
  final int index;
  final Widget child;
  const StaggeredItem({required this.index, required this.child, super.key});

  @override
  State<StaggeredItem> createState() => _StaggeredItemState();
}

class _StaggeredItemState extends State<StaggeredItem>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c =
      AnimationController(vsync: this, duration: AppDurations.enter);

  @override
  void initState() {
    super.initState();
    final delayMs = (widget.index.clamp(0, 12)) * 40;
    Future.delayed(Duration(milliseconds: delayMs), () {
      if (mounted) _c.forward();
    });
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.maybeOf(context)?.disableAnimations ?? false;
    if (reduce) return widget.child;
    final curved = CurvedAnimation(parent: _c, curve: AppCurves.out);
    return FadeTransition(
      opacity: curved,
      child: SlideTransition(
        position: Tween(begin: const Offset(0, 0.06), end: Offset.zero).animate(curved),
        child: widget.child,
      ),
    );
  }
}

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
