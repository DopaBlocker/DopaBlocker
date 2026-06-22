import 'package:flutter/material.dart';

/// Paleta unificada do DopaBlocker (espelho da spec §2 — dark-only).
/// Gradiente azul→roxo é a assinatura de marca; roxo sólido (#7C5CFF) é a
/// primária funcional.
abstract final class AppColors {
  static const background = Color(0xFF08080B);
  static const surface = Color(0xFF121218);
  static const surfaceHigh = Color(0xFF1A1A22);
  static const surfaceInput = Color(0xFF1A1A22);
  static const surfaceHover = Color(0xFF22222C);
  static const border = Color(0xFF26262F);
  static const borderStrong = Color(0xFF33333F);
  static const hairline = Color(0x0DFFFFFF); // rgba(255,255,255,.05)
  static const scrim = Color(0x99000000); // rgba(0,0,0,.6)

  static const primary = Color(0xFF7C5CFF);
  static const primaryHover = Color(0xFF8E78FF);
  static const primaryPressed = Color(0xFF6A4FE6);
  static const primaryDim = Color(0x247C5CFF); // ~14% alpha

  static const brandFrom = Color(0xFF3D6BFF);
  static const brandTo = Color(0xFF8B5CFF);

  static const success = Color(0xFF2FD477);
  static const successDim = Color(0x242FD477);
  static const warning = Color(0xFFE0A23B);
  static const warningDim = Color(0x24E0A23B);
  static const danger = Color(0xFFF2545B);
  static const dangerDim = Color(0x24F2545B);

  static const textPrimary = Color(0xFFECECF1);
  static const textSecondary = Color(0xFF8C8C97);
  static const textFaint = Color(0xFF5C5C66);

  static const divider = Color(0xFF26262F);

  /// Gradiente de marca (logo, CTA primário, anel de destaque).
  static const brandGradient = LinearGradient(
    begin: Alignment.topLeft,
    end: Alignment.bottomRight,
    colors: [brandFrom, brandTo],
  );

  /// Paleta categórica (avatares / gráficos).
  static const categorical = <Color>[
    Color(0xFF7C5CFF),
    Color(0xFFEC4899),
    Color(0xFF2FD477),
    Color(0xFFE0A23B),
    Color(0xFF38BDF8),
    Color(0xFFF2545B),
  ];
}

/// Escala de espaçamento base 4pt (spec §2.7).
abstract final class AppSpacing {
  static const double x1 = 4;
  static const double x2 = 8;
  static const double x3 = 12;
  static const double x4 = 16;
  static const double x5 = 20;
  static const double x6 = 24;
  static const double x8 = 32;
  static const double x10 = 40;
  static const double x12 = 48;
}

/// Raios (spec §2.8).
abstract final class AppRadii {
  static const double sm = 8;
  static const double md = 10;
  static const double lg = 16;
  static const double xl = 20;
  static const double avatar = 12;
  static const double pill = 999;

  static final BorderRadius controlR = BorderRadius.circular(md);
  static final BorderRadius cardR = BorderRadius.circular(lg);
  static final BorderRadius sheetR = BorderRadius.circular(xl);
}

/// Durações e curvas de movimento (spec §2.10).
abstract final class AppDurations {
  static const micro = Duration(milliseconds: 140);
  static const enter = Duration(milliseconds: 220);
  static const exit = Duration(milliseconds: 150);
}

abstract final class AppCurves {
  /// Expo-out — entrada/realce "premium" (ui-ux-pro-max: Modern Dark).
  static const out = Cubic(0.16, 1, 0.3, 1);
  static const in_ = Cubic(0.4, 0, 1, 1);
}

/// Escala tipográfica (spec §2.6). Família base Inter; mono para números.
abstract final class AppType {
  static const _sans = 'Inter';
  static const _mono = 'JetBrains Mono';

