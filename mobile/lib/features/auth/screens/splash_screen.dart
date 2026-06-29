import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

class SplashScreen extends StatelessWidget {
  const SplashScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const AppBrandMark(size: 72),
              const SizedBox(height: AppSpacing.x6),
              Text('DopaBlocker', style: AppType.h1),
              const SizedBox(height: AppSpacing.x2),
              Text('Recuperando seu foco…', style: AppType.bodySm),
              const SizedBox(height: AppSpacing.x10),
              const SizedBox(
                width: 22,
                height: 22,
                child: CircularProgressIndicator(
                  strokeWidth: 2.4,
                  color: AppColors.primary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
