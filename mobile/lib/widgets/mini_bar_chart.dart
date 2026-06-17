import 'package:flutter/material.dart';

import '../theme.dart';

/// Gráfico de barras minimalista (valores normalizados 0..1).
/// Usado nas seções de progresso/dashboard do mockup.
class MiniBarChart extends StatelessWidget {
  final List<double> values;
  final int? highlightIndex;
  final List<String>? labels;
  final double height;

  const MiniBarChart({
    required this.values,
    this.highlightIndex,
    this.labels,
    this.height = 88,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        SizedBox(
          height: height,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              for (var i = 0; i < values.length; i++)
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 3),
                    child: Align(
                      alignment: Alignment.bottomCenter,
                      child: FractionallySizedBox(
                        heightFactor: values[i].clamp(0.04, 1.0),
                        child: Container(
                          decoration: BoxDecoration(
                            color: i == highlightIndex ? AppColors.primary : AppColors.surfaceHigh,
                            borderRadius: BorderRadius.circular(6),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
        if (labels != null) ...[
          const SizedBox(height: 8),
          Row(
            children: [
              for (final l in labels!)
                Expanded(
                  child: Text(
                    l,
                    textAlign: TextAlign.center,
                    style: const TextStyle(color: AppColors.textFaint, fontSize: 10),
                  ),
                ),
            ],
          ),
        ],
      ],
    );
  }
}