  static const display = TextStyle(
      fontFamily: _sans, fontSize: 32, fontWeight: FontWeight.w700, letterSpacing: -0.5, height: 1.2, color: AppColors.textPrimary);
  static const h1 = TextStyle(
      fontFamily: _sans, fontSize: 24, fontWeight: FontWeight.w700, letterSpacing: -0.4, height: 1.2, color: AppColors.textPrimary);
  static const h2 = TextStyle(
      fontFamily: _sans, fontSize: 20, fontWeight: FontWeight.w600, letterSpacing: -0.3, height: 1.25, color: AppColors.textPrimary);
  static const title = TextStyle(
      fontFamily: _sans, fontSize: 16, fontWeight: FontWeight.w600, letterSpacing: -0.1, height: 1.35, color: AppColors.textPrimary);
  static const body = TextStyle(
      fontFamily: _sans, fontSize: 16, fontWeight: FontWeight.w400, height: 1.5, color: AppColors.textPrimary);
  static const bodySm = TextStyle(
      fontFamily: _sans, fontSize: 13, fontWeight: FontWeight.w400, height: 1.5, color: AppColors.textSecondary);
  static const label = TextStyle(
      fontFamily: _sans, fontSize: 11, fontWeight: FontWeight.w600, letterSpacing: 1.2, height: 1.2, color: AppColors.textSecondary);
  static const caption = TextStyle(
      fontFamily: _sans, fontSize: 12, fontWeight: FontWeight.w400, height: 1.4, color: AppColors.textSecondary);

  /// Números/dados tabulares (KPIs, contadores, countdown, código).
  static TextStyle mono({
    double size = 15,
    FontWeight weight = FontWeight.w500,
    Color color = AppColors.textPrimary,
    double? letterSpacing,
  }) =>
      TextStyle(
        fontFamily: _mono,
        fontSize: size,
        fontWeight: weight,
        color: color,
        letterSpacing: letterSpacing,
        fontFeatures: const [FontFeature.tabularFigures()],
      );
}

/// Tema escuro único do app.
abstract final class AppTheme {
  static ThemeData get dark {
    const scheme = ColorScheme.dark(
      primary: AppColors.primary,
      secondary: AppColors.brandTo,
      surface: AppColors.surface,
      error: AppColors.danger,
      onPrimary: Colors.white,
      onSurface: AppColors.textPrimary,
    );

    final base = ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      colorScheme: scheme,
      fontFamily: AppType._sans,
      scaffoldBackgroundColor: AppColors.background,
      splashColor: AppColors.primary.withValues(alpha: 0.08),
      highlightColor: Colors.transparent,
      dividerColor: AppColors.divider,
      appBarTheme: const AppBarTheme(
        backgroundColor: AppColors.background,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        centerTitle: false,
        iconTheme: IconThemeData(color: AppColors.textPrimary),
        titleTextStyle: TextStyle(
          fontFamily: AppType._sans,
          color: AppColors.textPrimary,
          fontSize: 24,
          fontWeight: FontWeight.w700,
          letterSpacing: -0.4,
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: AppColors.surfaceInput,
        hintStyle: const TextStyle(color: AppColors.textFaint),
        labelStyle: const TextStyle(color: AppColors.textSecondary),
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        border: OutlineInputBorder(
          borderRadius: AppRadii.controlR,
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: AppRadii.controlR,
          borderSide: BorderSide.none,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: AppRadii.controlR,
          borderSide: const BorderSide(color: AppColors.primary, width: 1.4),
        ),
      ),
    );

    return base.copyWith(
      textTheme: base.textTheme.copyWith(
        displaySmall: AppType.display,
        headlineMedium: AppType.h1,
        headlineSmall: AppType.h2,
        titleMedium: AppType.title,
        bodyLarge: AppType.body,
        bodyMedium: AppType.body,
        bodySmall: AppType.bodySm,
        labelLarge: AppType.label,
        labelSmall: AppType.caption,
      ),
    );
  }
}
