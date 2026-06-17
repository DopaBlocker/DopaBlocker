import 'package:flutter/material.dart';

/// Paleta do DopaBlocker, espelhando o mockup de telas (tema escuro com
/// acento roxo e verde de sucesso).
abstract final class AppColors {
  static const background = Color(0xFF08080B);
  static const surface = Color(0xFF15151C);
  static const surfaceHigh = Color(0xFF1F1F29);
  static const surfaceInput = Color(0xFF24242F);

  static const primary = Color(0xFF7C5CFF);
  static const primaryDim = Color(0xFF2A2440);

  static const success = Color(0xFF2FD477);
  static const successDim = Color(0xFF14331F);

  static const warning = Color(0xFFE0A23B);
  static const warningDim = Color(0xFF33280F);

  static const danger = Color(0xFFF2545B);

  static const textPrimary = Color(0xFFF3F3F6);
  static const textSecondary = Color(0xFF8A8A99);
  static const textFaint = Color(0xFF5E5E6E);

  static const divider = Color(0xFF22222C);
}

/// Tema escuro único do app.
abstract final class AppTheme {
  static ThemeData get dark {
    const scheme = ColorScheme.dark(
      primary: AppColors.primary,
      secondary: AppColors.primary,
      surface: AppColors.surface,
      error: AppColors.danger,
      onPrimary: Colors.white,
      onSurface: AppColors.textPrimary,
    );

    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      colorScheme: scheme,
      scaffoldBackgroundColor: AppColors.background,
      splashColor: AppColors.primary.withOpacity(0.08),
      highlightColor: Colors.transparent,
      appBarTheme: const AppBarTheme(
        backgroundColor: AppColors.background,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        centerTitle: false,
        iconTheme: IconThemeData(color: AppColors.textPrimary),
        titleTextStyle: TextStyle(
          color: AppColors.textPrimary,
          fontSize: 26,
          fontWeight: FontWeight.w700,
          letterSpacing: -0.5,
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: AppColors.surfaceInput,
        hintStyle: const TextStyle(color: AppColors.textFaint),
        labelStyle: const TextStyle(color: AppColors.textSecondary),
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: BorderSide.none,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: const BorderSide(color: AppColors.primary, width: 1.4),
        ),
      ),
    );
  }
}
