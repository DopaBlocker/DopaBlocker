import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/chips.dart';

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
